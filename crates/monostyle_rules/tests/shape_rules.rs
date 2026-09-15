//! Tests for the structural and comment rules.
//!
//! These cover the rules that measure shape and prose rather than layout: nesting depth, parameter
//! lists, unit and file size, and the comment rules. Each is asserted both for firing and for staying
//! quiet, because a rule that fires on correct code is worse than one that never fires.

use monostyle_core::Language;
use monostyle_lexer::lex;
use monostyle_rules::RulesConfig;
use monostyle_rules::comments;
use monostyle_rules::structure;

/// Runs a rule over Rust source with the default configuration.
fn run(
	rule: fn(&monostyle_lexer::LexedFile, &RulesConfig) -> Vec<monostyle_core::Finding>,
	source: &str,
) -> Vec<monostyle_core::Finding> {
	let lexed = lex(source, Language::Rust);

	rule(&lexed, &RulesConfig::default())
}

/// Runs the documentation rule, which takes no configuration.
fn run_documentation(source: &str) -> Vec<monostyle_core::Finding> {
	let lexed = lex(source, Language::Rust);

	comments::thin_documentation(&lexed)
}

// ---------------------------------------------------------------------------
// Nesting
// ---------------------------------------------------------------------------

#[test]
fn deeply_nested_control_flow_is_reported() {
	let source = "\
fn a(x: i32) {
    if x > 0 {
        if x > 1 {
            if x > 2 {
                if x > 3 {
                    work();
                }
            }
        }
    }
}
";
	let findings = run(structure::deep_nesting, source);

	assert!(
		!findings.is_empty(),
		"four levels of nesting should be reported"
	);
	assert!(findings[0].message.contains("nesting level"));
}

#[test]
fn nesting_within_the_limit_is_accepted() {
	let source = "\
fn a(x: i32) {
    if x > 0 {
        if x > 1 {
            work();
        }
    }
}
";
	assert_eq!(
		run(structure::deep_nesting, source),
		[] as [monostyle_core::Finding; 0]
	);
}

#[test]
fn nesting_depth_is_configurable() {
	let source = "\
fn a(x: i32) {
    if x > 0 {
        if x > 1 {
            work();
        }
    }
}
";
	let lexed = lex(source, Language::Rust);

	let strict = RulesConfig {
		max_nesting_depth: 1,
		..RulesConfig::default()
	};

	assert_ne!(
		structure::deep_nesting(&lexed, &strict),
		[] as [monostyle_core::Finding; 0]
	);
}

#[test]
fn a_deeply_nested_declaration_is_not_reported_as_a_deep_statement() {
	// A `struct` inside a function is a declaration rather than control flow, so the rule looks for
	// nesting keywords and not depth alone.
	let source = "\
fn a() {
    struct Config {
        value: i32,
    }
}
";
	assert_eq!(
		run(structure::deep_nesting, source),
		[] as [monostyle_core::Finding; 0]
	);
}

// ---------------------------------------------------------------------------
// Parameter lists
// ---------------------------------------------------------------------------

#[test]
fn a_wide_argument_list_is_reported() {
	let source = "fn a() { let value = compute(first, second, third, fourth, fifth); }\n";
	let findings = run(structure::long_parameter_list, source);

	assert_eq!(
		findings.len(),
		1,
		"five arguments is over the limit of three"
	);
}

#[test]
fn a_narrow_argument_list_is_accepted() {
	let source = "fn a() { let value = compute(first, second); }\n";

	assert_eq!(
		run(structure::long_parameter_list, source),
		[] as [monostyle_core::Finding; 0]
	);
}

#[test]
fn a_short_numeric_call_is_accepted_even_with_many_arguments() {
	// The rule measures rendered width as well as count, so `Span::new(0, 0, 1, 1)` is not reported.
	let source = "fn a() { let span = Span::new(0, 0, 1, 1); }\n";

	assert!(
		run(structure::long_parameter_list, source).is_empty(),
		"a short numeric call reads fine and should not be reported"
	);
}

#[test]
fn a_split_argument_list_is_accepted() {
	let source = "\
fn a() {
    let value = compute(
        first,
        second,
        third,
        fourth,
        fifth,
    );
}
";
	assert!(
		run(structure::long_parameter_list, source).is_empty(),
		"a list already split across lines has been given its space"
	);
}

