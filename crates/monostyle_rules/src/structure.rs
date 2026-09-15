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
use crate::unit_measures;

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
					Span::new(line.start_byte, line.end_byte, line.number, line.number),
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

/// Reports calls and declarations whose argument list is too long for one line.
///
/// A long argument list is hard to read inline; splitting it across lines gives each argument its own
/// space and makes the call scannable.
///
/// # Which list is measured
///
/// Every parenthesis group on the line is examined and the widest one is reported. Measuring only the
/// first group meant the rule usually measured the wrong thing: on
/// `let value = compute(first, second, third)` the first `(` belongs to the enclosing function's empty
/// parameter list, so the count was zero and the rule never fired on the case it exists for.
///
/// Width is considered alongside the count so that a call like `Span::new(0, 0, 1, 1)` is not reported.
/// The width test is relative to the argument count rather than a fixed threshold, because five short
/// names are only about thirty columns and would otherwise be silently accepted.
#[must_use]
pub fn long_parameter_list(file: &LexedFile, config: &RulesConfig) -> Vec<Finding> {
	/// Columns of argument text that still fit comfortably on one line.
	const MAX_INLINE_WIDTH: usize = 40;

	/// Average columns per argument below which the arguments are trivial.
	///
	/// A single digit or a short constant is about one to three columns; a descriptive name is eight or
	/// more. The threshold sits between them.
	const TRIVIAL_ARGUMENT_WIDTH: usize = 4;

	let mut findings = Vec::new();

	for line in &file.lines {
		if !line.is_code() {
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

		let Some((parameters, width)) = widest_argument_list(&line.masked_code) else {
			continue;
		};

		if parameters <= config.max_parameters_inline {
			continue;
		}

		// The width check spares a call whose arguments are individually trivial, such as
		// `Span::new(0, 0, 1, 1)`. It compares the *average* argument width rather than the total, because
		// a total threshold of forty columns rejected five ordinary argument names — the exact case the
		// rule exists for — while a per-argument average distinguishes names from single digits.
		let average_width = width / parameters.max(1);

		if average_width <= TRIVIAL_ARGUMENT_WIDTH && width <= MAX_INLINE_WIDTH {
			continue;
		}

		findings.push(
			FindingBuilder::new(
				"readability/long-parameter-list",
				Category::Readability,
				Span::new(line.start_byte, line.end_byte, line.number, line.number),
			)
			.severity(Severity::Minor)
			.weight(1.0)
			.message(format!(
				"this call takes {parameters} arguments spanning {width} columns on one line, over the \
				 limit of {}",
				config.max_parameters_inline
			))
			.suggestion(
				"Split the arguments across multiple lines, one per line, so each is individually \
				 readable.",
			)
			.build(),
		);
	}

	findings
}

/// Returns the argument count and width of the widest parenthesis group on a line.
///
/// Returns `None` when the line has no argument list worth measuring, which includes an empty pair such
/// as a function's own parameter list.
fn widest_argument_list(masked: &str) -> Option<(usize, usize)> {
	let characters: Vec<char> = masked.chars().collect();
	let mut widest: Option<(usize, usize)> = None;
	let mut index = 0;

	while index < characters.len() {
		if characters.get(index) != Some(&'(') {
			index += 1;
			continue;
		}

		// Collect this group's contents, tracking nesting so a nested call is not split across groups.
		let mut depth = 0;
		let mut content = Vec::new();
		let mut cursor = index;

		while let Some(character) = characters.get(cursor).copied() {
			match character {
				'(' => {
					depth += 1;
				}
				')' => {
					depth -= 1;

					if depth == 0 {
						break;
					}
				}
				character if depth == 1 => content.push(character),
				_ => {}
			}

			cursor += 1;
		}

		// An empty pair has nothing to measure; a function's own parameter list is the common case.
		if content.iter().any(|character| !character.is_whitespace()) {
			let width = content.iter().collect::<String>().trim().len();
			let commas = content
				.iter()
				.filter(|character| **character == ',')
				.count();
			let candidate = (commas + 1, width);

			// Keep the widest group, because that is the one a reader has to parse.
			if widest.is_none_or(|(current_count, current_width)| {
				candidate.1 > current_width
					|| (candidate.1 == current_width && candidate.0 > current_count)
			}) {
				widest = Some(candidate);
			}
		}

		// Resume after this group, so nested groups are not measured a second time.
		index = cursor + 1;
	}

	widest
}

/// Reports files that are too long to navigate./// Reports files that are too long to navigate.
pub fn oversized_file(file: &LexedFile, config: &RulesConfig) -> Vec<Finding> {
	// Zero disables the rule, matching how `max-line-width` behaves, so a project can turn off a size
	// limit without disabling the rule by name.
	if config.max_file_lines == 0 {
		return Vec::new();
	}

	let lines = file.source_line_count();

	if lines <= config.max_file_lines {
		return Vec::new();
	}

	let span = file.lines.first().map_or(Span::new(0, 0, 1, 1), |line| {
		Span::new(line.start_byte, line.end_byte, line.number, line.number)
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

/// Reports functions that are longer than configured.
pub fn oversized_units(file: &LexedFile, config: &RulesConfig) -> Vec<Finding> {
	let units = unit_measures::UnitSet::new(file);
	let mut findings = Vec::new();

	for (unit, _metrics) in units.iter() {
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
