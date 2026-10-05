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
fn coordinate_and_unit_abbreviations_are_kept_but_the_alphabet_is_not() {
	// `dx`/`ms` name a direction and a unit — every convention writes them. A bare
	// `d` or `e` names nothing; the author chose it, so the author can lengthen it.
	let kept = run(
		quality::short_identifiers,
		"fn a() { let dx = origin.x - target.x; let ms = elapsed.as_millis(); }\n",
	);

	assert!(
		kept.is_empty(),
		"coordinates and units are vocabulary: {kept:?}"
	);

	let chosen = run(
		quality::short_identifiers,
		"fn a() { let d = deadline(); let e = fetch(d); let op = e.kind; }\n",
	);

	assert_eq!(
		chosen.len(),
		3,
		"a chosen letter or abbreviation is a name to lengthen: {chosen:?}"
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

#[test]
fn a_rustdoc_error_list_is_not_commented_out_code() {
	// A rustdoc `# Errors` section lists variants as `Error::Variant`. Each of those lines contains the
	// `::` marker the code test looks for, so a naive run counts the documentation of a public API as
	// dead code. Reporting it would push authors to delete the documentation to raise their score.
	let findings = run(
		quality::commented_out_code,
		"/// Connects to the wallet.\n\
		 ///\n\
		 /// # Errors\n\
		 ///\n\
		 /// This method may return errors such as:\n\
		 /// - `WalletError::Connection` if the connection fails\n\
		 /// - `WalletError::WindowClosed` if the user closes the window\n\
		 /// - `WalletError::WindowBlocked` if the window is blocked\n\
		 pub fn connect() {}\n",
	);

	assert!(
		findings.is_empty(),
		"documentation should never be read as commented-out code: {findings:?}"
	);
}

#[test]
fn commented_out_code_adjacent_to_documentation_is_still_reported() {
	// Skipping documentation must not skip the code around it: the doc block ends the run above and
	// starts a new one, so a hidden implementation next to a doc comment is still found.
	let findings = run(
		quality::commented_out_code,
		"/// Explains the function below.\n\
		 ///\n\
		 /// - `Thing::Other` and `Thing::Another` are both handled\n\
		 // fn old(a: i32) -> i32 {\n\
		 //     if a > 0 { return a; }\n\
		 //     a\n\
		 // }\n\
		 pub fn current() {}\n",
	);

	assert_eq!(
		findings.len(),
		1,
		"hidden code beside documentation should still be reported: {findings:?}"
	);
}

#[test]
fn an_enum_discriminant_is_named_by_its_variant() {
	// `Insolvent = 5,` — the variant is the name. Requiring a constant for each discriminant
	// would duplicate the variant's own name.
	let findings = run(
		quality::magic_numbers,
		"enum Error {\n    Empty = 1,\n    InvalidMint = 6,\n    Insolvent = 5,\n}\n",
	);

	assert!(
		findings.is_empty(),
		"a discriminant names itself: {findings:?}"
	);
}

#[test]
fn a_type_parameter_is_named_by_its_type() {
	// `[u8; 32]` and `String<64>` say what the number means: a byte width and a capacity.
	let findings = run(
		quality::magic_numbers,
		"pub struct Config {\n    pub authority: [u8; 32],\n    pub label: String<64>,\n    pub values: Vec<u16, 8>,\n}\n",
	);

	assert!(
		findings.is_empty(),
		"type parameters name their own sizes: {findings:?}"
	);
}

#[test]
fn a_data_table_row_is_named_by_its_table() {
	// A row of byte values has no name to give each entry.
	let findings = run(
		quality::magic_numbers,
		"const WRAPPED_SOL_MINT: [u8; 32] = [\n    6, 155, 136, 87, 254, 171, 129, 132,\n    251, 104, 127, 99, 70, 24, 192, 53,\n];\n",
	);

	assert!(
		findings.is_empty(),
		"a table row names itself by column: {findings:?}"
	);
}

#[test]
fn a_literal_in_ordinary_code_is_still_reported() {
	// The exemptions are positional. A bare literal in the middle of logic is still unnamed.
	let findings = run(
		quality::magic_numbers,
		"fn rate(seconds: u64) -> u64 {\n    seconds * 86400 / 7\n}\n",
	);

	assert!(
		!findings.is_empty(),
		"an unnamed literal in logic is reported"
	);
}

#[test]
fn a_same_line_handler_that_acts_on_the_error_is_kept() {
	// Swift writes `} catch { result(flutterError(error)) }`; the earlier code fell through to
	// the next line, which is the enclosing brace, and read that as an empty body.
	let source = "func handle() {\n  do {\n    try work()\n  } catch { result(flutterError(error, code: \"invalid\")) }\n}\n";
	let lexed = lex(source, Language::Swift);
	let findings = quality::empty_handlers(&lexed, &RulesConfig::default());

	assert!(
		findings.is_empty(),
		"the handler acts on the error: {findings:?}"
	);
}

#[test]
fn a_same_line_handler_that_discards_the_error_is_reported() {
	let source = "func handle() {\n  do {\n    try work()\n  } catch {}\n}\n";
	let lexed = lex(source, Language::Swift);
	let findings = quality::empty_handlers(&lexed, &RulesConfig::default());

	assert_eq!(
		findings.len(),
		1,
		"an empty handler is reported: {findings:?}"
	);
}

#[test]
fn a_named_field_initialiser_names_its_literal() {
	// `max_line_width: 120` in a configuration default says what the number is; the field name
	// is the name. Requiring a constant for each field would rename the field twice.
	let findings = run(
		quality::magic_numbers,
		"impl Default for Config {\n    fn default() -> Self {\n        Self {\n            max_line_width: 120,\n            max_unit_lines: 80,\n            min_maintainability: 40.0,\n        }\n    }\n}\n",
	);

	assert!(
		findings.is_empty(),
		"a named field names its value: {findings:?}"
	);
}

#[test]
fn an_unnamed_tuple_value_is_still_reported() {
	// The exemption is the field name. A bare value in a tuple or argument list has none.
	let findings = run(
		quality::magic_numbers,
		"fn build() -> Limits {\n    Limits::new(120, 80)\n}\n",
	);

	assert!(!findings.is_empty(), "an unnamed argument is reported");
}

#[test]
fn an_assertions_literals_are_the_values_it_claims() {
	// `expect(disc(ix), 12)` and `assert_eq!(len, 3)` state what the code must
	// produce; naming the number would hide the claim. A literal in the logic
	// the assertion exercises is still a choice, and still reported.
	let claimed = run(
		quality::magic_numbers,
		"fn probe(ix: u8) {\n    expect(disc(ix), 12);\n    assert_eq!(len(&ix), 3);\n}\n",
	);

	assert!(
		claimed.is_empty(),
		"assertion arguments are the expected values: {claimed:?}"
	);

	let logic = run(
		quality::magic_numbers,
		"fn probe(ix: u8) {\n    let limit = compute(47);\n    expect(limit, 47);\n}\n",
	);

	assert_eq!(
		logic.len(),
		1,
		"a literal in the logic under assertion is still a choice: {logic:?}"
	);
}

#[test]
fn a_constructor_wrapped_named_field_names_its_literal() {
	// `lastValidBlockHeight: BigInt.from(123)` — the field names the value and
	// the constructor names the type; a constant on top adds nothing.
	let findings = run(
		quality::magic_numbers,
		"BlockMeta {\n    lastValidBlockHeight: BigInt.from(123),\n    unixTimestamp: u64(500),\n}\n",
	);

	assert!(
		findings.is_empty(),
		"the field and the constructor name the number: {findings:?}"
	);
}

#[test]
fn a_collection_of_only_literals_is_data() {
	// A byte fixture or a golden vector is data; naming its numbers would
	// obscure the shape the reader is checking.
	let findings = run(
		quality::magic_numbers,
		"fn fixture() {\n    messageBytes: Uint8List.fromList([1, 2, 3, 4, 5]),\n}\n",
	);

	assert!(
		findings.is_empty(),
		"an all-literal collection is a data table: {findings:?}"
	);
}

#[test]
fn a_value_with_a_closer_before_its_opener_is_not_a_constructor() {
	// `b) c(7` has a `)` before its `(`, so it is not a constructor call and it
	// does not name anything — the 7 stays a finding, and reading the value
	// must not panic on the reversed range.
	let findings = run(
		quality::magic_numbers,
		"fn edge() {
    a: b) c(7),
}
",
	);

	assert_eq!(
		findings.len(),
		1,
		"the reversed value is not a constructor: {findings:?}"
	);
}
