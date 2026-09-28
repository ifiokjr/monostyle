//! Regression tests for spacing before returns.

use monostyle_core::Language;
use monostyle_lexer::lex;
use monostyle_rules::RulesConfig;
use monostyle_rules::whitespace;

/// Runs the return-spacing rule over a Dart snippet.
fn returns_in(source: &str) -> Vec<monostyle_core::Finding> {
	let lexed = lex(source, Language::Dart);

	whitespace::blank_line_before_return(&lexed, &RulesConfig::default())
}

#[test]
fn a_return_after_work_without_separation_is_reported() {
	let findings = returns_in("int answer() {\n  final value = 42;\n  return value;\n}\n");

	assert_eq!(findings.len(), 1, "the crowded return should be reported");
}

#[test]
fn a_blank_line_before_a_return_satisfies_the_rule() {
	let findings = returns_in("int answer() {\n  final value = 42;\n\n  return value;\n}\n");

	assert!(
		findings.is_empty(),
		"the blank line already separates the return: {findings:?}"
	);
}

#[test]
fn a_return_under_an_attribute_keeps_the_attribute_attached() {
	// `#[cfg(test)]\nreturn …` — the attribute configures the return, so the blank
	// the rule asks for belongs above the attribute. rustfmt rejects a break
	// between an outer attribute and its item (clippy: empty_line_after_outer_attr).
	let findings = returns_in(
		"fn command(&self) -> Command {\n    #[cfg(test)]\n    return Command::new(&self.path);\n\n    #[cfg(not(test))]\n    return self.build();\n}\n",
	);

	assert!(
		findings.is_empty(),
		"a return directly under its attribute needs no inserted blank: {findings:?}"
	);
}
