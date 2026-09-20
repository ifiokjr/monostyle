// Language snippets shared by the test suites that need them.
//
// One snippet per language, each written in that language's own syntax. The file is included by
// the lexer's language-profile tests and by the metrics crate's unit-detection test: the metrics
// crate cannot depend on another crate's test binary, and a copy of the table in each suite would
// let the two drift apart as languages are added.
//
// The header uses `//` rather than `//!` because the file is spliced in by `include!`, and inner
// doc comments are not allowed to arrive through a macro expansion.

use monostyle_core::Language;

/// A snippet exercising one language's comment, string, and decision syntax.
pub struct Snippet {
	/// The language.
	pub language: Language,
	/// Source written in that language.
	pub source: &'static str,
}

/// A snippet per language, each written in its own syntax.
pub const SNIPPETS: &[Snippet] = &[
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
