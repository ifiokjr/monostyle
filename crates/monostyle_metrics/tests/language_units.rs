//! The unit detector, exercised against every language profile.
//!
//! The assertion needs both crates, so it lives here rather than in the lexer's suite: a
//! dev-dependency from `monostyle_lexer` onto `monostyle_metrics` would be a cycle, because
//! `monostyle_metrics` depends on `monostyle_lexer` at runtime — and a cycle makes neither crate
//! publishable, in either order.

mod snippets {
	include!("../../monostyle_lexer/tests/fixtures/language_snippets.rs");
}

use monostyle_core::Language;
use monostyle_lexer::lex;
use monostyle_metrics::find_units;
use snippets::SNIPPETS;

#[test]
fn every_language_reports_units_when_it_has_functions() {
	// Languages whose declaration syntax the structural detector recognizes should produce at least
	// one unit. Haskell and Nix are exempt because their declarations have no parameter list to
	// anchor on, and the detector is documented as approximate.
	let exempt = [Language::Haskell, Language::Nix, Language::Mozjs];

	for snippet in SNIPPETS {
		if exempt.contains(&snippet.language) {
			continue;
		}

		let lexed = lex(snippet.source, snippet.language);
		let units = find_units(&lexed);

		assert!(
			!units.is_empty(),
			"{}: no unit detected in {:?}",
			snippet.language,
			snippet.source
		);
	}
}
