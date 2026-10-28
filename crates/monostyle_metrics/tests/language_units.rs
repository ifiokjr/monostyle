//! The unit detector, exercised against every language profile.
//!
//! The assertion needs both crates, so it lives here rather than in the lexer's suite: a
//! dev-dependency from `monostyle_lexer` onto `monostyle_metrics` would be a cycle, because
//! `monostyle_metrics` depends on `monostyle_lexer` at runtime — and a cycle makes neither crate
//! publishable, in either order.

mod snippets {
	include!("../../monostyle_lexer/tests/fixtures/language_snippets.rs");
}

mod shell {
	include!("../../monostyle_lexer/tests/fixtures/shell_here_strings.rs");
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
	let exempt = [
		Language::Haskell,
		Language::Nix,
		Language::Mozjs,
		Language::Yaml,
		Language::Properties,
	];

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

#[test]
fn shell_snapshot_and_identity_functions_end_at_their_own_closing_braces() {
	let lexed = lex(shell::SNAPSHOT_AND_IDENTITY, Language::Shell);
	let units = find_units(&lexed);
	let spans: Vec<_> = units
		.iter()
		.map(|unit| {
			(
				unit.name.as_str(),
				unit.start_line,
				unit.end_line,
				unit.depth,
			)
		})
		.collect();

	assert_eq!(
		spans,
		[
			("owned_snapshot", 1, 4, 0),
			("owned_identity", 6, 17, 0),
			("owned_next", 19, 21, 0),
		]
	);
}

#[test]
fn braced_shell_functions_without_control_flow_have_separate_spans() {
	let source = "first() {\n  echo first\n}\n\nfunction second {\n  echo second\n}\n";
	let lexed = lex(source, Language::Shell);
	let units = find_units(&lexed);
	let spans: Vec<_> = units
		.iter()
		.map(|unit| (unit.name.as_str(), unit.start_line, unit.end_line))
		.collect();

	assert_eq!(spans, [("first", 1, 3), ("second", 5, 7)]);
}

#[test]
fn nested_shell_blocks_do_not_close_their_enclosing_function() {
	let source = r#"collect() {
  while read -r row; do
    if [ -n "$row" ]; then
      case "$row" in
        first) echo yes ;;
        *) echo no ;;
      esac
    fi
  done <<< "$rows"
}

next() {
  echo done
}
"#;
	let lexed = lex(source, Language::Shell);
	let units = find_units(&lexed);
	let spans: Vec<_> = units
		.iter()
		.map(|unit| (unit.name.as_str(), unit.start_line, unit.end_line))
		.collect();

	assert_eq!(spans, [("collect", 1, 10), ("next", 12, 14)]);
	assert_eq!(lexed.total_decisions(), 3);
	assert_eq!(lexed.lines[1].nesting, ["while"]);
	assert_eq!(lexed.lines[2].nesting, ["if"]);
	assert_eq!(lexed.lines[3].nesting, ["case"]);
}
