//! Tests for the quality rules and line-length rule.
//!
//! These rules are judgement calls rather than formatting, so most of what these tests assert is the
//! *absence* of a finding: a rule that fires on idiomatic code teaches authors to ignore the tool.

use monostyle_core::Language;
use monostyle_lexer::lex;
use monostyle_rules::RulesConfig;
use monostyle_rules::line_length;
use monostyle_rules::quality;

/// Runs a rule over a Rust snippet.
fn run(
	rule: fn(&monostyle_lexer::LexedFile, &RulesConfig) -> Vec<monostyle_core::Finding>,
	source: &str,
) -> Vec<monostyle_core::Finding> {
	let lexed = lex(source, Language::Rust);

	rule(&lexed, &RulesConfig::default())
}

/// Runs the line-length rule.
fn run_lines(source: &str) -> Vec<monostyle_core::Finding> {
	let lexed = lex(source, Language::Rust);

	line_length::overlong_lines(&lexed, &RulesConfig::default())
}

// ---------------------------------------------------------------------------
// Magic numbers
// ---------------------------------------------------------------------------

#[test]
fn a_meaningful_literal_is_reported() {
	let findings = run(
		quality::magic_numbers,
		"fn a() { let d = elapsed * 86400; }\n",
	);

	assert_eq!(
		findings.len(),
		1,
		"86400 is a day in seconds and needs a name"
	);
	assert!(findings[0].message.contains("86400"));
}

#[test]
fn conventional_literals_are_left_alone() {
	// Flagging `0` and `1` is the fastest way to make a reader stop reading findings.
	let findings = run(
		quality::magic_numbers,
		"fn a(x: i32) { let y = x + 0; let z = x * 1; let w = x / 2; let v = x % 10; }\n",
	);

	assert!(
		findings.is_empty(),
		"conventional values should not be reported: {findings:?}"
	);
}

#[test]
fn a_literal_in_a_constant_declaration_is_already_named() {
	let findings = run(quality::magic_numbers, "const TIMEOUT: i32 = 86400;\n");

	assert!(
		findings.is_empty(),
		"a constant declaration is the fix, not the problem: {findings:?}"
	);
}

#[test]
fn digits_inside_an_identifier_are_not_literals() {
	let findings = run(
		quality::magic_numbers,
		"fn a() { let base64_value = compute(); }\n",
	);

	assert!(
		findings.is_empty(),
		"`base64` is an identifier, not a number: {findings:?}"
	);
}

#[test]
fn magic_numbers_can_be_turned_off() {
	let config = RulesConfig {
		report_magic_numbers: false,

		..RulesConfig::default()
	};
	let lexed = lex("fn a() { let d = elapsed * 86400; }\n", Language::Rust);

	assert_eq!(
		quality::magic_numbers(&lexed, &config),
		[] as [monostyle_core::Finding; 0]
	);
}

// ---------------------------------------------------------------------------
// Short identifiers
// ---------------------------------------------------------------------------

#[test]
fn an_unconventional_short_name_is_reported() {
	let findings = run(quality::short_identifiers, "let ab = compute();\n");

	assert_eq!(findings.len(), 1, "`ab` carries no meaning: {findings:?}");
}

#[test]
fn conventional_short_names_are_kept() {
	// A loop index named `i` and a coordinate named `x` are clearer than any longer alternative, so
	// reporting them would be wrong.
	let findings = run(
		quality::short_identifiers,
		"fn a() { for i in items { let x = i; let y = x; } }\n",
	);

	assert!(
		findings.is_empty(),
		"idiomatic short names should not be reported: {findings:?}"
	);
}

#[test]
fn names_that_meet_the_length_are_ignored() {
	let findings = run(quality::short_identifiers, "let value = compute();\n");

	assert_eq!(findings, [] as [monostyle_core::Finding; 0]);
}

#[test]
fn short_identifiers_can_be_turned_off() {
	let config = RulesConfig {
		report_short_identifiers: false,

		..RulesConfig::default()
	};
	let lexed = lex("let ab = compute();\n", Language::Rust);

	assert_eq!(
		quality::short_identifiers(&lexed, &config),
		[] as [monostyle_core::Finding; 0]
	);
}

// ---------------------------------------------------------------------------
// Empty handlers
// ---------------------------------------------------------------------------

