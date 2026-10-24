//! Tests for the cognitive complexity counter.
//!
//! The numbers here are pinned against the SonarSource specification the docs cite: only the
//! constructs the specification lists raise the nesting level, the unit's own body is depth
//! zero, and a line that closes one body and opens another settles before it raises. These
//! tests exist because the counter used to treat every `{` as a nesting step, which made
//! monostyle's numbers incomparable with the spec's.

use monostyle_core::Language;
use monostyle_lexer::lex;
use monostyle_metrics::cognitive_complexity;

/// Computes cognitive complexity for `source` in `language`.
fn measure(source: &str, language: Language) -> monostyle_metrics::CognitiveComplexity {
	cognitive_complexity(&lex(source, language))
}

// ---------------------------------------------------------------------------
// The cases from the issue, measured against the specification
// ---------------------------------------------------------------------------
#[test]
fn sequential_ifs_stay_flat() {
	let source = "fn a(v: u32) -> u32 {\n\tif v > 1 {\n\t\treturn 1;\n\t}\n\tif v > 1 {\n\t\treturn 1;\n\t}\n\tif v > 1 {\n\t\treturn 1;\n\t}\n\tif v > 1 {\n\t\treturn 1;\n\t}\n\t0\n}\n";

	let measured = measure(source, Language::Rust);

	assert_eq!(
		measured.total, 4,
		"one point per if, no nesting between siblings"
	);
	assert_eq!(measured.nesting_penalty, 0);
}

#[test]
fn a_top_level_if_else_nests_once() {
	let source = "fn a(v: u32) -> u32 {\n\tif v > 1 {\n\t\t1\n\t} else {\n\t\t0\n\t}\n}\n";

	let measured = measure(source, Language::Rust);

	assert_eq!(measured.total, 2, "if and else each add one");
	assert_eq!(
		measured.max_nesting, 1,
		"the else body sits beside the if body, not beneath it"
	);
}

#[test]
fn match_arm_blocks_are_not_nesting() {
	let arms = "\t\tSome(n) => {\n\t\t\tif n > 10 {\n\t\t\t\t100\n\t\t\t} else {\n\t\t\t\tn\n\t\t\t}\n\t\t}\n";
	let source = format!(
		"fn process(msg: Option<u32>) -> u32 {{\n\tmatch msg {{\n{arms}{arms}{arms}\t\tNone => 0,\n\t}}\n}}\n"
	);

	let measured = measure(&source, Language::Rust);

	// The specification charges the match 1 and each arm's `if`/`else` 3: 1 + 3 * 3 = 10.
	assert_eq!(measured.total, 10, "the match and its arms' decisions only");
	assert_eq!(
		measured.max_nesting, 2,
		"the match body, then the ifs inside the arms"
	);
}

#[test]
fn else_if_chains_stay_flat() {
	let source = "fn grade(score: u32) -> u32 {\n\tif score > 90 {\n\t\t4\n\t} else if score > 80 {\n\t\t3\n\t} else if score > 70 {\n\t\t2\n\t} else {\n\t\t1\n\t}\n}\n";

	let measured = measure(source, Language::Rust);

	assert_eq!(measured.total, 4, "one point per link in the chain");
	assert_eq!(
		measured.nesting_penalty, 0,
		"chaining alternatives is exempt from nesting"
	);
	assert_eq!(measured.max_nesting, 1);
}

// ---------------------------------------------------------------------------
// What does and does not raise the nesting level
// ---------------------------------------------------------------------------
#[test]
fn the_unit_body_is_depth_zero() {
	let source = "fn a(v: u32) -> u32 {\n\tif v > 1 {\n\t\treturn 1;\n\t}\n\tv\n}\n";

	let measured = measure(source, Language::Rust);

	assert_eq!(
		measured.total, 1,
		"the signature's brace opens the unit, it does not nest it"
	);
	assert_eq!(measured.nesting_penalty, 0);
	assert_eq!(measured.max_nesting, 1);
}

#[test]
fn a_nested_if_pays_the_nesting_penalty() {
	let source = "fn a(outer: bool, inner: bool) -> u32 {\n\tif outer {\n\t\tif inner {\n\t\t\t1\n\t\t} else {\n\t\t\t0\n\t\t}\n\t} else {\n\t\t2\n\t}\n}\n";

	let measured = measure(source, Language::Rust);

	assert_eq!(measured.total, 5, "if 1, inner if 2, and an else for each");
	assert_eq!(measured.nesting_penalty, 1);
	assert_eq!(measured.max_nesting, 2);
}

