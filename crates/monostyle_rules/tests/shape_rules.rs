//! Tests for the structural and comment rules.
//!
//! These cover the rules that measure shape and prose rather than layout: nesting depth, parameter
//! lists, unit and file size, and the comment rules. Each is asserted both for firing and for staying
//! quiet, because a rule that fires on correct code is worse than one that never fires.

use monostyle_core::Language;
use monostyle_lexer::lex;
use monostyle_rules::RulesConfig;
use monostyle_rules::comments;
use monostyle_rules::complexity;

/// Joins generated lines into one string.
///
/// Building a fixture by mapping `format!` over a range and collecting is the shape clippy flags, and a
/// named helper states the intent more plainly than the fold it expands to.
fn lines_of(items: impl IntoIterator<Item = String>) -> String {
	items.into_iter().fold(String::new(), |mut text, line| {
		text.push_str(&line);

		text
	})
}

use monostyle_rules::structure;

/// Runs a rule over Rust source with the default configuration.
fn run(
	rule: fn(&monostyle_lexer::LexedFile, &RulesConfig) -> Vec<monostyle_core::Finding>,
	source: &str,
) -> Vec<monostyle_core::Finding> {
	let lexed = lex(source, Language::Rust);

	rule(&lexed, &RulesConfig::default())
}

/// Runs the exit-count rule.
fn complexity_exits(
	file: &monostyle_lexer::LexedFile,
	config: &RulesConfig,
) -> Vec<monostyle_core::Finding> {
	complexity::exits_per_unit(file, config)
}

/// Runs the `NPath` rule.
fn complexity_npath(
	file: &monostyle_lexer::LexedFile,
	config: &RulesConfig,
) -> Vec<monostyle_core::Finding> {
	complexity::npath_per_unit(file, config)
}

/// Runs the maintainability rule.
fn complexity_maintainability(
	file: &monostyle_lexer::LexedFile,
	config: &RulesConfig,
) -> Vec<monostyle_core::Finding> {
	complexity::maintainability_per_unit(file, config)
}

/// Runs the cyclomatic rule.
fn complexity_cyclomatic(
	file: &monostyle_lexer::LexedFile,
	config: &RulesConfig,
) -> Vec<monostyle_core::Finding> {
	complexity::cyclomatic_per_unit(file, config)
}

/// Runs the per-file complexity rule.
fn complexity_per_file(
	file: &monostyle_lexer::LexedFile,
	config: &RulesConfig,
) -> Vec<monostyle_core::Finding> {
	complexity::complexity_per_file(file, config)
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
	// The rule reports on count past a grace margin or on width. This call trips the count: six arguments
	// against a limit of three is two past the margin.
	let source = "fn a() { let value = compute(first, second, third, fourth, fifth, sixth); }\n";
	let findings = run(structure::long_parameter_list, source);

	assert_eq!(
		findings.len(),
		1,
		"six arguments is past the limit and its margin"
	);
}

