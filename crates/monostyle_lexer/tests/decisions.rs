//! Tests for decision-keyword extraction.
//!
//! A keyword counted in the wrong position invents complexity that is not there and sends the
//! blank-line rules hunting for statements that do not exist. Both failure modes were found in real
//! repositories: `impl From<X> for Y` padded a blank line between an attribute and its item, and
//! `Address::default()` padded one inside a boolean chain.

use monostyle_core::Language;
use monostyle_lexer::lex;

/// Returns the decisions found on each line of `source`.
fn decisions(source: &str, language: Language) -> Vec<Vec<String>> {
	lex(source, language)
		.lines
		.iter()
		.map(|line| line.decisions.clone())
		.collect()
}

#[test]
fn a_for_in_impl_position_is_not_a_loop_decision() {
	// `for` names the trait being implemented, not a loop. Counting it made the line read as a
	// control-flow statement, which is what detached `#[cfg]` from its `impl` downstream.
	let lines = decisions(
		"impl From<PinaPodError> for solana_program_error::ProgramError {\n    fn from(e: PinaPodError) -> Self {\n        todo!()\n    }\n}\n",
		Language::Rust,
	);

	assert!(
		lines[0].is_empty(),
		"`for` in an impl declaration is not a decision: {:?}",
		lines[0]
	);
}

#[test]
fn a_for_loop_still_counts() {
	let lines = decisions("for item in items {\n    use(item);\n}\n", Language::Rust);

	assert_eq!(lines[0], vec!["for"]);
}

#[test]
fn a_higher_ranked_trait_bound_is_not_a_loop_decision() {
	// `for<'a>` in a where clause is a binder, not a loop, and carries no `in`.
	let lines = decisions("where T: for<'a> Fn(&'a u8) -> u8\n", Language::Rust);

	assert!(
		lines[0].is_empty(),
		"a trait-bound `for` is not a decision: {:?}",
		lines[0]
	);
}

#[test]
fn a_default_trait_call_is_not_a_branch_decision_in_rust() {
	// Rust has no `default` arm — the wildcard is `_` — so the only bare `default` in Rust is a
	// `Default` implementation call like `ServerConfig::default()`.
	let lines = decisions(
		"let config = ServerConfig::default();\nlet address = Address::default();\n",
		Language::Rust,
	);

	assert!(
		lines[0].is_empty(),
		"`::default()` is a call, not a branch: {:?}",
		lines[0]
	);
	assert!(
		lines[1].is_empty(),
		"`::default()` is a call, not a branch: {:?}",
		lines[1]
	);
}

#[test]
fn a_switch_default_arm_still_counts_in_typescript() {
	let lines = decisions(
		"switch (kind) {\n    case 1:\n        break;\n    default:\n        break;\n}\n",
		Language::TypeScript,
	);

	assert!(
		lines
			.iter()
			.any(|line| line.iter().any(|decision| decision == "default")),
		"a real `default:` arm must still count"
	);
}

#[test]
fn a_default_label_is_not_a_branch_in_rust_even_when_called_default() {
	// The word `default` inside a longer expression — `unwrap_or_default()` — is part of a name.
	let lines = decisions("let value = option.unwrap_or_default();\n", Language::Rust);

	assert!(
		lines[0].is_empty(),
		"unwrap_or_default is one identifier: {:?}",
		lines[0]
	);
}