#[test]
fn an_if_inside_an_else_is_not_nested() {
	let source = "fn a(v: u32) -> u32 {\n\tif v > 1 {\n\t\t1\n\t} else {\n\t\tif v > 2 {\n\t\t\t2\n\t\t} else {\n\t\t\t3\n\t\t}\n\t}\n}\n";

	let measured = measure(source, Language::Rust);

	assert_eq!(measured.total, 4);
	assert_eq!(
		measured.nesting_penalty, 0,
		"the else continues the if's level"
	);
	assert_eq!(measured.max_nesting, 1);
}

#[test]
fn closure_bodies_nest() {
	let source = "fn total(values: &[u32]) -> u32 {\n\tvalues.iter().fold(0, |acc, v| {\n\t\tif v % 2 == 0 {\n\t\t\tacc + v\n\t\t} else {\n\t\t\tacc\n\t\t}\n\t})\n}\n";

	let measured = measure(source, Language::Rust);

	assert_eq!(
		measured.total, 3,
		"fold's closure is a nested function: if 2, else 1"
	);
	assert_eq!(measured.nesting_penalty, 1);
	assert_eq!(measured.max_nesting, 2);
}

#[test]
fn nested_functions_nest() {
	let source = "fn outer(v: u32) -> u32 {\n\tfn inner(x: u32) -> u32 {\n\t\tif x > 1 {\n\t\t\t1\n\t\t} else {\n\t\t\t0\n\t\t}\n\t}\n\tinner(v)\n}\n";

	let measured = measure(source, Language::Rust);

	assert_eq!(
		measured.total, 3,
		"the nested fn costs nothing but nests its body"
	);
	assert_eq!(measured.nesting_penalty, 1);
	assert_eq!(measured.max_nesting, 2);
}

#[test]
fn or_pattern_arms_are_not_closures() {
	let source = "fn classify(n: u32) -> u32 {\n\tmatch n {\n\t\t1 | 2 => {\n\t\t\t0\n\t\t}\n\t\t_ => {\n\t\t\t1\n\t\t}\n\t}\n}\n";

	let measured = measure(source, Language::Rust);

	assert_eq!(
		measured.total, 1,
		"only the match; the arm blocks are plain blocks"
	);
	assert_eq!(measured.max_nesting, 1);
}

#[test]
fn try_nests_catch_rather_than_itself() {
	let source = "int Load() {\n\ttry {\n\t\treturn Read();\n\t} catch (IOException e) {\n\t\treturn 0;\n\t}\n}\n";

	let measured = measure(source, Language::CSharp);

	assert_eq!(
		measured.total, 1,
		"catch adds one; try adds nothing and raises nothing"
	);
	assert_eq!(measured.nesting_penalty, 0);
	assert_eq!(measured.max_nesting, 1);
}

// ---------------------------------------------------------------------------
// Shapes that used to leak or overshoot depth
// ---------------------------------------------------------------------------
#[test]
fn a_single_line_if_else_neither_leaks_nor_overshoots() {
	let source = "fn pick(flag: bool) -> u32 {\n\tif flag { 1 } else { 0 }\n\t0\n}\n";

	let measured = measure(source, Language::Rust);

	assert_eq!(measured.total, 2);
	assert_eq!(measured.nesting_penalty, 0);
	assert_eq!(
		measured.max_nesting, 1,
		"both bodies open and close on their own line"
	);
}

#[test]
fn an_allman_body_claims_its_construct() {
	let source = "int Check(bool a)\n{\n\tif (a)\n\t{\n\t\treturn 1;\n\t}\n\treturn 0;\n}\n";

	let measured = measure(source, Language::CSharp);

	assert_eq!(measured.total, 1, "the if raises once, on its keyword line");
	assert_eq!(measured.nesting_penalty, 0);
	assert_eq!(measured.max_nesting, 1);
}

#[test]
fn end_keyword_languages_still_raise_and_close() {
	let source =
		"def check(ready)\n\tif ready\n\t\twhile running?\n\t\t\twork\n\t\tend\n\tend\nend\n";

	let measured = measure(source, Language::Ruby);

	assert_eq!(measured.total, 3, "if 1, while 1 plus its nesting point");
	assert_eq!(measured.nesting_penalty, 1);
	assert_eq!(measured.max_nesting, 2);
}

#[test]
fn shell_blocks_raise_and_close_without_braces() {
	let source = "run() {\n\tif [ -f \"$1\" ]; then\n\t\tfor f in *; do\n\t\t\tcat \"$f\"\n\t\tdone\n\tfi\n}\n";

	let measured = measure(source, Language::Shell);

	assert_eq!(measured.total, 3, "if 1, for 1 plus its nesting point");
	assert_eq!(measured.nesting_penalty, 1);
	assert_eq!(measured.max_nesting, 2);
}

#[test]
fn logical_operator_sequences_are_unchanged() {
	let source = "fn check(a: bool, b: bool, c: bool) -> bool {\n\ta && b && c\n}\n";

	let measured = measure(source, Language::Rust);

	assert_eq!(measured.total, 1, "one sequence, one point");
}
