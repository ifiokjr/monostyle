//! Regression tests for separating runs of statements.
//!
//! The boundary is the contract: a run is reported only when it is longer than
//! `max-statements-per-group`, and the message names the limit so a reader can split a run until the
//! finding clears. Splitting must therefore always be able to clear it — that is the case the rule
//! previously could not satisfy, which is what led an auto-fixer to stack blank lines instead.

use std::fmt::Write;

use monostyle_core::Finding;
use monostyle_core::Language;
use monostyle_lexer::lex;
use monostyle_rules::RulesConfig;
use monostyle_rules::whitespace;

/// Runs the group-separation rule over a source snippet.
fn groups_in(source: &str, language: Language) -> Vec<Finding> {
	let lexed = lex(source, language);

	whitespace::group_separation(&lexed, &RulesConfig::default())
}

/// Runs the rule with a raised limit.
fn groups_with_limit(source: &str, language: Language, limit: usize) -> Vec<Finding> {
	let lexed = lex(source, language);
	let config = RulesConfig {
		max_statements_per_group: limit,
		..RulesConfig::default()
	};

	whitespace::group_separation(&lexed, &config)
}

/// Builds a run of numbered one-line statements.
fn statements(count: usize) -> String {
	let mut text = String::new();

	push_numbered(&mut text, count, |index| format!("call_{index}();"));

	text
}

/// Appends `count` numbered lines to a fixture, each built by `template`.
///
/// Writing into the buffer rather than appending a formatted string is what keeps these fixtures from
/// allocating once per line, which is the shape `clippy::format_push_string` asks for.
fn push_numbered(text: &mut String, count: usize, template: impl Fn(usize) -> String) {
	for index in 1..=count {
		let _ = writeln!(text, "{}", template(index));
	}
}

// ---------------------------------------------------------------------------
// The boundary
// ---------------------------------------------------------------------------

#[test]
fn a_run_longer_than_the_limit_is_reported_with_the_limit_in_the_message() {
	let findings = groups_in(&statements(9), Language::Dart);

	assert_eq!(
		findings.len(),
		1,
		"nine statements exceed the limit: {findings:?}"
	);
	assert!(
		findings[0].message.contains("9 statements"),
		"the message names the run length: {}",
		findings[0].message
	);
	assert!(
		findings[0].message.contains("(limit 8)"),
		"the message names the limit so the run can be split until it clears: {}",
		findings[0].message
	);
}

#[test]
fn a_run_exactly_at_the_limit_is_not_reported() {
	let findings = groups_in(&statements(8), Language::Dart);

	assert!(
		findings.is_empty(),
		"eight statements are the most a group may hold: {findings:?}"
	);
}

#[test]
fn a_run_below_the_limit_is_not_reported() {
	let findings = groups_in(&statements(7), Language::Dart);

	assert!(
		findings.is_empty(),
		"seven statements are well within the limit: {findings:?}"
	);
}

// ---------------------------------------------------------------------------
// Splitting must clear the finding
// ---------------------------------------------------------------------------

#[test]
fn splitting_a_long_run_in_two_still_reports_each_half_over_the_limit() {
	// Two halves of twelve are each over the limit of eight, so the split has not gone far enough and
	// both halves are reported. This is the honest answer: the finding names the limit, so following it
	// means splitting again.
	let mut source = statements(12);
	source.push('\n');
	source.push_str(&statements(12));

	let findings = groups_in(&source, Language::Dart);

	assert_eq!(
		findings.len(),
		2,
		"each twelve-statement half is over the limit: {findings:?}"
	);
	assert!(
		findings
			.iter()
			.all(|finding| finding.message.contains("12 statements")),
		"both halves report their own length: {findings:?}"
	);
}

#[test]
fn splitting_a_long_run_until_every_group_fits_clears_the_finding() {
	// The case the rule could not previously satisfy. Six statements per group is under the limit, so
	// the finding is gone — which is what makes the rule's own advice followable.
	let mut source = String::new();

	for group in 0..4 {
		push_numbered(&mut source, 6, |index| format!("call_{group}_{index}();"));

		source.push('\n');
	}

	let findings = groups_in(&source, Language::Dart);

	assert!(
		findings.is_empty(),
		"four groups of six are all within the limit: {findings:?}"
	);
}

// ---------------------------------------------------------------------------
// Function bodies are measured
// ---------------------------------------------------------------------------

