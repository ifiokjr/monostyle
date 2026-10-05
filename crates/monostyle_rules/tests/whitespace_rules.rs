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
	run_in(rule, source, Language::Rust)
}

fn run_in(
	rule: fn(&monostyle_lexer::LexedFile, &RulesConfig) -> Vec<monostyle_core::Finding>,
	source: &str,
	language: Language,
) -> Vec<monostyle_core::Finding> {
	let lexed = lex(source, language);
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
	// Group separation fires past the run limit. A preamble of `mod` declarations is the
	// trigger; a `use` header is a directive block the formatter owns, which the import
	// test pins separately.
	let body = lines_of((0..14).map(|index| format!("mod module_{index};\n")));
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

#[test]
fn two_single_line_control_flow_statements_still_need_the_blank_between_them() {
	// `if (a > b) return 1;` followed by `if (c < d) return 0;` — breathing room
	// is uniform: every control-flow statement separates from the statement above
	// it, whatever either weighs.
	let findings = run(
		whitespace::blank_line_before_control_flow,
		"if (a > b) return 1;\nif (c < d) return 0;\n\nconst e = 'amazing';\n",
	);

	assert_eq!(
		findings.len(),
		1,
		"a single-line guard still asks for the blank: {findings:?}"
	);
}

#[test]
fn a_multi_line_block_still_asks_for_a_blank_before_the_next_statement() {
	// The exemption is for single-line statements. A block that opens a brace holds
	// a group of statements, so the statement after it still needs the blank.
	let findings = run(
		whitespace::blank_line_before_control_flow,
		"fn work(ready: bool) {\n    let a = compute();\n    if ready {\n        go();\n    }\n    let done = true;\n}\n",
	);

	assert_eq!(
		findings.len(),
		1,
		"a multi-line block still asks for the blank: {findings:?}"
	);
}

#[test]
fn a_single_line_block_still_asks_for_a_blank_before_a_control_flow_successor() {
	// `if first { go(); }` before `if second { go(); }` — the second guard opens
	// control flow, so it separates from the statement above it like any other.
	let findings = run(
		whitespace::blank_line_before_control_flow,
		"fn work(first: bool, second: bool) {\n    if first { go(); }\n    if second { go(); }\n}\n",
	);

	assert_eq!(
		findings.len(),
		1,
		"a single-line block before a guard still asks for the blank: {findings:?}"
	);
}

#[test]
fn a_match_with_single_line_arms_takes_no_padding_before_a_return_arm() {
	// The damage reported from the pinapod review: the fixer inserted a blank
	// between `Ok(payload) => payload,` and `Err(tokens) => return tokens,` because
	// the return rule fired on the arm. The arms are alternatives — cases of one
	// decision — not a sequence needing separation.
	let findings = run(
		whitespace::blank_line_before_return,
		"\tlet payload = match parse_payload(variant) {\n\t\tOk(payload) => payload,\n\t\tErr(tokens) => return tokens,\n\t};\n",
	);

	assert!(
		findings.is_empty(),
		"match arms are alternatives, not a sequence: {findings:?}"
	);
}

#[test]
fn a_return_in_a_dart_switch_case_is_not_reported() {
	let findings = run(
		whitespace::blank_line_before_return,
		"switch (code) {\n  case 0:\n    return 'acid';\n  case 1:\n    return 'coral';\n}\n",
	);

	assert!(
		findings.is_empty(),
		"switch cases are alternatives: {findings:?}"
	);
}

#[test]
fn a_return_outside_a_match_is_still_reported() {
	// The exemption is for arms. A return at the top level or in a function body is
	// a sequential exit and the rule still applies.
	let findings = run(
		whitespace::blank_line_before_return,
		"fn work() -> u32 {\n    prepare();\n    return compute();\n}\n",
	);

	assert_eq!(
		findings.len(),
		1,
		"a return after work still needs the blank: {findings:?}"
	);
}

#[test]
fn an_else_if_continuing_a_braced_chain_needs_no_blank() {
	// The chain is one decision: a line that closes one branch and opens the next —
	// `} else if ... {` — is a continuation, and padding between its own branches would ask
	// for a blank line in the middle of a single statement.
	let findings = run(
		whitespace::blank_line_before_control_flow,
		"fn work(ready: bool, done: bool) {\n    if ready {\n        go();\n    } else if done {\n        stop();\n    } else {\n        wait();\n    }\n}\n",
	);

	assert_eq!(
		findings.len(),
		0,
		"no branch of a chain may be padded from its siblings: {findings:?}"
	);
}

#[test]
fn a_catch_or_finally_continuation_needs_no_blank() {
	let findings = run_in(
		whitespace::blank_line_before_control_flow,
		"function work() {\n    try {\n        useIt();\n    } catch (error) {\n        log(error);\n    } finally {\n        cleanup();\n    }\n}\n",
		Language::TypeScript,
	);

	assert_eq!(
		findings.len(),
		0,
		"catch and finally continue the try: {findings:?}"
	);
}

#[test]
fn a_bare_else_in_an_end_keyword_language_needs_no_blank() {
	let findings = run_in(
		whitespace::blank_line_before_control_flow,
		"if ready\n  go\nelse\n  wait\nend\n",
		Language::Ruby,
	);

	assert_eq!(
		findings.len(),
		0,
		"an else branch is not a new decision: {findings:?}"
	);
}

#[test]
fn a_detached_comment_fix_never_deletes_the_comment_below_the_gap() {
	// The gap between a doc block and the code may hold another comment. The blank is what
	// separates; the comment below the gap belongs to the code and must survive the fix.
	let source =
		"/// A doc block.\n\n    // prepare the check\n    if ready {\n        go();\n    }\n";
	let lexed = lex(source, Language::Rust);
	let findings = whitespace::detached_comment(&lexed, &RulesConfig::default());

	assert_eq!(
		findings.len(),
		1,
		"the blank detaches the doc block: {findings:?}"
	);

	let fix = findings[0].fix.as_ref().expect("the finding carries a fix");
	let removed = &source[fix.span.start_byte..fix.span.end_byte];

	assert!(
		removed.chars().all(char::is_whitespace),
		"the fix may only delete blank lines, but deletes {removed:?}"
	);
}

#[test]
fn a_collection_for_inside_a_literal_is_not_a_statement() {
	// Dart's collection forms — `{ for (...) ..., if (...) ... }` — are expressions, one per
	// element. The before-rule once padded the second `for`, and dart format removes it.
	let source = "void f() {\n  final defaults = {\n    for (final field\n        in members.whereType<FieldDeclaration>())\n      for (final variable in field.fields.variables)\n        if (variable.initializer != null)\n          variable.name.lexeme: variable.initializer,\n  };\n}\n";
	let lexed = lex(source, Language::Dart);
	let before = whitespace::blank_line_before_control_flow(&lexed, &RulesConfig::default());

	assert_eq!(
		before.len(),
		0,
		"collection elements are one expression: {before:?}"
	);
}

#[test]
fn a_switch_expression_assigned_to_a_binding_is_not_a_statement() {
	// `final x = cond ?? switch (i) { ... };` is one declaration whose value happens to branch.
	// A blank above the `switch` or below its `};` splits the value from its binding, and
	// dart format removes both.
	let source = "Widget build(BuildContext context) {\n  final resolvedFill =\n      fillColor ??\n      switch (icon) {\n        Pin.coffee => const Color(0xFFF5CB83),\n        Pin.market => const Color(0xFFAED9BC),\n      };\n  final iconData = lookup(icon);\n\n  return paint(resolvedFill, iconData);\n}\n";
	let lexed = lex(source, Language::Dart);
	let before = whitespace::blank_line_before_control_flow(&lexed, &RulesConfig::default());
	let after = whitespace::blank_line_after_control_flow(&lexed, &RulesConfig::default());

	assert_eq!(
		before.len(),
		0,
		"the switch is an assigned value: {before:?}"
	);
	assert_eq!(
		after.len(),
		0,
		"the declaration has not ended at the closer: {after:?}"
	);
}

#[test]
fn an_unclosed_condition_line_never_ends_a_control_flow_block() {
	// `} else if (name.startsWith('hand') ||` continues onto the next line. The chain has not
	// ended, so the after-rule must not insert a blank inside the condition.
	let source = "void probe(String name) {\n  String category;\n  if (name.startsWith('face')) {\n    category = 'faces';\n  } else if (name.startsWith('hand') ||\n      name.contains('_hand') ||\n      name.contains('thumb')) {\n    category = 'hands';\n  }\n}\n";
	let lexed = lex(source, Language::Dart);
	let after = whitespace::blank_line_after_control_flow(&lexed, &RulesConfig::default());

	assert_eq!(
		after.len(),
		0,
		"the condition is still open, so nothing follows a finished block: {after:?}"
	);
}

#[test]
fn a_multi_line_condition_is_not_a_statement_that_ended() {
	// `if (\n  a ||\n  b\n) return x;` — the last condition line ends with `||`, and the `)`
	// that follows closes the test. Padding there would land inside the condition.
	let source = "function pick(asset: Asset): number {\n  if (\n    asset.kind === \"token\" || asset.kind === \"quoteToken\" ||\n    asset.kind === \"mintBadge\" || asset.kind === \"nft\"\n  ) return asset.mint;\n\n  return 0;\n}\n";
	let lexed = lex(source, Language::TypeScript);
	let after = whitespace::blank_line_after_control_flow(&lexed, &RulesConfig::default());

	assert_eq!(after.len(), 0, "the condition is still open: {after:?}");
}

// ---------------------------------------------------------------------------
// Keyword positions and delimiter-free continuations
//
// The cases below came out of rolling monostyle across real repositories: every one is a shape a
// formatter rejects a blank line in, which means the fixer and the formatter disagree about the
// same file until the rule learns the shape.
// ---------------------------------------------------------------------------

#[test]
fn an_attribute_stays_attached_to_its_declaration() {
	// A blank line between `#[cfg]` and the item it configures is a clippy error
	// (`empty_line_after_outer_attribute`) with `-D warnings`, so this defect broke downstream
	// builds rather than merely disagreeing with a formatter.
	let findings = run(
		whitespace::blank_line_before_control_flow,
		"#[cfg(feature = \"solana-program-error\")]\nimpl From<PinaPodError> for solana_program_error::ProgramError {\n    fn from(e: PinaPodError) -> Self {\n        todo!()\n    }\n}\n",
	);

	assert!(
		findings.is_empty(),
		"an attribute and its item are one unit: {findings:?}"
	);
}

#[test]
fn a_decision_under_an_attribute_reports_separation_above_the_block() {
	// The finding names the decision, and the separation it asks for is the one above the whole
	// attribute block — an attribute belongs to the statement below it, so a blank that already
	// sits above the attribute satisfies the rule. Reporting the separated form would
	// double-count a separation the file already has.
	let findings = run_in(
		whitespace::blank_line_before_control_flow,
		"fn work() {\n    ready();\n    #[cfg(feature = \"slow\")]\n    if slow_path() {\n        wait();\n    }\n}\n",
		Language::Rust,
	);

	assert_eq!(
		findings.len(),
		1,
		"exactly the missing separation is reported: {findings:?}"
	);
	assert_eq!(
		findings[0].span.start_line, 4,
		"the finding points at the decision"
	);

	let separated = run_in(
		whitespace::blank_line_before_control_flow,
		"fn work() {\n    ready();\n\n    #[cfg(feature = \"slow\")]\n    if slow_path() {\n        wait();\n    }\n}\n",
		Language::Rust,
	);

	assert!(
		separated.is_empty(),
		"a blank above the attribute block satisfies the rule: {separated:?}"
	);
}

#[test]
fn a_method_chain_continuation_is_not_a_statement() {
	// `args.bounty\n    .set(if on { 1 } else { 0 });` — the `.set` line carries an `if`, but it
	// continues the receiver above it. A blank here is removed by every formatter.
	let findings = run(
		whitespace::blank_line_before_control_flow,
		"fn build(args: &mut Args, on: bool) {\n    args.bounty\n        .set(if on { 1 } else { 0 });\n    args.flag = on;\n}\n",
	);

	assert!(
		findings.is_empty(),
		"a method-chain line is a continuation, not a decision statement: {findings:?}"
	);
}

#[test]
fn a_builder_call_chain_is_not_padded_between_its_calls() {
	// Chained builder calls joined only by `.` — no enclosing brackets — were padded before the
	// line carrying an `if` argument, which dprint rejected in nine files.
	let findings = run(
		whitespace::blank_line_before_control_flow,
		"fn assert_pool(on: bool) {\n    builder()\n        .manifest_hash(manifest_hash)\n        .service_vault_bump(bump)\n        .remaining_result_receipts(if on { total } else { 0 })\n        .send();\n}\n",
	);

	assert!(
		findings.is_empty(),
		"a builder chain is one statement: {findings:?}"
	);
}

#[test]
fn a_boolean_operator_continuation_is_not_a_statement() {
	// `|| mint_at(bundle, index)? != Address::default()` was padded because `default` counted as
	// a branch. The line is a continuation of the condition above it either way.
	let findings = run(
		whitespace::blank_line_before_control_flow,
		"fn verify(bundle: &Bundle, index: usize) -> bool {\n    index >= usize::from(bundle.count)\n        || bundle.kinds[index] != 0\n        || mint_at(bundle, index)? != Address::default()\n}\n",
	);

	assert!(
		findings.is_empty(),
		"an operator continuation line is not a statement: {findings:?}"
	);
}

#[test]
fn a_conditional_import_continuation_is_not_a_statement() {
	// Dart conditional imports: `import 'stub.dart'\n    if (dart.library.io) 'io.dart'` is one
	// directive. Padding inside it failed `dart format` in six downstream files.
	let findings = run_in(
		whitespace::blank_line_before_control_flow,
		"import 'platform_stub.dart'\n    if (dart.library.io) 'platform_io.dart'\n    if (dart.library.js_interop) 'platform_web.dart';\n\nvoid main() {}\n",
		Language::Dart,
	);

	assert!(
		findings.is_empty(),
		"an import directive spans its `if` clauses: {findings:?}"
	);
}

#[test]
fn an_unterminated_directive_line_marks_the_next_line_a_continuation() {
	// The general form of the import case: a directive line without its `;` is unfinished, so
	// whatever follows continues it rather than starting a statement.
	let findings = run_in(
		whitespace::blank_line_before_control_flow,
		"export {\n    runtimeNative,\n    if (dart.library.js_interop) runtimeWeb\n} from './runtime.dart';\n\nvoid main() {}\n",
		Language::Dart,
	);

	assert!(
		findings.is_empty(),
		"export clauses are one directive: {findings:?}"
	);
}

#[test]
fn a_destructuring_condition_opens_its_body_on_a_later_line() {
	// `if let Some(Segment { .. }) = classify_vec(inner)` closes its *pattern* on
	// one line and opens its *body* on the next; the `})` line starts with a brace
	// but ends a binding, not a statement. A blank between the two was removed by
	// rustfmt in pinapod-derive.
	let findings = run(
		whitespace::blank_line_after_control_flow,
		"fn kind(inner: &Type) -> Option<Kind> {\n    if let Some(Segment {\n        payload: Vec { elem, max, pfx },\n        ..\n    }) = classify_vec(inner)\n    {\n        return Some(Kind::Segment(elem));\n    }\n    None\n}\n",
	);

	// The only separation the rule may ask for is after the *body* closes, before the
	// `None` — never between the pattern's closer and the `{` that opens the body.
	assert_eq!(
		findings.len(),
		1,
		"one finding, after the body: {findings:?}"
	);
	assert_eq!(
		findings[0].span.start_line, 9,
		"the finding targets the tail statement"
	);
}

#[test]
fn a_condition_closer_whose_body_opens_below_is_not_finished() {
	// `if unsafe { … } == 0` ends its condition on one line — `} == 0` — and opens
	// the body on the next. The closer starts with a brace but the statement has
	// not finished, and rustfmt removed the blank monostyle put before the `{`.
	let findings = run(
		whitespace::blank_line_after_control_flow,
		"fn probe(target: &mut Target) -> bool {\n    if unsafe {\n        std::ptr::copy_nonoverlapping(limits.as_ptr(), target, 4);\n        true\n    } == compare(target)\n    {\n        let error = std::io::Error::last_os_error();\n        return false;\n    }\n    true\n}\n",
	);

	// The only separation the rule may ask for is after the *body* closes — before
	// the trailing `true` — never between the condition's closer and its `{`.
	assert_eq!(
		findings.len(),
		1,
		"one finding, after the body: {findings:?}"
	);
	assert_eq!(
		findings[0].span.start_line, 10,
		"the finding targets the tail statement"
	);
}

#[test]
fn a_closer_followed_by_a_continuation_line_has_not_finished() {
	// A switch expression as a ternary arm: the `}` closes the value, and the
	// `:` line continues the binding above. dart format removed the blank
	// monostyle placed before the `:` in skribble's font comparison.
	let findings = run_in(
		whitespace::blank_line_after_control_flow,
		"String build(int? level, WiredFont font) {\n  final family = level == null\n      ? switch (font) {\n          WiredFont.casual => 'RecursiveCasualOriginal',\n          WiredFont.mono => 'RecursiveMonoOriginal',\n        }\n      : font.familyFor(level!);\n  return family;\n}\n",
		Language::Dart,
	);

	assert!(
		findings.is_empty(),
		"a closer followed by a continuation has not finished: {findings:?}"
	);
}

// ---------------------------------------------------------------------------
// Breathing room around control flow — the fix returns
// ---------------------------------------------------------------------------

#[test]
fn a_multi_line_control_flow_statement_gets_the_blank_above_it_fixed() {
	// The reviewer's rule: there should always be breathing room around a
	// control-flow statement. The fix was demoted to a finding after it dominated
	// real diffs with blanks formatters removed, but the formatter conflicts were
	// the positional defects fixed since — the fix returns with them.
	let source = "export function hasBinary(dir: string): boolean {\n  const binDir = join(dir, \"bin\");\n  if (!existsSync(binDir)) {\n    return false;\n  }\n  const entries = readdirSync(binDir);\n  return entries.some((entry) => entry.startsWith(\"mdt\"));\n}\n";
	let lexed = lex(source, Language::TypeScript);
	let config = RulesConfig::default();
	let findings = whitespace::blank_line_before_control_flow(&lexed, &config);

	assert!(
		findings
			.iter()
			.any(|finding| finding.fix.is_some() && finding.span.start_line == 3),
		"the crowded `if` carries a fix: {findings:?}"
	);
}

#[test]
fn a_single_line_control_flow_statement_still_needs_the_blank_above_it() {
	// Breathing room is uniform: even a single-line decision separates from the
	// statement above it, so each guard is visible as control flow at a glance.
	let source = "fn pick(count: u32) -> u32 {\n    let base = load();\n    if count > 0 { return base + 1; }\n    let other = 2;\n    if count > 2 { return base + 2; }\n    base\n}\n";
	let lexed = lex(source, Language::Rust);
	let findings = whitespace::blank_line_before_control_flow(&lexed, &RulesConfig::default());

	assert_eq!(
		findings.len(),
		2,
		"each single-line guard asks for the blank above it: {findings:?}"
	);
}

#[test]
fn the_restored_fix_anchors_above_an_attribute_block() {
	// The fix inserts at the attribute block's start rather than between the
	// attribute and the statement — a break there is a clippy error under
	// `-D warnings`.
	let source = "fn work() {\n    ready();\n    #[cfg(feature = \"slow\")]\n    if slow_path() {\n        wait();\n    }\n}\n";
	let lexed = lex(source, Language::Rust);
	let findings = whitespace::blank_line_before_control_flow(&lexed, &RulesConfig::default());

	assert_eq!(
		findings.len(),
		1,
		"exactly the crowded decision: {findings:?}"
	);
	let fix = findings[0].fix.as_ref().expect("the fix is restored");
	assert_eq!(fix.span.start_line, 3, "the anchor is the attribute line");
}

#[test]
fn a_match_arm_guard_is_part_of_its_arm() {
	// A pattern with a multi-line guard: `[root, module, name]` then
	// `if root.ident == "core" && … =>`. The guard's `if` belongs to the arm
	// above it — a blank between the pattern and its guard is removed by rustfmt.
	let findings = run(
		whitespace::blank_line_before_control_flow,
		"fn kind(path: &Path) -> Option<Kind> {\n    match path.segments {\n        [root, module, name] if root == \"pinapod\" && module == \"pod\" => name,\n        [root, module, name]\n            if (root == \"core\" || root == \"std\") && module == \"option\" =>\n        {\n            return Some(Kind::Option(name));\n        }\n        _ => None,\n    }\n}\n",
	);

	assert!(
		findings.is_empty(),
		"an arm guard continues its arm, not a new statement: {findings:?}"
	);
}

#[test]
fn a_do_while_tail_is_the_same_statement() {
	// `} while (…);` closes the loop the lines above opened — the `while` is the
	// tail of one statement, and dart format removes any blank placed before it.
	let findings = run_in(
		whitespace::blank_line_before_control_flow,
		"String nextId() {\n  do {\n    id = 'drawing-${next.value++}';\n  } while (editor.annotations.any((a) => a.id == id));\n  return id;\n}\n",
		Language::Dart,
	);

	assert!(
		findings.is_empty(),
		"a do-while tail continues its statement: {findings:?}"
	);
}

// ---------------------------------------------------------------------------
// Excessive indentation measures nesting, not the formatter's alignment
// ---------------------------------------------------------------------------

#[test]
fn a_deeply_indented_call_argument_is_not_excessive_nesting() {
	// A formatter splits a long call by indenting each argument one level past
	// the call, so a call nested three levels deep produces argument lines at
	// seven levels. Those lines continue the call above them; they do not open
	// anything. Counting them made every real repository report hundreds of
	// findings that no author could act on without flattening a call the
	// formatter would re-split.
	let source = "fn probe() {\n\tif ready {\n\t\tif armed {\n\t\t\tif live {\n\t\t\t\tlet outcome = handle(\n\t\t\t\t\trequest,\n\t\t\t\t\tResponse {\n\t\t\t\t\t\tstatus,\n\t\t\t\t\t\tbody: Payload {\n\t\t\t\t\t\t\tbytes,\n\t\t\t\t\t\t\tmeta,\n\t\t\t\t\t\t},\n\t\t\t\t\t},\n\t\t\t\t);\n\t\t\t}\n\t\t}\n\t}\n}\n";
	let findings = run(whitespace::excessive_indentation, source);

	assert!(
		findings.is_empty(),
		"argument lines continue the call above them: {findings:?}"
	);
}

#[test]
fn a_deeply_indented_statement_is_still_reported() {
	// The rule keeps its purpose: a statement that opens inside that much
	// nesting really is hard to read, and flattening it is the fix the
	// suggestion names.
	let source = "fn probe() {\n\tif a {\n\t\tif b {\n\t\t\tif c {\n\t\t\t\tif d {\n\t\t\t\t\tif e {\n\t\t\t\t\t\tif f {\n\t\t\t\t\t\t\tif g {\n\t\t\t\t\t\t\t\twork();\n\t\t\t\t\t\t\t}\n\t\t\t\t\t\t}\n\t\t\t\t\t}\n\t\t\t}\n\t\t}\n\t}\n}\n";
	let findings = run(whitespace::excessive_indentation, source);

	assert_eq!(
		findings.len(),
		2,
		"both over-limit statements are reported: {findings:?}"
	);
	assert_eq!(findings[0].span.start_line, 8, "the `if g` is reported");
	assert_eq!(
		findings[1].span.start_line, 9,
		"the call inside it is reported"
	);
}

#[test]
fn a_match_arm_label_is_not_excessive_nesting() {
	// `case`/`default`/`when` labels sit one level deeper than the switch they
	// belong to, and their bodies one deeper still. A switch inside a method is
	// a normal shape, not a nesting problem, and formatters keep the arms where
	// the author put them.
	let source = "fn describe(value: u32) -> String {\n\tif a {\n\t\tif b {\n\t\t\tmatch value {\n\t\t\t\t0 => \"zero\".to_string(),\n\t\t\t\t1 => {\n\t\t\t\t\tlet label = compute(\n\t\t\t\t\t\tvalue,\n\t\t\t\t\t\tContext {\n\t\t\t\t\t\t\tkind,\n\t\t\t\t\t\t\tname,\n\t\t\t\t\t\t},\n\t\t\t\t\t);\n\t\t\t\t\tlabel\n\t\t\t\t}\n\t\t\t\t_ => \"other\".to_string(),\n\t\t\t}\n\t\t}\n\t}\n}\n";
	let findings = run(whitespace::excessive_indentation, source);

	assert!(
		findings.is_empty(),
		"arm bodies and their call arguments are not nesting: {findings:?}"
	);
}

#[test]
fn a_control_flow_statement_inside_a_call_argument_is_excessive_nesting() {
	// The teeth the blanket exemption removed: a closure body inside a call is
	// nesting the author chose, whatever encloses it. Its decisions and returns
	// are the flattening the rule suggests — extract the closure.
	let source = "function probe() {\n\tif (a) {\n\t\tif (b) {\n\t\t\tif (c) {\n\t\t\t\tif (live) {\n\t\t\t\t\t\tif (armed) {\n\t\t\t\t\t\t\tconst out = run(async () => {\n\t\t\t\t\t\t\t\tif (ready) {\n\t\t\t\t\t\t\t\t\treturn work();\n\t\t\t\t\t\t\t\t}\n\t\t\t\t\t\t\t});\n\t\t\t\t\t\t}\n\t\t\t\t\t}\n\t\t\t\t}\n\t\t\t}\n\t\t}\n\t}\n}\n";
	let findings = run(whitespace::excessive_indentation, source);

	assert_eq!(
		findings.len(),
		2,
		"the guard and the return inside the call are the author's nesting: {findings:?}"
	);
	assert_eq!(
		findings[0].span.start_line, 8,
		"the `if (ready)` is reported"
	);
	assert_eq!(
		findings[1].span.start_line, 9,
		"the return inside it is reported"
	);
}

#[test]
fn a_statement_inside_a_multi_line_arm_body_is_measured() {
	// The label exemption covers the label, not the region: an `if` nested inside
	// a multi-line arm body is nesting like any other, and a switch does not make
	// it free.
	let source = "fn probe(v: u32) {\n\tif a {\n\t\tif b {\n\t\t\tif c {\n\t\t\t\tif d {\n\t\t\t\t\tmatch v {\n\t\t\t\t\t\t0 => {\n\t\t\t\t\t\t\tif e {\n\t\t\t\t\t\t\t\twork();\n\t\t\t\t\t\t\t}\n\t\t\t\t\t\t}\n\t\t\t\t\t\t_ => {}\n\t\t\t\t\t}\n\t\t\t\t}\n\t\t\t}\n\t\t}\n\t}\n}\n";
	let findings = run(whitespace::excessive_indentation, source);

	assert_eq!(
		findings.len(),
		2,
		"an `if` inside an arm body is real nesting: {findings:?}"
	);
	assert_eq!(findings[0].span.start_line, 8, "the `if e` is reported");
	assert_eq!(
		findings[1].span.start_line, 9,
		"the statement under it is reported"
	);
}

#[test]
fn a_closing_bracket_line_is_not_excessive_nesting() {
	// A closer is indented to match what it closes, so a deeply nested literal
	// ends with a deep `)` or `]`. It opens nothing.
	let source = "fn probe() {\n\tif a {\n\t\tif b {\n\t\t\tif c {\n\t\t\t\tlet value = build(\n\t\t\t\t\tConfig {\n\t\t\t\t\t\tname,\n\t\t\t\t\t\tpayload: Payload {\n\t\t\t\t\t\t\tbytes: vec![\n\t\t\t\t\t\t\t\t1,\n\t\t\t\t\t\t\t],\n\t\t\t\t\t\t},\n\t\t\t\t\t},\n\t\t\t\t);\n\t\t\t}\n\t\t}\n\t}\n}\n";
	let findings = run(whitespace::excessive_indentation, source);

	assert!(
		findings.is_empty(),
		"a closer matches what it closes rather than opening a block: {findings:?}"
	);
}

#[test]
fn the_indentation_limit_is_configurable() {
	let source = "fn probe() {\n\tif a {\n\t\tif b {\n\t\t\tif c {\n\t\t\t\tif d {\n\t\t\t\t\tif e {\n\t\t\t\t\t\tif f {\n\t\t\t\t\t\t\tif g {\n\t\t\t\t\t\t\t\twork();\n\t\t\t\t\t\t\t}\n\t\t\t\t\t\t}\n\t\t\t\t\t}\n\t\t\t\t}\n\t\t\t}\n\t\t}\n\t}\n}\n";
	let lexed = lex(source, Language::Rust);
	let default_findings = whitespace::excessive_indentation(&lexed, &RulesConfig::default());
	let raised = RulesConfig {
		max_indent_width: 40,
		..RulesConfig::default()
	};

	assert!(
		!default_findings.is_empty(),
		"the fixture nests past the default limit"
	);
	assert!(
		whitespace::excessive_indentation(&lexed, &raised).is_empty(),
		"the configured limit raises the bar"
	);
}

#[test]
fn the_tab_width_is_configurable_for_tab_indented_projects() {
	// A project that formats with `useTabs: true, indentWidth: 2` — dprint's
	// common TypeScript setting — draws a seven-tab line fourteen columns wide,
	// but the lexer charges four columns per tab. Both the indentation and the
	// line-length rules measure the formatter's output rather than the author's
	// choice, so the project needs a way to say what its tabs mean.
	let source = "function probe() {\n\t\t\t\t\t\t\tconst value = compute(\n\t\t\t\t\t\t\t\tinput,\n\t\t\t\t\t\t\t);\n}\n";
	let lexed = lex(source, Language::TypeScript);
	let two_wide = RulesConfig {
		tab_width: 2,
		..RulesConfig::default()
	};

	assert!(
		whitespace::excessive_indentation(&lexed, &two_wide).is_empty(),
		"seven tabs are fourteen columns when a tab is two"
	);
	assert_eq!(
		whitespace::excessive_indentation(&lexed, &RulesConfig::default()).len(),
		1,
		"the default of four columns still reports it"
	);
}
