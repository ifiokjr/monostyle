//! Tests for the cognitive complexity counter.
//!
//! The numbers here are pinned against the `SonarSource` specification the docs cite: only the
//! constructs the specification lists raise the nesting level, the unit's own body is depth
//! zero, and a line that closes one body and opens another settles before it raises. A block
//! that opens and closes on one line must not leave that level behind for the rest of the unit.

use monostyle_core::Language;
use monostyle_lexer::lex;
use monostyle_metrics::cognitive_complexity;

/// Computes cognitive complexity for `source` in `language`.
fn measure(source: &str, language: Language) -> monostyle_metrics::CognitiveComplexity {
	cognitive_complexity(&lex(source, language))
}

#[test]
fn sequential_ifs_stay_flat() {
	let source = "\
fn a(v: u32) -> u32 {
	if v > 1 {
		return 1;
	}
	if v > 1 {
		return 1;
	}
	if v > 1 {
		return 1;
	}
	if v > 1 {
		return 1;
	}
	0
}
";

	let measured = measure(source, Language::Rust);

	assert_eq!(
		measured.total, 4,
		"one point per if, no nesting between siblings"
	);
	assert_eq!(measured.nesting_penalty, 0);
	assert_eq!(measured.max_nesting, 1);
}

#[test]
fn a_top_level_if_else_nests_once() {
	let source = "\
fn a(v: u32) -> u32 {
	if v > 1 {
		1
	} else {
		0
	}
}
";

	let measured = measure(source, Language::Rust);

	assert_eq!(measured.total, 2, "if and else each add one");
	assert_eq!(measured.nesting_penalty, 0);
	assert_eq!(
		measured.max_nesting, 1,
		"the else body sits beside the if body, not beneath it"
	);
}

#[test]
fn match_arm_blocks_are_not_nesting() {
	let arms = "\
		Some(n) => {
			if n > 10 {
				100
			} else {
				n
			}
		}
";
	let source = format!(
		"\
fn process(msg: Option<u32>) -> u32 {{
	match msg {{
{arms}{arms}{arms}		None => 0,
	}}
}}
"
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
	let source = "\
fn grade(score: u32) -> u32 {
	if score > 90 {
		4
	} else if score > 80 {
		3
	} else if score > 70 {
		2
	} else {
		1
	}
}
";

	let measured = measure(source, Language::Rust);

	assert_eq!(measured.total, 4, "one point per link in the chain");
	assert_eq!(
		measured.nesting_penalty, 0,
		"chaining alternatives is exempt from nesting"
	);
	assert_eq!(measured.max_nesting, 1);
}

#[test]
fn the_unit_body_is_depth_zero() {
	let source = "\
fn a(v: u32) -> u32 {
	if v > 1 {
		return 1;
	}
	v
}
";

	let measured = measure(source, Language::Rust);

	assert_eq!(
		measured.total, 1,
		"the signature's brace opens the unit, it does not nest it"
	);
	assert_eq!(measured.nesting_penalty, 0);
	assert_eq!(measured.max_nesting, 1);
}

#[test]
fn sibling_functions_each_start_at_depth_zero() {
	let source = "\
fn a(v: u32) -> u32 {
	if v > 1 { 1 } else { 0 }
}
fn b(v: u32) -> u32 {
	if v > 1 { 1 } else { 0 }
}
";

	let measured = measure(source, Language::Rust);

	assert_eq!(
		measured.total, 4,
		"each function's if and else, nothing shared"
	);
	assert_eq!(
		measured.nesting_penalty, 0,
		"the second function is not nested inside the first"
	);
	assert_eq!(measured.max_nesting, 1);
}

#[test]
fn a_nested_if_pays_the_nesting_penalty() {
	let source = "\
fn a(outer: bool, inner: bool) -> u32 {
	if outer {
		if inner {
			1
		} else {
			0
		}
	} else {
		2
	}
}
";

	let measured = measure(source, Language::Rust);

	assert_eq!(measured.total, 5, "if 1, inner if 2, and an else for each");
	assert_eq!(measured.nesting_penalty, 1);
	assert_eq!(measured.max_nesting, 2);
}