#[test]
fn a_long_run_inside_a_function_body_is_reported() {
	// The blind spot: every statement in a function body was classified as a new item, so a run inside
	// one was never measured. A function body is exactly the sequence this rule exists to measure.
	let mut source = String::from("fn work() {\n");

	push_numbered(&mut source, 10, |index| format!("    step_{index}();"));

	source.push_str("}\n");

	let findings = groups_in(&source, Language::Rust);

	assert_eq!(
		findings.len(),
		1,
		"a ten-statement function body is one long run: {findings:?}"
	);
	assert!(
		findings[0].message.contains("10 statements"),
		"the body's statements are what is counted, not its declaration: {}",
		findings[0].message
	);
}

#[test]
fn a_function_body_split_into_fitting_groups_is_not_reported() {
	let mut source = String::from("fn work() {\n");

	push_numbered(&mut source, 8, |index| format!("    step_{index}();"));

	source.push_str("\n    tail();\n}\n");

	let findings = groups_in(&source, Language::Rust);

	assert!(
		findings.is_empty(),
		"a split body is two fitting groups: {findings:?}"
	);
}

#[test]
fn a_long_run_inside_a_control_flow_block_is_reported() {
	let mut source = String::from("fn work() {\n    if flag {\n");

	push_numbered(&mut source, 9, |index| format!("        step_{index}();"));

	source.push_str("    }\n}\n");

	let findings = groups_in(&source, Language::Rust);

	assert_eq!(
		findings.len(),
		1,
		"statements inside an if block are a run too: {findings:?}"
	);
}

#[test]
fn a_declaration_without_a_keyword_breaks_the_run() {
	// Dart declares methods with no keyword, so `void emit()` is only recognizable from its shape. The
	// declaration is an item rather than a statement, so the run it introduces is measured separately
	// instead of being counted from the declaration line.
	let mut source = String::from("void emit() {\n");

	push_numbered(&mut source, 10, |index| format!("  step_{index}();"));

	source.push_str("}\n");

	let findings = groups_in(&source, Language::Dart);

	assert_eq!(findings.len(), 1, "the body is one long run: {findings:?}");
	assert!(
		findings[0].message.contains("10 statements"),
		"the declaration is not counted as a statement: {}",
		findings[0].message
	);
}

// ---------------------------------------------------------------------------
// Items and data are not statements
// ---------------------------------------------------------------------------

#[test]
fn a_type_body_of_many_members_is_not_reported() {
	// A struct's fields are one item, not ten statements. Reporting them was the false positive that
	// made the rule noise on every data-heavy file.
	let mut source = String::from("struct Big {\n");

	push_numbered(&mut source, 10, |index| format!("    field_{index}: u32,"));

	source.push_str("}\n");

	let findings = groups_in(&source, Language::Rust);

	assert!(
		findings.is_empty(),
		"a struct's fields are one logical group: {findings:?}"
	);
}

#[test]
fn an_impl_block_of_many_methods_is_not_reported() {
	let mut source = String::from("impl Thing {\n");

	push_numbered(&mut source, 10, |index| {
		format!("    fn method_{index}(&self) {{}}")
	});

	source.push_str("}\n");

	let findings = groups_in(&source, Language::Rust);

	assert!(
		findings.is_empty(),
		"sibling methods are separate items: {findings:?}"
	);
}

#[test]
fn a_class_body_of_many_fields_is_not_reported() {
	let mut source = String::from("class Widget {\n");

	push_numbered(&mut source, 10, |index| {
		format!("  final field_{index} = {index};")
	});

	source.push_str("}\n");

	let findings = groups_in(&source, Language::Dart);

	assert!(
		findings.is_empty(),
		"a class body holds declarations rather than statements: {findings:?}"
	);
}

#[test]
fn a_struct_literal_of_many_fields_is_not_reported() {
	let mut source = String::from("fn build() -> Point {\n    let point = Point {\n");

	push_numbered(&mut source, 12, |index| {
		format!("        field_{index}: {index},")
	});

	source.push_str("    };\n    point\n}\n");

	let findings = groups_in(&source, Language::Rust);

	assert!(
		findings.is_empty(),
		"a literal's fields are data rather than statements: {findings:?}"
	);
}

// ---------------------------------------------------------------------------
// State does not leak between bodies
// ---------------------------------------------------------------------------

#[test]
fn a_closed_declaration_does_not_silence_the_statements_after_it() {
	// The old flag was never cleared, so one `fn` made the rest of the file invisible to the rule.
	let mut source = String::from("fn tiny() {}\n");
	source.push_str(&statements(9));

	let findings = groups_in(&source, Language::Rust);

	assert_eq!(
		findings.len(),
		1,
		"the run after a one-line declaration is still measured: {findings:?}"
	);
}

