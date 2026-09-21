//! Tests for the layout rules.
//!
//! These cover the whitespace rules: the blank line before a control-flow statement, the separation of
//! statement groups, and the ceiling on blank-line runs. Each is asserted both for firing and for
//! staying quiet, because a rule that fires on correct code is worse than one that never fires.
//!
//! The blank line before a trailing return has its own file, `whitespace_returns.rs`.

use monostyle_core::Language;
use monostyle_lexer::lex;
use monostyle_rules::RulesConfig;
use monostyle_rules::whitespace;

/// Joins generated lines into one string.
///
/// Building a fixture by mapping `format!` over a range and appending is the shape clippy flags, and a
/// named helper states the intent more plainly than the fold it expands to.
fn lines_of(items: impl IntoIterator<Item = String>) -> String {
	items.into_iter().fold(String::new(), |mut text, line| {
		text.push_str(&line);

		text
	})
}

/// Runs a rule over Rust source with the default configuration.
fn run(
	rule: fn(&monostyle_lexer::LexedFile, &RulesConfig) -> Vec<monostyle_core::Finding>,
	source: &str,
) -> Vec<monostyle_core::Finding> {
	let lexed = lex(source, Language::Rust);

	rule(&lexed, &RulesConfig::default())
}

#[test]
fn a_guard_clause_return_is_never_reported() {
	// An early return is the shape the style recommends, so reporting it would penalize the structure
	// the guide asks for.
	let findings = run(
		whitespace::blank_line_before_return,
		"fn guard(value: Option<u32>) -> u32 {\n    let Some(value) = value else { return 0 };\n    compute(value)\n}\n",
	);

	assert!(
		findings.is_empty(),
		"a guard clause should not be reported: {findings:?}"
	);
}

#[test]
fn a_control_flow_statement_with_no_blank_line_above_it_is_reported() {
	let findings = run(
		whitespace::blank_line_before_control_flow,
		"fn crowded() {\n    let a = 1;\n    if a > 0 {\n        work();\n    }\n}\n",
	);

	assert_eq!(
		findings.len(),
		1,
		"a crowded control-flow statement should be reported: {findings:?}"
	);
}

#[test]
fn a_separated_control_flow_statement_is_not_reported() {
	let findings = run(
		whitespace::blank_line_before_control_flow,
		"fn separated() {\n    let a = 1;\n\n    if a > 0 {\n        work();\n    }\n}\n",
	);

	assert!(
		findings.is_empty(),
		"a separated control-flow statement should not be reported: {findings:?}"
	);
}

#[test]
fn a_long_statement_run_is_reported() {
	// Group separation fires past the run limit. The most common real trigger is a run of `use` or
	// `mod` declarations, which is what a preamble looks like before anyone groups it.
	let body = lines_of((0..14).map(|index| format!("use crate::module_{index};\n")));
	let findings = run(
		whitespace::group_separation,
		&format!("{body}\nfn entry() {{}}\n"),
	);

	assert_eq!(
		findings.len(),
		1,
		"an unbroken run of declarations should be reported: {findings:?}"
	);
}

#[test]
fn a_short_statement_run_is_not_reported() {
	let findings = run(
		whitespace::group_separation,
		"fn short_run() {\n    let a = 1;\n    let b = 2;\n    work(a, b);\n}\n",
	);

	assert!(
		findings.is_empty(),
		"a short run is one group and should not be reported: {findings:?}"
	);
}

#[test]
fn a_run_of_blank_lines_longer_than_one_is_reported() {
	// Every other rule in this module asks for a gap. Without a ceiling, following all of them at
	// once can grow a gap without limit, and five blank lines between two `match` arms satisfy every
	// rule that asked for one while being harder to read than the crowded code it replaced.
	let findings = run(
		whitespace::excessive_blank_lines,
		"fn split() {\n    let a = 1;\n\n\n\n    let b = 2;\n}\n",
	);

	assert_eq!(
		findings.len(),
		1,
		"three consecutive blank lines should be reported: {findings:?}"
	);
}

#[test]
fn a_single_blank_line_is_allowed() {
	// One blank line is what the other rules meant, so it must never be reported.
	let findings = run(
		whitespace::excessive_blank_lines,
		"fn separated() {\n    let a = 1;\n\n    let b = 2;\n}\n",
	);

	assert!(
		findings.is_empty(),
		"one blank line is the requested separation: {findings:?}"
	);
}

#[test]
fn a_trailing_run_of_blank_lines_is_not_reported() {
	// A run at the end of a file separates nothing, so it is a formatting question rather than a
	// readability one.
	let findings = run(
		whitespace::excessive_blank_lines,
		"fn only() {\n    work();\n}\n\n\n\n",
	);

	assert!(
		findings.is_empty(),
		"a trailing run separates nothing: {findings:?}"
	);
}

#[test]
fn a_keyword_inside_an_attribute_is_not_control_flow() {
	// A serde option list contains the word `default`, and a decorator or annotation can contain any
	// keyword at all. None of them is a statement, so a blank line before one — which is what the
	// finding asks for — would separate the attribute from the declaration it describes.
	let findings = run(
		whitespace::blank_line_before_control_flow,
		"#[derive(Debug)]\n#[serde(default, rename_all = \"kebab-case\")]\npub struct Config {\n    value: u32,\n}\n",
	);

	assert!(
		findings.is_empty(),
		"an attribute is metadata, not a branch: {findings:?}"
	);
}

#[test]
fn a_decorator_containing_a_keyword_is_not_control_flow() {
	// The Python and TypeScript shape of the same case.
	let findings = run(
		whitespace::blank_line_before_control_flow,
		"@component({\n    selector: \"app-root\",\n})\nexport class AppComponent {}\n",
	);

	assert!(
		findings.is_empty(),
		"a decorator is metadata, not a branch: {findings:?}"
	);
}