#[test]
fn an_if_inside_an_else_is_not_nested() {
	let source = "\
fn a(v: u32) -> u32 {
	if v > 1 {
		1
	} else {
		if v > 2 {
			2
		} else {
			3
		}
	}
}
";

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
	let source = "\
fn total(values: &[u32]) -> u32 {
	values.iter().fold(0, |acc, v| {
		if v % 2 == 0 {
			acc + v
		} else {
			acc
		}
	})
}
";

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
	let source = "\
fn outer(v: u32) -> u32 {
	fn inner(x: u32) -> u32 {
		if x > 1 {
			1
		} else {
			0
		}
	}
	inner(v)
}
";

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
	let source = "\
fn classify(n: u32) -> u32 {
	match n {
		1 | 2 => {
			0
		}
		_ => {
			1
		}
	}
}
";

	let measured = measure(source, Language::Rust);

	assert_eq!(
		measured.total, 1,
		"only the match; the arm blocks are plain blocks"
	);
	assert_eq!(measured.max_nesting, 1);
}

#[test]
fn try_nests_catch_rather_than_itself() {
	let source = "\
int Load() {
	try {
		return Read();
	} catch (IOException e) {
		return 0;
	}
}
";

	let measured = measure(source, Language::CSharp);

	assert_eq!(
		measured.total, 1,
		"catch adds one; try adds nothing and raises nothing"
	);
	assert_eq!(measured.nesting_penalty, 0);
	assert_eq!(measured.max_nesting, 1);
}

#[test]
fn a_single_line_if_else_neither_leaks_nor_overshoots() {
	let source = "\
fn pick(flag: bool) -> u32 {
	if flag { 1 } else { 0 }
	0
}
";

	let measured = measure(source, Language::Rust);

	assert_eq!(measured.total, 2);
	assert_eq!(measured.nesting_penalty, 0);
	assert_eq!(
		measured.max_nesting, 1,
		"both bodies open and close on their own line"
	);
}

#[test]
fn a_one_line_block_does_not_leak_into_the_rest_of_the_unit() {
	// Each `let` holds a whole `if`/`else` that nets zero depth. The final `if` is still at the
	// unit's depth of 0 — the function body is not a nesting level — so nothing after a one-liner
	// inherits a level the one-liner opened.
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
	let measured = measure(source, Language::Rust);

	assert_eq!(measured.total, 7, "three one-liners plus the final if");
	assert_eq!(measured.nesting_penalty, 0);
	assert_eq!(measured.max_nesting, 1);
}

#[test]
fn a_one_liner_inside_a_nested_block_is_charged_for_its_own_depth_only() {
	// The one-liner sits one level deep and costs 3 (its `if` at 1 + 1, its `else` exempt).
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
	let measured = measure(source, Language::Rust);

	assert_eq!(measured.total, 5);
	assert_eq!(measured.nesting_penalty, 1);
	assert_eq!(measured.max_nesting, 2);
}

#[test]
fn an_if_else_chain_charges_each_arm_at_the_shared_depth() {
	// `} else {` closes and opens on one line. The `else` arm sits at the same depth as the `if`,
	// with no transient bump in between, and the function body itself is not a level.
	let source = "\
fn a(v: u32) -> u32 {
	if v > 1 {
		1
	} else {
		2
	}
}
";
	let measured = measure(source, Language::Rust);

	assert_eq!(measured.total, 2);
	assert_eq!(measured.nesting_penalty, 0);
	assert_eq!(measured.max_nesting, 1);
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
	let measured = measure(source, Language::Rust);

	assert_eq!(measured.total, 1);
	assert_eq!(measured.nesting_penalty, 0);
	assert_eq!(measured.max_nesting, 1);
}

#[test]
fn an_allman_body_claims_its_construct() {
	let source = "\
int Check(bool a)
{
	if (a)
	{
		return 1;
	}
	return 0;
}
";

	let measured = measure(source, Language::CSharp);

	assert_eq!(measured.total, 1, "the if raises once, on its keyword line");
	assert_eq!(measured.nesting_penalty, 0);
	assert_eq!(measured.max_nesting, 1);
}

#[test]
fn an_allman_else_if_keeps_its_body_at_the_if_level() {
	// The chain's brace is on the next line, after the previous body has closed. The `if` inside
	// that body is one level deep, the same as an `if` inside the first branch.
	let source = "\
int Grade(int score)
{
	if (score > 90)
	{
		return 4;
	}
	else if (score > 80)
	{
		if (score > 85)
		{
			return 3;
		}
		return 2;
	}
	return 0;
}
";

	let measured = measure(source, Language::CSharp);

	assert_eq!(
		measured.total, 4,
		"if, else if, and the nested if at one level"
	);
	assert_eq!(measured.nesting_penalty, 1);
	assert_eq!(measured.max_nesting, 2);
}

