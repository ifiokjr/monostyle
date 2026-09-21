//! Tests for the fix engine.
//!
//! Applying edits to source is the one place monostyle writes to a user's files, so these tests cover
//! the failure modes that would corrupt code: overlapping edits, offsets past the end of the file, and
//! a partially applied set.

use monostyle::fix::apply_fixes;
use monostyle_core::Fix;
use monostyle_core::Language;
use monostyle_core::Span;
use monostyle_lexer::lex;
use monostyle_rules::RulesConfig;
use monostyle_rules::whitespace;

/// Builds a replacement fix over a byte range.
fn replace(start: usize, end: usize, replacement: &str) -> Fix {
	Fix::replace(Span::new(start, end, 1, 1), replacement, "test edit")
}

#[test]
fn a_single_replacement_is_applied() {
	let (fixed, conflicts) = apply_fixes("hello world", &[replace(0, 5, "goodbye")]);

	assert_eq!(fixed, "goodbye world");
	assert_eq!(conflicts, 0);
}

#[test]
fn multiple_edits_are_applied_without_corrupting_offsets() {
	// The property that matters: edits near the end must not invalidate the offsets of edits near the
	// start, which is why they are applied back to front.
	let (rewritten, conflicts) = apply_fixes(
		"aaa bbb ccc",
		&[replace(0, 3, "XXX"), replace(8, 11, "ZZZ")],
	);

	assert_eq!(rewritten, "XXX bbb ZZZ");
	assert_eq!(conflicts, 0);
}

#[test]
fn many_edits_apply_correctly() {
	let source = "a\nb\nc\nd\ne\n";
	let fixes: Vec<Fix> = (0..5)
		.map(|index| replace(index * 2, index * 2 + 1, "X"))
		.collect();

	let (rewritten, conflicts) = apply_fixes(source, &fixes);

	assert_eq!(rewritten, "X\nX\nX\nX\nX\n");
	assert_eq!(conflicts, 0);
}

#[test]
fn overlapping_edits_keep_the_first_and_report_the_conflict() {
	// Two rules editing the same bytes cannot both be right, and picking one arbitrarily could produce
	// code neither rule intended. The conflict is reported rather than silently dropped.
	let (fixed, conflicts) = apply_fixes("hello world", &[replace(0, 5, "A"), replace(2, 7, "B")]);

	assert_eq!(conflicts, 1, "the overlapping edit should be reported");
	assert_eq!(fixed, "A world", "the first edit in file order should win");
}

#[test]
fn an_insertion_is_a_zero_width_replacement() {
	let source = "fn a() {\n    work();\n}\n";
	// Insert a newline just before `work`, which is what the blank-line fix does.
	let insertion = Fix::insert(Span::new(9, 9, 2, 2), "\n", "blank line");

	let (fixed, conflicts) = apply_fixes(source, &[insertion]);

	assert_eq!(fixed, "fn a() {\n\n    work();\n}\n");
	assert_eq!(conflicts, 0);
}

#[test]
fn a_deletion_removes_the_range() {
	let (fixed, conflicts) = apply_fixes(
		"hello world",
		&[Fix::delete(Span::new(5, 11, 1, 1), "remove")],
	);

	assert_eq!(fixed, "hello");
	assert_eq!(conflicts, 0);
}

#[test]
fn an_empty_fix_changes_nothing() {
	// A fix that replaces nothing with nothing is filtered out before application, so it cannot be
	// counted as an edit that happened.
	let empty = Fix::replace(Span::new(3, 3, 1, 1), "", "no-op");

	assert!(empty.is_empty());

	let (fixed, conflicts) = apply_fixes("hello", &[empty]);

	assert_eq!(fixed, "hello");
	assert_eq!(conflicts, 0);
}

#[test]
fn an_out_of_range_edit_is_reported_rather_than_panicking() {
	// A rule bug must not take down a run, and slicing past the end would panic.
	let (fixed, conflicts) = apply_fixes("short", &[replace(100, 200, "X")]);

	assert_eq!(fixed, "short");
	assert_eq!(conflicts, 1);
}

#[test]
fn a_reversed_range_is_reported_rather_than_panicking() {
	let (fixed, conflicts) = apply_fixes("hello", &[replace(4, 1, "X")]);

	assert_eq!(fixed, "hello");
	assert_eq!(
		conflicts, 1,
		"a reversed range should be treated as a conflict"
	);
}

#[test]
fn an_empty_fix_list_is_a_no_op() {
	let (fixed, conflicts) = apply_fixes("unchanged", &[]);

	assert_eq!(fixed, "unchanged");
	assert_eq!(conflicts, 0);
}