#[test]
fn parentheses_inside_a_string_do_not_count_as_arguments() {
	// The count is taken from the masked view, so a string full of commas cannot inflate it.
	let source = "fn a() { let text = \"one, two, three, four, five\"; }\n";

	assert_eq!(
		run(structure::long_parameter_list, source),
		[] as [monostyle_core::Finding; 0]
	);
}

// ---------------------------------------------------------------------------
// Size
// ---------------------------------------------------------------------------

#[test]
fn an_oversized_file_is_reported() {
	let lines: String = (0..700)
		.map(|index| {
			format!(
				"const V{index}: i32 = {index};
"
			)
		})
		.fold(String::new(), |mut text, line| {
			text.push_str(&line);

			text
		});
	let findings = run(structure::oversized_file, &lines);

	assert_eq!(
		findings.len(),
		1,
		"a seven-hundred-line file exceeds the default limit"
	);
}

#[test]
fn a_normal_file_is_accepted() {
	assert_eq!(
		run(structure::oversized_file, "fn a() {}\n"),
		[] as [monostyle_core::Finding; 0]
	);
}

#[test]
fn an_oversized_unit_is_reported() {
	let body: String = (0..100)
		.map(|index| {
			format!(
				"    let v{index} = {index};
"
			)
		})
		.fold(String::new(), |mut text, line| {
			text.push_str(&line);

			text
		});
	let source = format!("fn a() {{\n{body}}}\n");

	let findings = run(structure::oversized_units, &source);

	assert!(
		!findings.is_empty(),
		"a hundred-line function exceeds the default limit"
	);
	assert!(findings[0].message.contains("spans"));
}

#[test]
fn a_file_limit_of_zero_disables_the_rule() {
	let lines: String = (0..700)
		.map(|index| {
			format!(
				"const V{index}: i32 = {index};
"
			)
		})
		.fold(String::new(), |mut text, line| {
			text.push_str(&line);

			text
		});
	let lexed = lex(&lines, Language::Rust);

	let config = RulesConfig {
		max_file_lines: 0,
		..RulesConfig::default()
	};

	assert_eq!(
		structure::oversized_file(&lexed, &config),
		[] as [monostyle_core::Finding; 0]
	);
}

// ---------------------------------------------------------------------------
// Comments
// ---------------------------------------------------------------------------

#[test]
fn a_complex_unit_without_an_explanation_is_reported() {
	let source = "\
fn tangled(x: i32) -> i32 {
    if x > 0 {
        if x > 1 {
            if x > 2 {
                if x > 3 {
                    return x;
                }
            }
        } else if x < 0 {
            if x < -1 {
                return -x;
            }
        }
    }
    x
}
";
	let findings = run(comments::comment_required_on_complex_units, source);

	assert!(
		!findings.is_empty(),
		"a complex function with no comment should be reported"
	);
	assert!(findings[0].message.contains("no comment"));
}

#[test]
fn a_complex_unit_with_an_explanation_is_accepted() {
	let source = "\
/// Why this exists: the upstream client fails transiently, and one retry policy
/// here is better than scattering error handling across every call site.
fn tangled(x: i32) -> i32 {
    if x > 0 {
        if x > 1 {
            if x > 2 {
                if x > 3 {
                    return x;
                }
            }
        } else if x < 0 {
            if x < -1 {
                return -x;
            }
        }
    }
    x
}
";
	assert!(
		run(comments::comment_required_on_complex_units, source).is_empty(),
		"a documented function should not be asked for a comment it has"
	);
}

#[test]
fn an_explanation_earns_credit() {
	let source =
		"// This exists because the upstream client fails transiently under load.\nfn a() {}\n";
	let findings = run(comments::comments_explaining_why, source);

	assert_eq!(findings.len(), 1);
	assert!(
		findings[0].penalty() < 0.0,
		"an explanation should earn credit, not cost points"
	);
}

#[test]
fn a_narrating_comment_costs_points() {
	let source = "// Increment the retry counter\nfn a() {}\n";
	let findings = run(comments::comments_narrating_code, source);

	assert_eq!(findings.len(), 1);
	assert!(findings[0].penalty() > 0.0);
}

