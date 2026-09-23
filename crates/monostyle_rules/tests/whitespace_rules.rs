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

#[test]
fn a_statement_after_a_control_flow_block_is_reported() {
	// Padding is symmetric: the blank before a branch announces it starting, and the blank after its
	// block announces it finishing.
	let findings = run(
		whitespace::blank_line_after_control_flow,
		"fn work() {\n    if ready {\n        go();\n    }\n    let done = true;\n}\n",
	);

	assert_eq!(
		findings.len(),
		1,
		"the let follows the block with no blank line: {findings:?}"
	);
	assert!(findings[0].fix.is_some(), "the padding is mechanical");
}

#[test]
fn a_statement_after_a_loop_is_reported() {
	let findings = run(
		whitespace::blank_line_after_control_flow,
		"fn work(items: &[u32]) {\n    for item in items {\n        push(item);\n    }\n    seal();\n}\n",
	);

	assert_eq!(findings.len(), 1, "a loop block asks for the same padding");
}

#[test]
fn the_end_of_an_enclosing_body_is_not_a_statement() {
	// The brace that ends the function is punctuation, not content: padding between a branch and the
	// end of its own scope would be noise.
	let findings = run(
		whitespace::blank_line_after_control_flow,
		"fn work() {\n    if ready {\n        go();\n    }\n}\n",
	);

	assert!(
		findings.is_empty(),
		"nothing follows the block: {findings:?}"
	);
}

#[test]
fn an_else_branch_continues_the_same_decision() {
	// The chain is one decision, so it is reported once, at the brace that ends its last branch — and
	// the statement after that is the one that needs the blank.
	let findings = run(
		whitespace::blank_line_after_control_flow,
		"fn work(ready: bool) {\n    if ready {\n        go();\n    } else {\n        wait();\n    }\n    let done = true;\n}\n",
	);

	assert_eq!(findings.len(), 1, "one decision, one finding");
	assert_eq!(
		findings[0].span.start_line, 7,
		"the finding sits on the statement after the chain"
	);
}

#[test]
fn a_match_arm_is_not_a_statement_after_the_block() {
	// The arms of a match are its alternatives. Reporting the second arm as a statement following the
	// first would flag every total function over an enum.
	let findings = run(
		whitespace::blank_line_after_control_flow,
		"fn name(code: u8) -> u32 {\n    match code {\n        0 => {\n            one();\n        }\n        _ => {\n            two();\n        }\n    }\n}\n",
	);

	assert!(
		findings.is_empty(),
		"match arms are alternatives, not a sequence: {findings:?}"
	);
}

#[test]
fn a_struct_literal_body_is_not_control_flow() {
	// A literal's fields are data, and the brace closing one is not a control-flow block.
	let findings = run(
		whitespace::blank_line_after_control_flow,
		"fn point() -> Point {\n    let p = Point {\n        x: 1,\n        y: 2,\n    };\n    p\n}\n",
	);

	assert!(findings.is_empty(), "a literal is data: {findings:?}");
}

#[test]
fn a_one_line_block_needs_no_padding_after() {
	// `if ready { go(); }` is a single statement. Asking for a blank after it would chop every tight
	// sequence into pieces.
	let findings = run(
		whitespace::blank_line_after_control_flow,
		"fn work(ready: bool) {\n    if ready { go(); }\n    let done = true;\n}\n",
	);

	assert!(findings.is_empty(), "a one-line block is one statement");
}

#[test]
fn a_control_flow_successor_is_left_to_the_before_rule() {
	// Two branches in a row need one blank between them, and `blank-line-before-control-flow` already
	// reports it with its own explanation. Reporting it here too would double-count one missing line.
	let findings = run(
		whitespace::blank_line_after_control_flow,
		"fn work(first: bool, second: bool) {\n    if first {\n        go();\n    }\n    if second {\n        go();\n    }\n}\n",
	);

	assert!(
		findings.is_empty(),
		"the blank before a branch belongs to the before rule: {findings:?}"
	);
}

