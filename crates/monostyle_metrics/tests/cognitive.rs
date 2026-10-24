//! Tests for cognitive complexity, pinning how the nesting depth is tracked.
//!
//! The depth once moved by a single open/close flag per line, so a block that opened and closed
//! on one line raised the level permanently and every later construct was charged for nesting
//! that did not exist. These tests pin the balance-based accounting that replaced it.

use monostyle_core::Language;
use monostyle_lexer::lex;
use monostyle_metrics::cognitive_complexity;

/// Measures a Rust snippet.
fn measure(source: &str) -> monostyle_metrics::CognitiveComplexity {
	cognitive_complexity(&lex(source, Language::Rust))
}

#[test]
fn a_one_line_block_does_not_leak_into_the_rest_of_the_unit() {
	// Each `let` holds a whole `if`/`else` in braces, so it nets zero depth. The final `if` sits
	// at the function-body depth of 1 — not four levels deep — which is the difference between
	// the reported 17 and the real 11: three one-liners charged 3 each plus the final `if` at 2.
	let source = "\
fn a(v: u32) -> u32 {
    let x = if v > 1 { 1 } else { 2 };
    let y = if v > 1 { 1 } else { 2 };
    let z = if v > 1 { 1 } else { 2 };
    if v > 1 {
        return x;
    }
    y + z
}
";
	let cognitive = measure(source);

	assert_eq!(
		cognitive.total, 11,
		"each one-liner must charge its own depth only"
	);
	assert_eq!(
		cognitive.nesting_penalty, 4,
		"every construct sits one level deep"
	);
	assert_eq!(
		cognitive.max_nesting, 2,
		"only the final `if` opens a level"
	);
}

#[test]
fn a_one_liner_inside_a_nested_block_is_charged_for_its_own_depth_only() {
	// The one-liner sits two levels deep and costs 4 (its `if` at 1 + 2, its `else` exempt).
	// The `else` arm after it must read the depth the one-liner found, not one it leaked.
	let source = "\
fn a(v: u32) -> u32 {
    if v > 1 {
        let x = if v > 2 { 1 } else { 2 };
        x
    } else {
        0
    }
}
";
	let cognitive = measure(source);

	assert_eq!(cognitive.total, 7);
	assert_eq!(cognitive.nesting_penalty, 3);
	assert_eq!(cognitive.max_nesting, 2);
}

#[test]
fn an_if_else_chain_charges_each_arm_at_the_shared_depth() {
	// `} else {` closes and opens on one line, so its balance is zero: the `else` arm sits at
	// the same depth as the `if`, with no transient bump in between.
	let source = "\
fn a(v: u32) -> u32 {
    if v > 1 {
        1
    } else {
        2
    }
}
";
	let cognitive = measure(source);

	assert_eq!(cognitive.total, 3);
	assert_eq!(cognitive.nesting_penalty, 1);
	assert_eq!(cognitive.max_nesting, 2);
}

#[test]
fn a_match_arm_one_liner_does_not_disturb_the_match() {
	// The `match` still opens one level, and an arm whose block opens and closes on its own line
	// leaves that level exactly as it found it.
	let source = "\
fn a(v: u32) -> u32 {
    match v {
        1 => 10,
        2 => { 20 + 2 }
        _ => 30,
    }
}
";
	let cognitive = measure(source);

	assert_eq!(cognitive.total, 2);
	assert_eq!(cognitive.nesting_penalty, 1);
	assert_eq!(cognitive.max_nesting, 2);
}

#[test]
fn an_indented_block_still_opens_by_keyword() {
	// Python has no braces, so its `if` must open a level through the keyword path the balance
	// accounting leaves in place for brace-free lines.
	let source = "\
def a(v):
    if v > 1:
        return 1
    return 0
";
	let cognitive = cognitive_complexity(&lex(source, Language::Python));

	assert_eq!(cognitive.total, 1);
	assert_eq!(cognitive.max_nesting, 1);
}

#[test]
fn an_end_terminated_block_closes_by_keyword() {
	// Ruby closes its blocks with `end`, so the second `if` must be charged at the function-body
	// depth again rather than inside the first one.
	let source = "\
def a(v)
  if v > 1
    1
  end
  if v > 2
    2
  end
end
";
	let cognitive = cognitive_complexity(&lex(source, Language::Ruby));

	assert_eq!(cognitive.total, 2);
	assert_eq!(cognitive.max_nesting, 1);
}

#[test]
fn a_shell_block_opens_at_then_and_closes_at_fi() {
	// Shell marks the block with `then` and `fi` rather than a listed keyword, and the string
	// interpolation must not read as code in between.
	let source = "\
run() {
  if [ -f \"$1\" ]; then
    echo yes
  fi
}
";
	let cognitive = cognitive_complexity(&lex(source, Language::Shell));

	assert_eq!(cognitive.total, 2);
	assert_eq!(cognitive.nesting_penalty, 1);
	assert_eq!(cognitive.max_nesting, 2);
}
