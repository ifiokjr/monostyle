//! Tests that exercise every language profile through the scanner.
//!
//! The profile table is data, and the paths through it are per language: a comment token that only one
//! language uses, a string form unique to another. These tests run a snippet written in each language's
//! own syntax through the scanner, so every profile's comment and literal handling is exercised rather
//! than only the languages the fixtures happen to cover.

use monostyle_core::Language;
use monostyle_lexer::LineKind;
use monostyle_lexer::lex;

/// A snippet exercising one language's comment, string, and decision syntax.
struct Snippet {
	/// The language.
	language: Language,
	/// Source written in that language.
	source: &'static str,
}

/// A snippet per language, each written in its own syntax.
const SNIPPETS: &[Snippet] = &[
	Snippet {
		language: Language::C,
		source: "int f(int a) { if (a) { return 1; } return 0; }\n",
	},
	Snippet {
		language: Language::Cpp,
		source: "auto f(int a) { if (a) { return 1; } return 0; }\n",
	},
	Snippet {
		language: Language::CSharp,
		source: "int F(int a) { if (a > 0) { return 1; } return 0; }\n",
	},
	Snippet {
		language: Language::Java,
		source: "int f(int a) { if (a > 0) { return 1; } return 0; }\n",
	},
	Snippet {
		language: Language::JavaScript,
		source: "function f(a) { if (a) { return 1; } return 0; }\n",
	},
	Snippet {
		language: Language::Kotlin,
		source: "fun f(a: Int): Int { if (a > 0) return 1; return 0 }\n",
	},
	Snippet {
		language: Language::Mozjs,
		source: "function f(a) { if (a) { return 1; } return 0; }\n",
	},
	Snippet {
		language: Language::Python,
		source: "def f(a):\n    if a:\n        return 1\n    return 0\n",
	},
	Snippet {
		language: Language::Rust,
		source: "fn f(a: i32) -> i32 { if a > 0 { 1 } else { 0 } }\n",
	},
	Snippet {
		language: Language::TypeScript,
		source: "function f(a: number): number { if (a > 0) { return 1; } return 0; }\n",
	},
	Snippet {
		language: Language::Tsx,
		source: "function f(a: number): number { if (a > 0) { return 1; } return 0; }\n",
	},
	Snippet {
		language: Language::Dart,
		source: "int f(int a) { if (a > 0) { return 1; } return 0; }\n",
	},
	Snippet {
		language: Language::Go,
		source: "func f(a int) int { if a > 0 { return 1 }; return 0 }\n",
	},
	Snippet {
		language: Language::Swift,
		source: "func f(a: Int) -> Int { if a > 0 { return 1 }; return 0 }\n",
	},
	Snippet {
		language: Language::Ruby,
		source: "def f(a)\n  if a > 0\n    return 1\n  end\n  0\nend\n",
	},
	Snippet {
		language: Language::Php,
		source: "<?php function f($a) { if ($a > 0) { return 1; } return 0; }\n",
	},
	Snippet {
		language: Language::Scala,
		source: "def f(a: Int): Int = { if (a > 0) 1 else 0 }\n",
	},
	Snippet {
		language: Language::Shell,
		source: "f() {\n  if [ \"$1\" ]; then\n    echo yes\n  fi\n}\n",
	},
	Snippet {
		language: Language::Lua,
		source: "function f(a)\n  if a then\n    return 1\n  end\n  return 0\nend\n",
	},
	Snippet {
		language: Language::Elixir,
		source: "def f(a) do\n  if a do\n    1\n  else\n    0\n  end\nend\n",
	},
	Snippet {
		language: Language::Haskell,
		source: "f a = if a > 0 then 1 else 0\n",
	},
	Snippet {
		language: Language::Nix,
		source: "{ a }: if a then 1 else 0\n",
	},
];

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
fn every_language_reports_units_when_it_has_functions() {
	// Languages whose declaration syntax the structural detector recognizes should produce at least one
	// unit. Haskell and Nix are exempt because their declarations have no parameter list to anchor on,
	// and the detector is documented as approximate.
	let exempt = [Language::Haskell, Language::Nix, Language::Mozjs];

	for snippet in SNIPPETS {
		if exempt.contains(&snippet.language) {
			continue;
		}

		let lexed = lex(snippet.source, snippet.language);
		let units = monostyle_metrics::find_units(&lexed);

		assert!(
			!units.is_empty(),
			"{}: no unit detected in {:?}",
			snippet.language,
			snippet.source
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