#[test]
fn a_handler_with_an_empty_body_is_reported() {
	let findings = run(
		quality::empty_handlers,
		"fn a() {\n    match run() {\n        Err(_) => {\n        }\n        Ok(v) => use_it(v),\n    }\n}\n",
	);

	assert!(
		!findings.is_empty(),
		"a silently discarded error should be reported"
	);
}

#[test]
fn a_handler_that_acts_on_the_error_is_kept() {
	let findings = run(
		quality::empty_handlers,
		"fn a() {\n    match run() {\n        Err(error) => log_error(error),\n        Ok(v) => use_it(v),\n    }\n}\n",
	);

	assert!(findings.is_empty(), "the error is handled: {findings:?}");
}

#[test]
fn a_multiline_handler_that_acts_on_the_error_is_kept() {
	// The arm opens a brace and continues on later lines. Reading the empty opening tail as an
	// inline body once reported every multi-line handler as discarding its error.
	let findings = run(
		quality::empty_handlers,
		"fn a() {\n    match run() {\n        Err(error) => {\n            log_error(error);\n        }\n        Ok(v) => use_it(v),\n    }\n}\n",
	);

	assert!(
		findings.is_empty(),
		"the error is handled across lines: {findings:?}"
	);
}

#[test]
fn empty_handlers_can_be_turned_off() {
	let config = RulesConfig {
		report_empty_handlers: false,

		..RulesConfig::default()
	};
	let lexed = lex(
		"fn a() {\n    match run() {\n        Err(_) => {\n        }\n        Ok(v) => use_it(v),\n    }\n}\n",
		Language::Rust,
	);

	assert_eq!(
		quality::empty_handlers(&lexed, &config),
		[] as [monostyle_core::Finding; 0]
	);
}

// ---------------------------------------------------------------------------
// Commented-out code
// ---------------------------------------------------------------------------

#[test]
fn a_block_of_commented_out_code_is_reported() {
	let findings = run(
		quality::commented_out_code,
		"// fn old(a: i32) -> i32 {\n//     if a > 0 { return a; }\n//     a\n// }\nfn current() {}\n",
	);

	assert_eq!(
		findings.len(),
		1,
		"commented-out code should be reported: {findings:?}"
	);
}

#[test]
fn prose_comments_are_not_reported_as_code() {
	// A multi-sentence explanation is the thing this tool asks for, so misreading it as code would be
	// the worst possible false positive.
	let findings = run(
		quality::commented_out_code,
		"// This function exists because the upstream client fails transiently\n\
		 // under load, and a single retry policy here is better than scattering\n\
		 // try blocks across every call site in the module.\nfn a() {}\n",
	);

	assert!(
		findings.is_empty(),
		"prose should never be read as code: {findings:?}"
	);
}

#[test]
fn a_single_commented_line_is_below_the_threshold() {
	let findings = run(
		quality::commented_out_code,
		"// let x = compute();\nfn a() {}\n",
	);

	assert!(findings.is_empty(), "one line is not a block");
}

#[test]
fn commented_out_code_can_be_turned_off() {
	let config = RulesConfig {
		report_commented_out_code: false,

		..RulesConfig::default()
	};
	let lexed = lex(
		"// fn old() {\n//     work();\n// }\nfn current() {}\n",
		Language::Rust,
	);

	assert_eq!(
		quality::commented_out_code(&lexed, &config),
		[] as [monostyle_core::Finding; 0]
	);
}

// ---------------------------------------------------------------------------
// Line length
// ---------------------------------------------------------------------------

#[test]
fn a_long_breakable_line_is_reported() {
	// A call with several arguments has somewhere to wrap to, so a finding is actionable.
	let arguments: String = (0..30).map(|index| format!("argument_{index}, ")).fold(
		String::new(),
		|mut text, argument| {
			text.push_str(&argument);

			text
		},
	);
	let source = format!("fn a() {{ let result = compute({arguments}done); }}\n");

	let findings = run_lines(&source);

	assert_eq!(findings.len(), 1, "a long argument list should be reported");
}

#[test]
fn a_line_at_the_limit_is_accepted() {
	// The boundary matters because a project sets this to match its formatter, and an off-by-one would
	// report every line the formatter considers correct.
	let padding = "a".repeat(100);
	let source = format!("fn {padding}() {{}}\n");

	assert!(
		source.trim_end().len() <= 120,
		"the fixture should be within the limit"
	);
	assert!(
		run_lines(&source).is_empty(),
		"a line within the limit should be accepted"
	);
}