#[test]
fn narrating_comments_can_be_tolerated() {
	let lexed = lex(
		"// Increment the retry counter\nfn a() {}\n",
		Language::Rust,
	);
	let config = RulesConfig {
		penalize_narrating_comments: false,
		..RulesConfig::default()
	};

	assert_eq!(
		comments::comments_narrating_code(&lexed, &config),
		[] as [monostyle_core::Finding; 0]
	);
}

#[test]
fn documentation_is_neither_credited_nor_penalized() {
	// A doc comment is expected on a public item, so it should not move a score either way.
	let source = "/// Returns the total.\nfn total() -> i32 { 1 }\n";

	assert_eq!(
		run(comments::comments_explaining_why, source),
		[] as [monostyle_core::Finding; 0]
	);
	assert_eq!(
		run(comments::comments_narrating_code, source),
		[] as [monostyle_core::Finding; 0]
	);
}

#[test]
fn a_file_with_more_comments_than_code_is_reported() {
	let mut source: String = (0..60)
		.map(|index| {
			format!(
				"// An ordinary note number {index} about the code below.
"
			)
		})
		.fold(String::new(), |mut text, line| {
			text.push_str(&line);

			text
		});
	source.push_str("fn a() {\n    work();\n}\n");

	let findings = run(comments::excessive_comments, &source);

	assert!(
		!findings.is_empty(),
		"a file that is mostly comment should be reported"
	);
}

#[test]
fn documentation_does_not_count_toward_the_comment_ratio() {
	// A well-documented module legitimately has more comment lines than code, so penalizing it would push
	// authors toward documenting less.
	let mut source: String = (0..60)
		.map(|index| {
			format!(
				"/// Documents item number {index} in detail.
"
			)
		})
		.fold(String::new(), |mut text, line| {
			text.push_str(&line);

			text
		});
	source.push_str("pub fn a() {\n    work();\n}\n");

	assert!(
		run(comments::excessive_comments, &source).is_empty(),
		"documentation should not be counted as excessive commentary"
	);
}

#[test]
fn a_short_file_is_not_reported_for_its_comment_ratio() {
	// Below the minimum code volume the ratio is dominated by rounding.
	assert_eq!(
		run(comments::excessive_comments, "// A note.\nfn a() {}\n"),
		[] as [monostyle_core::Finding; 0]
	);
}

#[test]
fn a_thin_documentation_block_is_reported() {
	let source = "\
/// @param a the first value
/// @param b the second value
/// @returns the sum
fn add(a: i32, b: i32) -> i32 {
    a + b
}
";
	let findings = run_documentation(source);

	assert!(
		!findings.is_empty(),
		"a block of annotations with no prose should be reported"
	);
}

#[test]
fn a_documentation_block_with_prose_is_accepted() {
	let source = "\
/// Adds two values together, saturating at the boundary.
///
/// @param a the first value
/// @param b the second value
fn add(a: i32, b: i32) -> i32 {
    a.saturating_add(b)
}
";
	assert!(
		run_documentation(source).is_empty(),
		"a block that explains itself should be accepted"
	);
}

#[test]
fn a_short_documentation_block_is_below_the_threshold() {
	let source = "/// The total.\nfn total() -> i32 { 1 }\n";

	assert_eq!(
		run_documentation(source),
		[] as [monostyle_core::Finding; 0]
	);
}

// ---------------------------------------------------------------------------
// Configuration switches
// ---------------------------------------------------------------------------

#[test]
fn comment_rules_can_be_turned_off() {
	// A block whose only prose is a handful of words, which is what the thin-documentation rule targets.
	let source = "\
/// @param a the first value
/// @param b the second value
/// @returns the sum
fn add(a: i32, b: i32) -> i32 {
    a + b
}
";
	let lexed = lex(source, Language::Rust);
	let config = RulesConfig {
		require_comment_on_complex_units: false,
		..RulesConfig::default()
	};

	// The require-comment rule is switchable, and the documentation rule is not: it fires on a doc block
	// that is structurally thin regardless of configuration.
	assert_eq!(
		comments::comment_required_on_complex_units(&lexed, &config),
		[] as [monostyle_core::Finding; 0]
	);
	assert_ne!(
		comments::thin_documentation(&lexed),
		[] as [monostyle_core::Finding; 0]
	);
}
