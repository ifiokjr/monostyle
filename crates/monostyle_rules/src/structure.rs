//! Structure rules.
//!
//! These rules answer "is this code flat and are its declarations given room?" — the
//! structural half of readability, as opposed to the blank-line half in
//! [`whitespace`](crate::whitespace).

use monostyle_core::Category;
use monostyle_core::Finding;
use monostyle_core::FindingBuilder;
use monostyle_core::Severity;
use monostyle_core::Span;
use monostyle_lexer::LexedFile;

use crate::config::RulesConfig;
use crate::metrics_bridge;

/// Reports functions nested deeper than the configured limit.
///
/// Nesting is reported separately from cognitive complexity because the remedy is
/// structural: flatten with early returns, or extract the inner block. Naming the depth
/// makes the fix obvious in a way a single complexity number does not.
pub fn deep_nesting(file: &LexedFile, config: &RulesConfig) -> Vec<Finding> {
	let mut findings = Vec::new();
	let mut depth: usize = 0;

	for line in &file.lines {
		if !line.is_code() {
			continue;
		}

		let opens = line.masked_code.matches('{').count();
		let closes = line.masked_code.matches('}').count();

		// The depth a line sits at is the depth before it opens anything.
		let line_depth = depth;

		if line_depth > config.max_nesting_depth && !line.nesting.is_empty() {
			findings.push(
				FindingBuilder::new(
					"readability/deep-nesting",
					Category::Readability,
					Span::new(0, line.text.len(), line.number, line.number),
				)
				.severity(Severity::Major)
				.weight(1.5)
				.message(format!(
					"`{}` sits at nesting level {line_depth}, over the limit of {}",
					line.nesting.join("`, `"),
					config.max_nesting_depth
				))
				.suggestion(
					"Flatten this with an early return, a guard clause, or by extracting the \
					 nested block into its own function.",
				)
				.build(),
			);
		}

		depth = depth.saturating_add(opens).saturating_sub(closes);
	}

	findings
}

/// Reports calls and declarations whose parameter list is too long for one line.
///
/// A long parameter list is hard to read inline; splitting it across lines gives each argument
/// its own space and makes the call scannable.
///
/// Width matters as much as count. `Span::new(0, 0, 1, 1)` has four arguments and reads
/// perfectly well, while three arguments each naming a long expression may not. Measuring the
/// rendered width is what keeps the rule from nagging about short numeric calls, which is the
/// most common false positive a count-only version produces.
pub fn long_parameter_list(file: &LexedFile, config: &RulesConfig) -> Vec<Finding> {
	/// Columns of argument text that still fit comfortably on one line.
	const MAX_INLINE_WIDTH: usize = 40;

	let mut findings = Vec::new();

	for line in &file.lines {
		if !line.is_code() || !line.masked_code.contains('(') {
			continue;
		}

		// Count and width come from the masked view, so text inside strings and comments cannot
		// inflate either.
		let (parameters, width) = measure_inline_parameters(&line.masked_code);

		if parameters <= config.max_parameters_inline || width <= MAX_INLINE_WIDTH {
			continue;
		}

		// A list that continues onto later lines has already been given its space.
		if line.parameter_span > 1 {
			continue;
		}

		// A declaration whose body opens here had its argument list laid out deliberately.
		if line.masked_code.trim_end().ends_with('{') {
			continue;
		}

		findings.push(
			FindingBuilder::new(
				"readability/long-parameter-list",
				Category::Readability,
				Span::new(0, line.text.len(), line.number, line.number),
			)
			.severity(Severity::Minor)
			.weight(1.0)
			.message(format!(
				"this call takes {parameters} arguments spanning {width} columns on one line, \
				 over the limit of {}",
				config.max_parameters_inline
			))
			.suggestion(
				"Split the arguments across multiple lines, one per line, so each is \
				 individually readable.",
			)
			.build(),
		);
	}

	findings
}