#[test]
fn the_limit_is_configurable() {
	let source = "fn a() { let value = compute(one, two, three); }\n";

	let strict = RulesConfig {
		max_line_width: 20,

		..RulesConfig::default()
	};
	let relaxed = RulesConfig {
		max_line_width: 400,

		..RulesConfig::default()
	};

	let lexed = lex(source, Language::Rust);

	assert_ne!(
		line_length::overlong_lines(&lexed, &strict),
		[] as [monostyle_core::Finding; 0]
	);
	assert_eq!(
		line_length::overlong_lines(&lexed, &relaxed),
		[] as [monostyle_core::Finding; 0]
	);
}

#[test]
fn a_limit_of_zero_disables_the_rule() {
	let config = RulesConfig {
		max_line_width: 0,

		..RulesConfig::default()
	};
	let lexed = lex(
		"fn a() { let value = compute(one, two, three); }\n",
		Language::Rust,
	);

	assert_eq!(
		line_length::overlong_lines(&lexed, &config),
		[] as [monostyle_core::Finding; 0]
	);
}

#[test]
fn an_unbreakable_long_line_is_not_reported() {
	// A single long literal or URL has nowhere to wrap to, so a finding would ask for something the
	// language does not allow.
	let literal = "x".repeat(300);
	let source = format!("fn a() {{ let url = \"{literal}\"; }}\n");

	assert!(
		run_lines(&source).is_empty(),
		"a line that cannot be broken should not be reported"
	);
}

#[test]
fn severity_rises_past_the_severe_ratio() {
	let config = RulesConfig::default();

	assert_eq!(config.severe_line_width(), (120.0 * 1.35) as usize);
}

#[test]
fn severe_width_never_equals_the_limit() {
	// A ratio below one would make the severe band narrower than the limit, so every finding would be
	// severe. The accessor guards against that.
	let config = RulesConfig {
		max_line_width: 100,
		severe_line_width_ratio: 0.1,

		..RulesConfig::default()
	};

	assert!(config.severe_line_width() > config.max_line_width);
}

#[test]
fn markdown_prose_is_never_reported() {
	// The rule is skipped entirely for Markdown, because prose has no line limit and a wrapped
	// paragraph legitimately exceeds any code width.
	let source = format!("# Title\n\n{}\n", "word ".repeat(100));
	let lexed = lex(&source, Language::Markdown);

	assert!(
		line_length::overlong_lines(&lexed, &RulesConfig::default()).is_empty(),
		"Markdown prose must never produce a line-length finding"
	);
}

// ---------------------------------------------------------------------------
// Line length inside fences
// ---------------------------------------------------------------------------

#[test]
fn an_overlong_line_inside_a_markdown_fence_is_reported_against_the_document() {
	// The Markdown rule path forwards to the line-length rule, so a missing forwarding call would make
	// every fence exempt from a limit its own language enforces.
	use monostyle_lexer::lex;
	use monostyle_rules::markdown;

	// The line must exceed the configured limit, which is 120 columns by default.
	let arguments: String = (0..20)
		.map(|index| format!("argument_number_{index}, "))
		.fold(String::new(), |mut text, argument| {
			text.push_str(&argument);

			text
		});
	let source = format!("```rust\nfn a() {{ let value = compute({arguments}done); }}\n```\n");
	let lexed = lex(&source, Language::Markdown);

	let findings = markdown::fence_readability(&lexed, &RulesConfig::default());

	assert!(
		findings
			.iter()
			.any(|finding| finding.rule == "readability/overlong-line"),
		"a long line inside a fence should be reported: {findings:?}"
	);
}

#[test]
fn markdown_prose_is_still_exempt_from_the_limit() {
	// The fence path uses the same rule as source, and that rule must still skip prose.
	use monostyle_lexer::lex;
	use monostyle_rules::line_length;

	let prose = format!("# Title\n\n{}\n", "word ".repeat(200));
	let lexed = lex(&prose, Language::Markdown);

	assert!(
		line_length::overlong_lines(&lexed, &RulesConfig::default()).is_empty(),
		"Markdown prose has no line limit"
	);
}
