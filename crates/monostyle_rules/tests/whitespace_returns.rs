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
	// The work above is a call rather than a binding, so the return is a
	// separate thought from it and the gap is still asked for.
	let findings = returns_in("int answer() {\n  prepare();\n  return compute();\n}\n");

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

#[test]
fn a_single_line_binding_and_the_return_that_uses_it_are_one_thought() {
	// The reviewer's two-line rule: when a single-line binding is followed by a
	// single-line return that uses it, the pair reads as one thought and the
	// blank between them is superfluous.
	let findings = returns_in(
		"bool hasBinary(String dir) {\n  final binDir = join(dir, \"bin\");\n  return binDir.existsSync;\n}\n",
	);

	assert!(
		findings.is_empty(),
		"a binding and its return need no gap: {findings:?}"
	);
}

#[test]
fn the_pair_rule_holds_across_binding_keywords_and_uses() {
	// `let`, `const`, `var`, `final`, and `val` all bind; the return may compute
	// with the binding rather than name it alone.
	let source =
		"Number pick(int count) {\n  const base = load();\n  return base * count + 1;\n}\n";
	let findings = returns_in(source);

	assert!(
		findings.is_empty(),
		"compute-with-the-binding counts: {findings:?}"
	);
}

#[test]
fn a_multi_line_return_keeps_its_gap() {
	// "If the return expression is multiple lines, then keep a gap."
	let findings = returns_in(
		"List<int> entries() {\n  final entries = readdirSync(binDir);\n  return entries\n      .where((entry) => entry.startsWith(\"mdt\"))\n      .toList();\n}\n",
	);

	assert!(
		!findings.is_empty(),
		"a multi-line return still asks for separation: {findings:?}"
	);
}

#[test]
fn a_return_unrelated_to_the_binding_keeps_its_gap() {
	// Without the reference the two lines are separate thoughts, so the gap
	// stays: the rule exempts pairs, not neighbours.
	let findings =
		returns_in("bool probe() {\n  const config = load();\n  return otherThing;\n}\n");

	assert!(
		!findings.is_empty(),
		"an unrelated return keeps the gap: {findings:?}"
	);
}