#[test]
fn the_result_is_identical_whatever_order_the_fixes_arrive_in() {
	// Callers assemble fixes from a findings list whose order depends on rule registration, so the
	// engine must be order-independent.
	let forward = vec![
		replace(0, 3, "AAA"),
		replace(4, 7, "BBB"),
		replace(8, 11, "CCC"),
	];
	let mut backward = forward.clone();
	backward.reverse();

	let (first, _) = apply_fixes("xxx yyy zzz", &forward);
	let (second, _) = apply_fixes("xxx yyy zzz", &backward);

	assert_eq!(first, second);
}

// ---------------------------------------------------------------------------
// The blank-line collapse fix, end to end
// ---------------------------------------------------------------------------

/// Runs the whitespace rules over `source` and applies every fix they offer.
///
/// This is the closest thing to `monostyle fix` that a test can express without touching the
/// filesystem, and it is the level the idempotence property has to hold at: the fix and the rule that
/// produced it must agree, or a second pass finds the same problem again.
fn collapse_blank_runs(source: &str, language: Language) -> String {
	let lexed = lex(source, language);
	let config = RulesConfig::default();
	let fixes: Vec<Fix> = whitespace::excessive_blank_lines(&lexed, &config)
		.into_iter()
		.filter_map(|finding| finding.fix)
		.collect();

	let (collapsed, conflicts) = apply_fixes(source, &fixes);

	assert_eq!(conflicts, 0, "the collapse fixes should not overlap");

	collapsed
}

#[test]
fn a_stacked_blank_run_is_collapsed_to_the_allowance() {
	// The damage this fix exists to repair: a run grown past the allowance by a fixer that only ever
	// inserted blank lines.
	let source = "fn split() {\n    let a = 1;\n\n\n\n    let b = 2;\n}\n";

	assert_eq!(
		collapse_blank_runs(source, Language::Rust),
		"fn split() {\n    let a = 1;\n\n    let b = 2;\n}\n"
	);
}

#[test]
fn a_collapsed_run_is_not_reported_again() {
	let source = "fn split() {\n    let a = 1;\n\n\n\n\n    let b = 2;\n}\n";
	let once = collapse_blank_runs(source, Language::Rust);

	let lexed = lex(&once, Language::Rust);
	let remaining = whitespace::excessive_blank_lines(&lexed, &RulesConfig::default());

	assert!(
		remaining.is_empty(),
		"the fixed source should be clean: {remaining:?}"
	);

	// And applying it again is a no-op, which is what makes the fixer safe to run on every commit.
	assert_eq!(collapse_blank_runs(&once, Language::Rust), once);
}

#[test]
fn an_eight_line_gap_is_collapsed_to_one() {
	// The worst stacking observed in a downstream repository was eight consecutive blank lines.
	let source = "fn split() {\n    let a = 1;\n\n\n\n\n\n\n\n\n    let b = 2;\n}\n";

	assert_eq!(
		collapse_blank_runs(source, Language::Rust),
		"fn split() {\n    let a = 1;\n\n    let b = 2;\n}\n"
	);
}

#[test]
fn the_python_allowance_of_two_blank_lines_survives_the_fix() {
	// PEP 8 asks for two blank lines before a top-level definition, so the fix must leave them alone
	// while still collapsing anything longer.
	let source = "def first():\n    pass\n\n\n\n\ndef second():\n    pass\n";

	assert_eq!(
		collapse_blank_runs(source, Language::Python),
		"def first():\n    pass\n\n\ndef second():\n    pass\n"
	);
}

#[test]
fn two_blank_lines_in_rust_are_collapsed_to_one() {
	// In Rust the allowance is one, so the same source collapses further than it does in Python. The
	// per-language floor is what makes one fix serve both.
	let source = "fn split() {\n    let a = 1;\n\n\n    let b = 2;\n}\n";

	assert_eq!(
		collapse_blank_runs(source, Language::Rust),
		"fn split() {\n    let a = 1;\n\n    let b = 2;\n}\n"
	);
}

#[test]
fn a_trailing_run_of_blank_lines_is_left_alone() {
	// The rule ignores a run that separates nothing, so the fix must not offer one. A file ending in
	// blank lines is a formatter question, and rewriting it here would fight that tool.
	let source = "fn done() {}\n\n\n\n";

	assert_eq!(collapse_blank_runs(source, Language::Rust), source);
}

#[test]
fn several_over_long_runs_in_one_file_are_all_collapsed() {
	let source = "fn one() {\n    a();\n\n\n\n    b();\n}\n\n\n\n\nfn two() {\n    c();\n}\n";
	let fixed = collapse_blank_runs(source, Language::Rust);

	assert_eq!(
		fixed, "fn one() {\n    a();\n\n    b();\n}\n\nfn two() {\n    c();\n}\n",
		"every run is collapsed independently"
	);
}

#[test]
fn the_collapse_fix_leaves_a_file_with_no_problems_untouched() {
	let source = "fn clean() {\n    let a = 1;\n\n    let b = 2;\n}\n";

	assert_eq!(collapse_blank_runs(source, Language::Rust), source);
}
