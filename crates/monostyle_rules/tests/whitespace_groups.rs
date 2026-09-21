//! Regression tests for separating runs of statements.

use monostyle_core::Language;
use monostyle_lexer::lex;
use monostyle_rules::RulesConfig;
use monostyle_rules::whitespace;

/// Runs the group-separation rule over a source snippet.
fn groups_in(source: &str, language: Language) -> Vec<monostyle_core::Finding> {
	let lexed = lex(source, language);

	whitespace::group_separation(&lexed, &RulesConfig::default())
}

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
fn eight_consecutive_statements_are_still_reported() {
	let source = "\
first();
second();
third();
fourth();
fifth();
sixth();
seventh();
eighth();
";
	let findings = groups_in(source, Language::Dart);

	assert_eq!(
		findings.len(),
		1,
		"the real eight-statement run should remain"
	);
	assert!(findings[0].message.contains("8 statements"));
}
