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
use monostyle_lexer::LexedLine;

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

	/// How many arguments past the limit a call may have before the count alone triggers a finding.
	///
	/// The grace exists because count alone is a poor signal: four short arguments read fine. Width is what
	/// makes an argument list hard to scan, so a call that stays narrow is left alone however many arguments
	/// it has, up to a few past the limit.
	const COUNT_GRACE: usize = 2;

	let mut findings = Vec::new();

	for line in &file.lines {
		if !line.is_code() {
			continue;
		}

		// An attribute is a declaration rather than a call: `#[derive(Debug, Clone, Copy)]` lists traits,
		// and reporting it asked for a derive list to be split across lines. The same applies to a decorator
		// and to a Java annotation, which is why the check is on the leading marker.
		if is_annotation(line) {
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

		// An argument list is hard to read when it is both long and wide. Either condition alone is a poor
		// signal: `compute(a, b, c, d)` has four arguments and spans twenty columns, while a two-argument call
		// whose arguments are long expressions can span sixty.
		//
		// Requiring both is what keeps the rule from firing on most calls in a real codebase. Testing width
		// alone caught every call over forty columns including three-argument ones, which was the noise that
		// made a reader stop trusting the finding.
		let over_count = parameters > config.max_parameters_inline + COUNT_GRACE;
		let too_wide = width > MAX_INLINE_WIDTH;

		if !(over_count && too_wide) {
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
	let mut index = 0;
	let mut widest: Option<(usize, usize)> = None;

	while let Some(start) = next_open_paren(&characters, index) {
		let (content, end) = argument_content(&characters, start);

		// An empty pair has nothing to measure; a function's own parameter list is the common case.
		if content.iter().any(|character| !character.is_whitespace()) {
			let candidate = measure(&content);

			// Keep the widest group, because that is the one a reader has to parse.
			if widest.is_none_or(|current| candidate.1 > current.1) {
				widest = Some(candidate);
			}
		}

		// Resume after this group, so nested groups are not measured a second time.
		index = end;
	}

	widest
}

/// Returns the index of the next opening parenthesis at or after `from`.
fn next_open_paren(characters: &[char], from: usize) -> Option<usize> {
	characters
		.iter()
		.skip(from)
		.position(|character| *character == '(')
		.map(|offset| from + offset)
}

/// Collects the content of the parenthesis group starting at `open`, and the index after its close.
///
/// Nesting is tracked so a nested call is not split across groups, and so the group's own commas are the
/// ones counted.
fn argument_content(characters: &[char], open: usize) -> (Vec<char>, usize) {
	let mut depth = 0;
	let mut content = Vec::new();
	let mut cursor = open;

	while let Some(character) = characters.get(cursor).copied() {
		match character {
			'(' => depth += 1,
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

	(content, cursor + 1)
}

/// Returns the argument count and rendered width of a parenthesis group's content.
fn measure(content: &[char]) -> (usize, usize) {
	let width = content.iter().collect::<String>().trim().len();
	let commas = content
		.iter()
		.filter(|character| **character == ',')
		.count();

	(commas + 1, width)
}

/// Whether a line is an annotation rather than a call.
///
/// Rust attributes, Python decorators, and Java annotations all group a list of names under a marker. The
/// list is not an argument list, so the width and count rules do not apply to it.
fn is_annotation(line: &LexedLine) -> bool {
	let trimmed = line.masked_code.trim_start();

	trimmed.starts_with("#[") || trimmed.starts_with("#![") || trimmed.starts_with('@')
}

/// Reports files that are too long to navigate.
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