#[test]
fn an_indented_block_still_opens_by_keyword() {
	// Python has no braces, so its `if` must open a level through the keyword path.
	let source = "\
def a(v):
    if v > 1:
        return 1
    return 0
";
	let measured = measure(source, Language::Python);

	assert_eq!(measured.total, 1);
	assert_eq!(measured.max_nesting, 1);
}

#[test]
fn python_elif_stays_at_the_if_level() {
	let source = "\
def a(v):
    if v > 1:
        return 1
    elif v > 2:
        if v > 3:
            return 3
        return 2
    return 0
";
	let measured = measure(source, Language::Python);

	assert_eq!(
		measured.total, 4,
		"if, elif, and the nested if at one level"
	);
	assert_eq!(measured.nesting_penalty, 1);
	assert_eq!(measured.max_nesting, 2);
}

#[test]
fn an_end_terminated_block_closes_by_keyword() {
	// Ruby closes its blocks with `end`, so the second `if` must be charged at the top again
	// rather than inside the first one.
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
	let measured = measure(source, Language::Ruby);

	assert_eq!(measured.total, 2);
	assert_eq!(measured.nesting_penalty, 0);
	assert_eq!(measured.max_nesting, 1);
}

#[test]
fn ruby_elsif_chains_stay_flat() {
	let source = "\
def grade(score)
  if score > 90
    4
  elsif score > 80
    3
  elsif score > 70
    2
  else
    1
  end
end
";
	let measured = measure(source, Language::Ruby);

	assert_eq!(measured.total, 4, "one point per link, including elsif");
	assert_eq!(measured.nesting_penalty, 0);
	assert_eq!(measured.max_nesting, 1);
}

#[test]
fn end_keyword_languages_still_raise_and_close() {
	let source = "\
def check(ready)
	if ready
		while running?
			work
		end
	end
end
";

	let measured = measure(source, Language::Ruby);

	assert_eq!(measured.total, 3, "if 1, while 1 plus its nesting point");
	assert_eq!(measured.nesting_penalty, 1);
	assert_eq!(measured.max_nesting, 2);
}

#[test]
fn shell_blocks_raise_and_close_without_braces() {
	let source = "\
run() {
	if [ -f \"$1\" ]; then
		for f in *; do
			cat \"$f\"
		done
	fi
}
";

	let measured = measure(source, Language::Shell);

	assert_eq!(measured.total, 3, "if 1, for 1 plus its nesting point");
	assert_eq!(measured.nesting_penalty, 1);
	assert_eq!(measured.max_nesting, 2);
}

#[test]
fn a_shell_block_opens_at_then_and_closes_at_fi() {
	// Shell marks the block with `then` and `fi` rather than a listed keyword, and the string
	// interpolation must not read as code in between. The function body is not a level.
	let source = "\
run() {
  if [ -f \"$1\" ]; then
    echo yes
  fi
}
";
	let measured = measure(source, Language::Shell);

	assert_eq!(measured.total, 1);
	assert_eq!(measured.nesting_penalty, 0);
	assert_eq!(measured.max_nesting, 1);
}

#[test]
fn shell_elif_does_not_stack_or_leak() {
	let source = "\
run() {
  if [ -f \"$1\" ]; then
    echo yes
  elif [ -d \"$1\" ]; then
    echo dir
  fi
  if [ -n \"$1\" ]; then
    echo set
  fi
}
";
	let measured = measure(source, Language::Shell);

	assert_eq!(measured.total, 3, "two ifs and the elif, each at the top");
	assert_eq!(measured.nesting_penalty, 0);
	assert_eq!(
		measured.max_nesting, 1,
		"fi closes the chain, so the second if is not inside it"
	);
}

#[test]
fn logical_operator_sequences_are_unchanged() {
	let source = "\
fn check(a: bool, b: bool, c: bool) -> bool {
	a && b && c
}
";

	let measured = measure(source, Language::Rust);

	assert_eq!(measured.total, 1, "one sequence, one point");
}