/// Counts a line's outermost arguments and measures how wide they render.
fn measure_inline_parameters(masked: &str) -> (usize, usize) {
	let Some(open) = masked.find('(') else {
		return (0, 0);
	};

	let arguments = arguments_of(&masked[open..]);
	let saw_content = arguments.iter().any(|character| !character.is_whitespace());

	if !saw_content {
		return (0, 0);
	}

	let width = arguments.iter().collect::<String>().trim().len();
	let commas = arguments
		.iter()
		.filter(|character| **character == ',')
		.count();

	(commas + 1, width)
}

/// Reports files that are too long to navigate.
pub fn oversized_file(file: &LexedFile, config: &RulesConfig) -> Vec<Finding> {
	let lines = file.source_line_count();

	if lines <= config.max_file_lines {
		return Vec::new();
	}

	let span = file.lines.first().map_or(Span::new(0, 0, 1, 1), |line| {
		Span::new(0, line.text.len(), line.number, line.number)
	});

	vec![
		FindingBuilder::new("readability/oversized-file", Category::Readability, span)
			.severity(Severity::Minor)
			.weight(1.0)
			.message(format!(
				"this file has {lines} lines of code, over the limit of {}",
				config.max_file_lines
			))
			.suggestion("Split this file into modules along its natural seams.")
			.build(),
	]
}

/// Reports over-long lines.
///
/// Long lines force horizontal scrolling and usually mean a call or condition is doing too
/// much to read at a glance.
pub fn overlong_lines(file: &LexedFile) -> Vec<Finding> {
	/// Columns beyond which a line is hard to read.
	const MAX_LINE_WIDTH: usize = 120;
	/// Columns beyond which a line is a serious problem rather than a minor one.
	const SEVERE_LINE_WIDTH: usize = 160;

	let mut findings = Vec::new();

	for line in &file.lines {
		if !line.is_code() {
			continue;
		}

		let width = line.code_len();

		if width <= MAX_LINE_WIDTH {
			continue;
		}

		let severity = if width > SEVERE_LINE_WIDTH {
			Severity::Major
		} else {
			Severity::Minor
		};

		findings.push(
			FindingBuilder::new(
				"readability/overlong-line",
				Category::Readability,
				Span::new(0, line.text.len(), line.number, line.number),
			)
			.severity(severity)
			.weight(0.5)
			.message(format!(
				"this line is {width} columns wide, over the {MAX_LINE_WIDTH} limit"
			))
			.suggestion(
				"Break this line at a logical boundary, or extract part of the expression into \
				 a named variable.",
			)
			.build(),
		);
	}

	findings
}

/// Reports functions that are longer than configured.
pub fn oversized_units(file: &LexedFile, config: &RulesConfig) -> Vec<Finding> {
	let units = metrics_bridge::units(file);
	let mut findings = Vec::new();

	for unit in units {
		let length = unit.line_count();

		if length <= config.max_unit_lines {
			continue;
		}

		findings.push(
			FindingBuilder::new(
				"readability/oversized-unit",
				Category::Readability,
				Span::new(0, 0, unit.start_line, unit.start_line),
			)
			.severity(Severity::Major)
			.weight(1.5)
			.message(format!(
				"`{}` spans {length} lines, over the limit of {}",
				unit.name, config.max_unit_lines
			))
			.suggestion(
				"Extract the distinct phases of this function into named helpers so each part \
				 can be read on its own.",
			)
			.build(),
		);
	}

	findings
}

/// Returns the characters between an opening parenthesis and its match.
///
/// Nested parentheses are skipped over rather than counted, which is what makes the returned
/// slice the outermost argument list.
fn arguments_of(text: &str) -> Vec<char> {
	let mut depth = 0;
	let mut characters = Vec::new();

	for character in text.chars() {
		match character {
			// Only content at depth 1 belongs to the outer list; deeper characters fall to the
			// final arm and are discarded. The parentheses themselves are never collected.
			'(' => depth += 1,
			')' => {
				depth -= 1;

				if depth == 0 {
					break;
				}
			}
			_ if depth == 1 => characters.push(character),
			_ => {}
		}
	}

	characters
}
