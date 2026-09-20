//! Tests that exercise every language profile through the scanner.
//!
//! The profile table is data, and the paths through it are per language: a comment token that only one
//! language uses, a string form unique to another. These tests run a snippet written in each language's
//! own syntax through the scanner, so every profile's comment and literal handling is exercised rather
//! than only the languages the fixtures happen to cover.

use monostyle_core::Language;
use monostyle_lexer::LineKind;
use monostyle_lexer::lex;

mod snippets {
	include!("fixtures/language_snippets.rs");
}

use snippets::SNIPPETS;

#[test]
fn every_language_detects_a_decision_point() {
	// A profile with no working decision keyword reports a complexity of one for every function, which
	// looks authoritative and is wrong.
	for snippet in SNIPPETS {
		let lexed = lex(snippet.source, snippet.language);

		assert!(
			lexed.total_decisions() > 0,
			"{}: no decision detected in {:?}",
			snippet.language,
			snippet.source
		);
	}
}

#[test]
fn every_language_hides_literal_contents_from_the_masked_view() {
	for snippet in SNIPPETS {
		let literal = match snippet.language {
			Language::Python => "    text = \"if for while\"\n",
			Language::Ruby => "  text = \"if for while\"\n",
			Language::Shell => "  text=\"if for while\"\n",
			Language::Nix => "  text = \"if for while\";\n",
			_ => "text = \"if for while\";\n",
		};

		let source = format!("{}{literal}", snippet.source);
		let lexed = lex(&source, snippet.language);

		let leaked = lexed
			.lines
			.iter()
			.any(|line| line.masked_code.contains("while"));

		assert!(
			!leaked,
			"{}: literal contents leaked into the masked view",
			snippet.language
		);
	}
}

#[test]
fn every_language_recognizes_its_own_comments() {
	for snippet in SNIPPETS {
		let comment = match snippet.language {
			Language::Python
			| Language::Ruby
			| Language::Shell
			| Language::Nix
			| Language::Elixir => "# a note\n",
			Language::Lua | Language::Haskell => "-- a note\n",
			_ => "// a note\n",
		};

		let source = format!("{comment}{}", snippet.source);
		let lexed = lex(&source, snippet.language);

		let first = lexed.lines.first().expect("a line");

		assert_eq!(
			first.kind,
			LineKind::Comment,
			"{}: `{comment}` was not recognized as a comment",
			snippet.language
		);
	}
}

#[test]
fn every_language_hides_comment_contents_from_the_masked_view() {
	for snippet in SNIPPETS {
		let comment = match snippet.language {
			Language::Python
			| Language::Ruby
			| Language::Shell
			| Language::Nix
			| Language::Elixir => "# if for while\n",
			Language::Lua | Language::Haskell => "-- if for while\n",
			_ => "// if for while\n",
		};

		let source = format!("{comment}{}", snippet.source);
		let lexed = lex(&source, snippet.language);
		let first = lexed.lines.first().expect("a line");

		assert!(
			!first.masked_code.contains("while"),
			"{}: comment contents leaked into the masked view",
			snippet.language
		);
	}
}

#[test]
fn every_language_scans_without_recovery() {
	// An unterminated construct means the scanner made a guess, so a clean scan is the baseline every
	// profile must meet on its own valid syntax.
	for snippet in SNIPPETS {
		let lexed = lex(snippet.source, snippet.language);

		assert!(
			lexed.is_clean(),
			"{}: the scan needed recovery on {:?}: {:?}",
			snippet.language,
			snippet.source,
			lexed.unterminated
		);
	}
}

#[test]
fn every_language_has_a_stable_line_count() {
	// A profile that swallows a newline reports fewer lines than the file has, which changes every
	// density denominator downstream.
	for snippet in SNIPPETS {
		let expected = snippet.source.lines().count();
		let lexed = lex(snippet.source, snippet.language);

		assert_eq!(
			lexed.lines.len(),
			expected,
			"{}: line count differs from the source",
			snippet.language
		);
	}
}

#[test]
fn multiline_constructs_span_lines_in_every_language_that_has_them() {
	/// A language with a multiline string or heredoc form, and source using it.
	const CASES: &[(Language, &str)] = &[
		(Language::Python, "text = '''\nif for while\n'''\n"),
		(Language::Rust, "let text = r#\"\nif for while\n\"#;\n"),
		(Language::Dart, "var text = '''\nif for while\n''';\n"),
		(Language::Go, "text := `\nif for while\n`\n"),
		(Language::Lua, "local text = [[\nif for while\n]]\n"),
		(Language::Nix, "text = ''\nif for while\n'';\n"),
		(Language::TypeScript, "const text = `\nif for while\n`;\n"),
		(Language::Shell, "cat <<EOF\nif for while\nEOF\n"),
	];

	for (language, source) in CASES {
		let lexed = lex(source, *language);

		assert!(
			lexed.is_clean(),
			"{language}: the multiline form was not closed"
		);
		assert_eq!(
			lexed.total_decisions(),
			0,
			"{language}: literal contents produced decisions"
		);
	}
}

#[test]
fn every_language_handles_an_empty_file() {
	for language in Language::ALL {
		let lexed = lex("", language);

		assert!(lexed.is_clean());
		assert_eq!(lexed.source_line_count(), 0);
	}
}

#[test]
fn every_language_handles_a_file_of_only_newlines() {
	for language in Language::ALL {
		let lexed = lex("\n\n\n", language);

		assert_eq!(lexed.source_line_count(), 0);
		assert!(lexed.lines.iter().all(monostyle_lexer::LexedLine::is_blank));
	}

	// The line count excludes the trailing newline, so three blank lines are three lines.
	assert_eq!(lex("\n\n\n", Language::Rust).lines.len(), 3);
}