#[test]
fn a_declaration_nested_in_a_function_ends_with_its_own_body() {
	// A struct declared inside a function ends at its closing brace, and the statements after it belong
	// to the function again.
	let mut source = String::from("fn work() {\n    struct Inner {\n");

	push_numbered(&mut source, 10, |index| {
		format!("        field_{index}: u32,")
	});

	source.push_str("    }\n");

	push_numbered(&mut source, 9, |index| format!("    step_{index}();"));

	source.push_str("}\n");

	let findings = groups_in(&source, Language::Rust);

	assert_eq!(
		findings.len(),
		1,
		"the run after the nested struct is measured: {findings:?}"
	);
	assert!(
		findings[0].message.contains("9 statements"),
		"the nested struct's fields are not part of the run: {}",
		findings[0].message
	);
}

// ---------------------------------------------------------------------------
// Statement shapes
// ---------------------------------------------------------------------------

#[test]
fn a_formatted_multiline_call_counts_as_one_statement() {
	let source = "\
void emit() {
  useValue(
    first,
    second,
    third,
    fourth,
    fifth,
    sixth,
    seventh,
  );
}
";
	let findings = groups_in(source, Language::Dart);

	assert!(
		findings.is_empty(),
		"one multiline call is not a run of statements: {findings:?}"
	);
}

#[test]
fn a_formatted_multiline_collection_counts_as_one_statement() {
	let source = "\
final values = [
  first,
  second,
  third,
  fourth,
  fifth,
  sixth,
  seventh,
];
";
	let findings = groups_in(source, Language::Dart);

	assert!(
		findings.is_empty(),
		"one multiline collection is not a run of statements: {findings:?}"
	);
}

#[test]
fn a_formatted_method_chain_counts_as_one_statement() {
	let source = "\
final value = source
  .first()
  .second()
  .third()
  .fourth()
  .fifth()
  .sixth()
  .seventh();
";
	let findings = groups_in(source, Language::Dart);

	assert!(
		findings.is_empty(),
		"one method chain is not a run of statements: {findings:?}"
	);
}

#[test]
fn a_multiline_call_inside_a_function_body_counts_once() {
	let source = "\
fn work() {
    let value = build(
        first,
        second,
        third,
        fourth,
        fifth,
        sixth,
        seventh,
    );
    use_it(value);
}
";
	let findings = groups_in(source, Language::Rust);

	assert!(
		findings.is_empty(),
		"a wrapped call is one statement inside the body: {findings:?}"
	);
}

#[test]
fn a_balanced_one_line_block_does_not_corrupt_the_run() {
	// `if a > 0 { work(); }` opens and closes a block on its own line. It must not leave the tracker
	// inside a body, which would silence the statements that follow it.
	let mut source = String::from("fn work() {\n    let a = 1;\n    if a > 0 { work(); }\n");

	push_numbered(&mut source, 9, |index| format!("    step_{index}();"));

	source.push_str("}\n");

	let findings = groups_in(&source, Language::Rust);

	assert_eq!(
		findings.len(),
		1,
		"the one-liner splits the body into two runs, the longer of which is reported: {findings:?}"
	);
}

// ---------------------------------------------------------------------------
// Languages and configuration
// ---------------------------------------------------------------------------

#[test]
fn a_long_run_inside_a_python_function_is_reported() {
	let mut source = String::from("def work():\n");

	push_numbered(&mut source, 10, |index| format!("    step_{index}()"));

	let findings = groups_in(&source, Language::Python);

	assert_eq!(
		findings.len(),
		1,
		"an indentation-delimited body is measured like any other: {findings:?}"
	);
}

#[test]
fn a_long_run_inside_a_ruby_method_is_reported() {
	let mut source = String::from("def work\n");

	push_numbered(&mut source, 10, |index| format!("  step_{index}"));

	source.push_str("end\n");

	let findings = groups_in(&source, Language::Ruby);

	assert_eq!(
		findings.len(),
		1,
		"an end-terminated body is measured like any other: {findings:?}"
	);
	assert!(
		findings[0].message.contains("10 statements"),
		"the closing `end` is not counted as a statement: {}",
		findings[0].message
	);
}

