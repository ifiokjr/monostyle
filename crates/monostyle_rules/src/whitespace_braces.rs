//! Brace ownership regressions for the public whitespace rules.

use monostyle_core::Language;
use monostyle_lexer::lex;

use super::RulesConfig;
use super::blank_line_after_control_flow;
use super::blank_line_before_control_flow;
use super::enclosing_brace_opener;

/// Checks ownership at a line boundary in a complete JavaScript fixture.
fn assert_opener(source: &str, index: usize, expected: Option<usize>) {
	let file = lex(source, Language::JavaScript);

	assert_eq!(
		enclosing_brace_opener(&file.lines, index),
		expected,
		"{source}"
	);
}

#[test]
fn pending_closers_consume_fewer_equal_and_more_openers_on_an_earlier_line() {
	let cases = [
		(
			"const value = { nested: {\n    value: true,\n},\nnext: true,\n};",
			3,
			Some(0),
		),
		(
			"function work() {\nconst value = { nested: {\n    value: true,\n}};\nnext();\n}",
			4,
			Some(0),
		),
		(
			"function work() { const value = {\nnested: {\n    value: true,\n}};\nnext();\n}",
			4,
			Some(0),
		),
	];

	for (source, index, expected) in cases {
		assert_opener(source, index, expected);
	}
}

#[test]
fn the_last_unmatched_opener_wins_when_a_line_closes_and_opens_groups() {
	assert_opener(
		"if (first) {\n    first();\n} else {\n    next();\n}",
		3,
		Some(2),
	);
	assert_opener(
		"function work() {\nif (first) {\n} else { const value = {};\nnext();\n}\n}",
		3,
		Some(2),
	);
}

#[test]
fn comments_and_literals_do_not_change_brace_ownership() {
	assert_opener(
		"function work() {\nconst text = '} {'; // } {\n/* } { */\nnext();\n}",
		3,
		Some(0),
	);
}

#[test]
fn closed_groups_and_top_level_code_have_no_enclosing_opener() {
	assert_opener("const value = { nested: {} };\nnext();", 1, None);
	assert_opener("if (ready) {\n    work();\n}\nnext();", 3, None);
	assert_opener("next();", 0, None);
	assert_opener("first();\nnext();", 1, None);
}

#[test]
fn nested_object_openers_preserve_the_following_control_flow_finding() {
	let file = lex(
		"const value = { nested: {\n    value: true,\n},\nnext: true,\n};\nif (ready) {\n    work();\n}\n",
		Language::JavaScript,
	);
	let config = RulesConfig::default();
	let before = blank_line_before_control_flow(&file, &config);
	let after = blank_line_after_control_flow(&file, &config);

	assert_eq!(before.len(), 1, "{before:?}");
	assert_eq!(before[0].span.start_line, 6);
	assert_eq!(before[0].rule, "readability/blank-line-before-control-flow");
	assert!(after.is_empty(), "{after:?}");
}

#[test]
fn nested_block_openers_preserve_the_enclosing_statement_finding() {
	let file = lex(
		"function work() { if (first) {\n    first();\n}\nnext();\nif (second) {\n    second();\n}\n}\n",
		Language::JavaScript,
	);
	let findings = blank_line_before_control_flow(&file, &RulesConfig::default());

	assert_eq!(findings.len(), 1, "{findings:?}");
	assert_eq!(findings[0].span.start_line, 5);
	assert_eq!(
		findings[0].rule,
		"readability/blank-line-before-control-flow"
	);
}

#[test]
fn a_block_after_a_same_line_literal_closer_still_requires_separation() {
	let file = lex(
		"const value = {\n    value: true,\n}; if (ready) {\n    work();\n    if (nested) {\n        next();\n    }\n}\n",
		Language::JavaScript,
	);
	let findings = blank_line_before_control_flow(&file, &RulesConfig::default());

	assert_eq!(findings.len(), 1, "{findings:?}");
	assert_eq!(findings[0].span.start_line, 5);
	assert_eq!(
		findings[0].rule,
		"readability/blank-line-before-control-flow"
	);
}
