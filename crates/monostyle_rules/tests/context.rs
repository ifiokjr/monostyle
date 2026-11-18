//! Tests for the rule context, which shares one file's unit measurements between the rules.

use monostyle_core::Language;
use monostyle_lexer::lex;
use monostyle_rules::RuleContext;
use monostyle_rules::RulesConfig;
use monostyle_rules::run_rules;

/// Three functions, so the unit set has something to measure.
const SOURCE: &str =
	"fn a() {\n    1\n}\n\nfn b(x: u8) -> u8 {\n    if x > 1 { x } else { 0 }\n}\n\nfn c() {}\n";

#[test]
fn units_are_measured_once_and_shared() {
	// The whole point of the context: seven rules ask for the units, and the set they get is the
	// same one, not a fresh measurement each time. Pointer identity is the proof that nothing was
	// rebuilt between the two calls.
	let lexed = lex(SOURCE, Language::Rust);
	let config = RulesConfig::default();
	let context = RuleContext::new(&lexed, &config);

	let first = context.units();
	let second = context.units();

	assert!(
		std::ptr::eq(first, second),
		"the second request for units should return the set the first one built"
	);
	assert_eq!(first.len(), 3, "the three functions should each be a unit");
	assert!(!first.is_empty());
}

#[test]
fn the_context_hands_back_what_it_was_given() {
	let lexed = lex(SOURCE, Language::Rust);
	let config = RulesConfig {
		max_cyclomatic_per_unit: 1,

		..RulesConfig::default()
	};
	let context = RuleContext::new(&lexed, &config);

	assert!(std::ptr::eq(context.file(), &raw const lexed));
	assert!(std::ptr::eq(context.config(), &raw const config));
	assert_eq!(context.config().max_cyclomatic_per_unit, 1);
}

#[test]
fn running_the_rules_through_the_context_sees_the_configuration() {
	// A limit of one makes `b`, with its single branch, too complex, so the configuration the
	// context carries is what the unit rules measured against.
	let lexed = lex(SOURCE, Language::Rust);
	let strict = RulesConfig {
		max_cyclomatic_per_unit: 1,

		..RulesConfig::default()
	};
	let relaxed = RulesConfig::default();

	let strict_findings = run_rules(&RuleContext::new(&lexed, &strict));
	let relaxed_findings = run_rules(&RuleContext::new(&lexed, &relaxed));

	assert!(
		strict_findings
			.iter()
			.any(|finding| finding.rule == "complexity/cyclomatic-per-unit"),
		"a limit of one should report the branching function"
	);
	assert!(
		relaxed_findings
			.iter()
			.all(|finding| finding.rule != "complexity/cyclomatic-per-unit"),
		"the default limit should not report it"
	);
}