#[test]
fn a_return_after_a_block_is_left_to_the_return_rule() {
	let findings = run(
		whitespace::blank_line_after_control_flow,
		"fn work(ready: bool) -> u32 {\n    if ready {\n        return 1;\n    }\n    return 0;\n}\n",
	);

	assert!(
		findings.is_empty(),
		"the blank before a return belongs to the return rule: {findings:?}"
	);
}

#[test]
fn existing_padding_after_a_block_is_accepted() {
	let findings = run(
		whitespace::blank_line_after_control_flow,
		"fn work() {\n    if ready {\n        go();\n    }\n\n    let done = true;\n}\n",
	);

	assert!(findings.is_empty(), "the blank is already there");
}

// ---------------------------------------------------------------------------
// Comment attachment
// ---------------------------------------------------------------------------
#[test]
fn a_comment_detached_from_its_code_is_reported() {
	// The damage reported from real use: the fixer padded above the statement and left its comment
	// stranded on the other side of the blank. The comment belongs to the line below it.
	let findings = run(
		whitespace::detached_comment,
		"fn work() {\n    loop {\n        // Position at end of line.\n\n        if done {\n            break;\n        }\n    }\n}\n",
	);

	assert_eq!(
		findings.len(),
		1,
		"the comment should not be separated from its if: {findings:?}"
	);
	assert_eq!(
		findings[0].span.start_line, 4,
		"the finding names the blank"
	);
}

#[test]
fn a_comment_directly_above_its_code_is_not_reported() {
	let findings = run(
		whitespace::detached_comment,
		"fn work() {\n    // Position at end of line.\n    if done {\n        break;\n    }\n}\n",
	);

	assert!(findings.is_empty(), "attached comments are correct");
}

#[test]
fn chained_comment_blocks_attach_to_the_code_below() {
	// Two blocks in a row are read as one description, so a blank between them still detaches the whole
	// run from its code, and the finding lands on the first blank.
	let findings = run(
		whitespace::detached_comment,
		"fn work() {\n    // What this does.\n\n    // More detail on that.\n    step();\n}\n",
	);

	assert_eq!(
		findings.len(),
		1,
		"the chain is one description: {findings:?}"
	);
	assert_eq!(findings[0].span.start_line, 3);
}

#[test]
fn a_trailing_comment_block_with_no_code_below_is_not_reported() {
	// A comment that documents nothing below has nothing to detach from.
	let findings = run(
		whitespace::detached_comment,
		"fn work() {\n    step();\n}\n\n// End of file notes.\n",
	);

	assert!(
		findings.is_empty(),
		"nothing below to attach to: {findings:?}"
	);
}

#[test]
fn an_inner_doc_header_is_a_file_header_not_an_attachment() {
	// The `//!` block at the top of a Rust file introduces the file, and the blank after it is how the
	// language's own tools format the separation.
	let findings = run(
		whitespace::detached_comment,
		"//! Crate documentation.\n//! Second line.\n\npub fn work() {}\n",
	);

	assert!(findings.is_empty(), "headers are exempt: {findings:?}");
}

#[test]
fn the_first_comment_block_of_a_file_is_a_header() {
	// License and copyright notices live here. They introduce the file rather than describing the first
	// line, so the blank after them is not a detachment.
	let findings = run(
		whitespace::detached_comment,
		"// Copyright 2026.\n// SPDX-License-Identifier: MIT\n\npub fn work() {}\n",
	);

	assert!(findings.is_empty(), "file headers are exempt: {findings:?}");
}

#[test]
fn a_doc_comment_detached_from_its_item_is_reported() {
	// A blank between a `///` block and the item it documents breaks rustdoc's association too, so the
	// same attachment applies.
	let findings = run(
		whitespace::detached_comment,
		"/// Loads the thing.\n\npub fn load() -> u32 {\n    1\n}\n",
	);

	assert_eq!(
		findings.len(),
		1,
		"a doc comment belongs to its item: {findings:?}"
	);
}

#[test]
fn disabling_the_attachment_rule_silences_it() {
	let lexed = lex(
		"fn work() {\n    // Position at end of line.\n\n    if done {\n        break;\n    }\n}\n",
		Language::Rust,
	);
	let config = RulesConfig {
		require_attached_comments: false,
		..RulesConfig::default()
	};

	assert_eq!(whitespace::detached_comment(&lexed, &config), []);
}