#[test]
fn a_call_with_a_few_short_arguments_is_accepted() {
	// Four short arguments span about thirty columns and read fine. The margin exists so the rule does not
	// fire on almost every call in a real codebase, which is what made a reader stop trusting it.
	let source = "fn a() { let value = compute(first, second, third, fourth); }\n";

	assert_eq!(
		run(structure::long_parameter_list, source),
		[] as [monostyle_core::Finding; 0]
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
fn a_doc_comment_behind_an_attribute_still_counts() {
	// Rust places attributes between a doc comment and the declaration it decorates. Reading only
	// the line directly above the declaration once asked such a function for a comment it already
	// had.
	let source = "\
/// Why this exists: the upstream client fails transiently, and one retry policy
/// here is better than scattering error handling across every call site.
#[must_use]
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
		"a doc comment behind an attribute should still count as documentation"
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

// ---------------------------------------------------------------------------
// Exit counting
// ---------------------------------------------------------------------------

#[test]
fn a_unit_with_many_returns_is_reported() {
	// Early returns are preferred to nesting, so the rule only fires well past the point where guards are
	// idiomatic.
	let arms: String = (0..12)
		.map(|index| {
			format!(
				"    if x == {index} {{ return {index}; }}
"
			)
		})
		.fold(String::new(), |mut text, line| {
			text.push_str(&line);

			text
		});
	let source = format!("fn many(x: i32) -> i32 {{\n{arms}    0\n}}\n");

	let findings = run(complexity_exits, &source);

	assert!(
		!findings.is_empty(),
		"twelve returns should exceed the limit"
	);
	assert!(findings[0].message.contains("returns from"));
}

#[test]
fn the_exit_message_names_raises_and_jumps() {
	// The breakdown is what tells a reader what kind of escapes they are consolidating.
	let arms: String = (0..10)
		.map(|index| {
			format!(
				"    if x == {index} {{ return {index}; }}
"
			)
		})
		.fold(String::new(), |mut text, line| {
			text.push_str(&line);

			text
		});
	let source = format!(
		"fn many(x: i32) -> i32 {{\n{arms}    if x > 99 {{ panic!(\"bad\"); }}\n    0\n}}\n"
	);

	let findings = run(complexity_exits, &source);

	assert_ne!(findings, [] as [monostyle_core::Finding; 0]);
	assert!(
		findings[0].message.contains("raise"),
		"the message should name the raises: {}",
		findings[0].message
	);
}

#[test]
fn loop_jumps_appear_in_the_exit_breakdown() {
	let arms: String = (0..10)
		.map(|index| {
			format!(
				"    if x == {index} {{ return {index}; }}
"
			)
		})
		.fold(String::new(), |mut text, line| {
			text.push_str(&line);

			text
		});
	let source = format!(
		"fn many(x: i32) -> i32 {{\n{arms}    for item in items {{ if item {{ continue; }} }}\n    0\n}}\n"
	);

	let findings = run(complexity_exits, &source);

	assert_ne!(findings, [] as [monostyle_core::Finding; 0]);
	assert!(
		findings[0].message.contains("jump"),
		"the message should name the jumps: {}",
		findings[0].message
	);
}

#[test]
fn a_unit_within_the_exit_limit_is_accepted() {
	assert_eq!(
		run(
			complexity_exits,
			"fn a(x: i32) -> i32 {\n    if x > 0 { return 1; }\n    0\n}\n"
		),
		[] as [monostyle_core::Finding; 0]
	);
}

// ---------------------------------------------------------------------------
// NPath and maintainability
// ---------------------------------------------------------------------------

#[test]
fn a_unit_with_many_paths_is_reported() {
	let arms: String = (0..14)
		.map(|index| {
			format!(
				"    if x > {index} {{ work(); }}
"
			)
		})
		.fold(String::new(), |mut text, line| {
			text.push_str(&line);

			text
		});
	let source = format!("fn many(x: i32) {{\n{arms}}}\n");

	let findings = run(complexity_npath, &source);

	assert!(
		!findings.is_empty(),
		"fourteen sequential branches exceed the path limit"
	);
	assert!(findings[0].message.contains("execution paths"));
}

#[test]
fn a_simple_unit_is_not_reported_for_its_paths() {
	assert_eq!(
		run(complexity_npath, "fn a() { work(); }\n"),
		[] as [monostyle_core::Finding; 0]
	);
}

#[test]
fn a_dense_unit_is_reported_for_low_maintainability() {
	// The index combines Halstead volume with complexity and length, so a unit dense with arithmetic and
	// branching scores low even when no other rule fires.
	let body: String = (0..40)
		.map(|index| {
			format!(
				"    let v{index} = a * {index} + b / {index} - c % {index};
"
			)
		})
		.fold(String::new(), |mut text, line| {
			text.push_str(&line);

			text
		});
	let source = format!("fn dense(a: i32, b: i32, c: i32) {{\n{body}}}\n");

	let findings = run(complexity_maintainability, &source);

	assert!(
		!findings.is_empty(),
		"a dense unit should have a low maintainability index"
	);
	assert!(findings[0].message.contains("maintainability index"));
}

#[test]
fn a_short_unit_is_never_reported_for_maintainability() {
	// Below the measurable length the index is a function of line count alone, so reporting it would be
	// noise.
	assert_eq!(
		run(complexity_maintainability, "fn a() { work(); }\n"),
		[] as [monostyle_core::Finding; 0]
	);
}

#[test]
fn a_clear_unit_is_accepted() {
	let source = "fn add(a: i32, b: i32) -> i32 {\n    let sum = a + b;\n\n    sum\n}\n";

	assert_eq!(
		run(complexity_maintainability, source),
		[] as [monostyle_core::Finding; 0]
	);
}

#[test]
fn the_cyclomatic_message_names_the_risk_band() {
	let arms: String = (0..14)
		.map(|index| {
			format!(
				"    if x > {index} {{ work(); }}
"
			)
		})
		.fold(String::new(), |mut text, line| {
			text.push_str(&line);

			text
		});
	let source = format!("fn many(x: i32) {{\n{arms}}}\n");

	let findings = run(complexity_cyclomatic, &source);

	assert_ne!(findings, [] as [monostyle_core::Finding; 0]);
	assert!(
		findings[0].message.contains("risk"),
		"the message should name the risk band: {}",
		findings[0].message
	);
}

#[test]
fn a_file_with_a_high_decision_density_is_reported() {
	// Density rather than a total, so a small file full of branches is caught while a large file with
	// proportionate complexity is not.
	let body = lines_of((0..60).map(|index| {
		format!(
			"    if x > {index} {{ work(); }}
"
		)
	}));
	let source = format!("fn many(x: i32) {{\n{body}}}\n");

	let findings = run(complexity_per_file, &source);

	assert!(!findings.is_empty(), "a dense file should be reported");
	assert!(findings[0].message.contains("decisions per 100 lines"));
}

#[test]
fn a_short_file_is_not_reported_for_density() {
	// Below the minimum length the ratio is dominated by rounding.
	assert_eq!(
		run(
			complexity_per_file,
			"fn a(x: i32) {\n    if x > 0 { work(); }\n}\n"
		),
		[] as [monostyle_core::Finding; 0]
	);
}