#[test]
fn a_long_run_inside_a_lua_function_is_reported() {
	let mut source = String::from("function work()\n");

	push_numbered(&mut source, 10, |index| {
		format!("  local step_{index} = {index}")
	});

	source.push_str("end\n");

	let findings = groups_in(&source, Language::Lua);

	assert_eq!(
		findings.len(),
		1,
		"a Lua body is measured like any other: {findings:?}"
	);
}

#[test]
fn python_definitions_separated_by_the_pep_8_blank_lines_are_not_a_run() {
	let source = "def first():\n    pass\n\n\ndef second():\n    pass\n";
	let findings = groups_in(source, Language::Python);

	assert!(
		findings.is_empty(),
		"two definitions are two items, whatever the gap between them: {findings:?}"
	);
}

#[test]
fn a_raised_limit_lets_a_longer_run_pass() {
	let findings = groups_with_limit(&statements(12), Language::Dart, 20);

	assert!(
		findings.is_empty(),
		"twelve statements are within a limit of twenty: {findings:?}"
	);
}

#[test]
fn the_limit_is_configurable_from_toml() {
	// The field is part of the public configuration surface, so its kebab-case name is part of the
	// contract a project writes in `monostyle.toml`.
	let config: RulesConfig =
		toml::from_str("max-statements-per-group = 12").expect("the key must deserialize");

	assert_eq!(config.max_statements_per_group, 12);

	let findings = groups_with_limit(
		&statements(12),
		Language::Dart,
		config.max_statements_per_group,
	);

	assert!(
		findings.is_empty(),
		"the configured limit is what the rule measures against: {findings:?}"
	);
}

#[test]
fn disabling_the_rule_removes_every_finding() {
	let lexed = lex(&statements(20), Language::Dart);
	let config = RulesConfig {
		require_group_separation: false,
		..RulesConfig::default()
	};

	let findings = whitespace::group_separation(&lexed, &config);

	assert!(
		findings.is_empty(),
		"a disabled rule reports nothing: {findings:?}"
	);
}

#[test]
fn a_dart_switch_expression_of_many_arms_is_not_reported() {
	// A total function over an enum necessarily lists one arm per variant, and only one of them runs. The
	// arms are alternatives rather than steps, so counting them reported a nine-pattern switch as nine
	// statements run together — which is every one of them.
	let source = "\
SectionColour fromCode(int code) => switch (code) {
  0 => SectionColour.acid,
  1 => SectionColour.coral,
  2 => SectionColour.cyan,
  3 => SectionColour.paper,
  4 => SectionColour.violet,
  5 => SectionColour.amber,
  6 => SectionColour.blue,
  7 => SectionColour.pink,
  _ => throw StateError('unknown palette colour'),
};
";
	let findings = groups_in(source, Language::Dart);

	assert!(
		findings.is_empty(),
		"switch arms are alternatives, not a statement run: {findings:?}"
	);
}

#[test]
fn a_rust_match_of_many_arms_is_not_reported() {
	let source = "\
fn name(code: u8) -> &'static str {
    match code {
        0 => \"acid\",
        1 => \"coral\",
        2 => \"cyan\",
        3 => \"paper\",
        4 => \"violet\",
        5 => \"amber\",
        6 => \"blue\",
        7 => \"pink\",
        _ => \"unknown\",
    }
}
";
	let findings = groups_in(source, Language::Rust);

	assert!(
		findings.is_empty(),
		"match arms are alternatives, not a statement run: {findings:?}"
	);
}

#[test]
fn a_switch_with_statement_arms_is_still_not_a_run() {
	// The C-family shape: each arm is a block of statements. The arms are still alternatives, so the
	// bodies inside them are not one sequence.
	let source = "\
void dispatch(int code) {
  switch (code) {
    case 0: {
      first();
      second();
      break;
    }
    case 1: {
      third();
      fourth();
      break;
    }
    default: {
      fallback();
      break;
    }
  }
}
";
	let findings = groups_in(source, Language::C);

	assert!(
		findings.is_empty(),
		"switch arms are alternatives: {findings:?}"
	);
}

#[test]
fn a_long_run_in_a_loop_body_is_still_reported() {
	// The other half of the same distinction: a loop body runs to completion, so its statements are a
	// sequence and the rule still measures them.
	let mut source = String::from("fn render() {\n    for y in 0..10 {\n");

	push_numbered(&mut source, 9, |index| {
		format!("        let step_{index} = {index};")
	});

	source.push_str("    }\n}\n");

	let findings = groups_in(&source, Language::Rust);

	assert_eq!(
		findings.len(),
		1,
		"a loop body is a statement sequence: {findings:?}"
	);
}
