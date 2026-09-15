//! The language profile table.
//!
//! Every list a profile points at is a module-level `const`, which is what lets profiles be
//! `'static`. Naming them also makes each language's syntax table individually readable and
//! testable, and lets language families share one table rather than repeating it — a fix to
//! [`C_DECISIONS`] reaches every language that spreads the C-family base.
//!
//! String rules are declared longest-opener-first by convention even though
//! [`LanguageProfile::string_at`] sorts defensively, because reading `'''` before `'` makes
//! the intent obvious.

use monostyle_core::Language;

use crate::profile::BlockStyle;
use crate::profile::HeredocSyntax;
use crate::profile::Interpolation;
use crate::profile::LanguageProfile;
use crate::profile::StringRule;

// ---------------------------------------------------------------------------
// Shared tables
// ---------------------------------------------------------------------------

/// Line comments shared by the C family.
const C_LINE_COMMENTS: &[&str] = &["//"];

/// Block comments shared by the C family.
const C_BLOCK_COMMENTS: &[(&str, &str)] = &[("/*", "*/")];

/// Quoted delimiters shared by the C family.
const C_STRINGS: &[StringRule] = &[
	StringRule::escaped("\"", "\""),
	StringRule::escaped("'", "'"),
];

/// Prefixes that may precede a C-family string delimiter.
const C_STRING_PREFIXES: &[char] = &['L', 'u', 'U', 'R', 'B'];

/// Decision points shared by the C family.
const C_DECISIONS: &[&str] = &[
	"if", "else if", "for", "while", "case", "when", "catch", "except", "rescue", "default",
];

/// Block-opening keywords shared by the C family.
const C_NESTING: &[&str] = &[
	"if", "else if", "else", "for", "while", "switch", "match", "case", "when", "catch", "except",
	"rescue", "loop", "try",
];

/// Short-circuiting operators shared by the C family.
const C_LOGICAL: &[&str] = &["&&", "||"];

/// The `goto` jump statement.
const GOTO: &[&str] = &["goto"];

/// An empty keyword table, for languages that have none.
const NONE: &[&str] = &[];

/// An empty string-rule table.
const NO_STRINGS: &[StringRule] = &[];

/// An empty block-comment table.
const NO_BLOCK_COMMENTS: &[(&str, &str)] = &[];

/// An empty character table.
const NO_CHARS: &[char] = &[];

/// Nullish and optional-chaining operators, each of which adds a path.
const NULLISH: &[&str] = &["??", "?.", "??="];

// ---------------------------------------------------------------------------
// Per-language tables
// ---------------------------------------------------------------------------

/// Rust string rules.
///
/// Hashed raw literals are absent because they are resolved from the hash count at open
/// time rather than from a fixed delimiter pair.
const RUST_STRINGS: &[StringRule] = &[
	StringRule::escaped("\"", "\""),
	StringRule::escaped("'", "'"),
];

/// Rust string prefixes.
const RUST_STRING_PREFIXES: &[char] = &['r', 'b'];

/// Rust decision points, which add `loop` and `match` arms.
const RUST_DECISIONS: &[&str] = &["if", "else if", "for", "while", "loop", "match", "default"];

/// Rust nesting keywords.
const RUST_NESTING: &[&str] = &["if", "else if", "else", "for", "while", "loop", "match"];

/// TypeScript and TSX string rules, including template literals.
const TS_STRINGS: &[StringRule] = &[
	StringRule::multiline_interpolated("`", "`"),
	StringRule::escaped("\"", "\""),
	StringRule::escaped("'", "'"),
];

/// TypeScript decision points.
const TS_DECISIONS: &[&str] = &["if", "else if", "for", "while", "case", "catch", "default"];

/// C# string rules, including verbatim and raw strings.
const CSHARP_STRINGS: &[StringRule] = &[
	StringRule::multiline_raw("\"\"\"", "\"\"\""),
	StringRule::multiline_raw("@\"", "\""),
	StringRule::escaped("\"", "\""),
	StringRule::escaped("'", "'"),
];

/// C# string prefixes.
const CSHARP_STRING_PREFIXES: &[char] = &['@', '$'];

/// C# decision points, which add `foreach` and pattern `when` clauses.
const CSHARP_DECISIONS: &[&str] = &[
	"if", "else if", "for", "foreach", "while", "case", "catch", "when", "default",
];

/// Kotlin string rules.
const KOTLIN_STRINGS: &[StringRule] = &[
	StringRule::multiline_interpolated("\"\"\"", "\"\"\""),
	StringRule::interpolated("\"", "\""),
	StringRule::escaped("'", "'"),
];

/// Kotlin decision points.
const KOTLIN_DECISIONS: &[&str] = &["if", "else if", "for", "while", "when", "catch", "default"];

/// Kotlin nesting keywords.
const KOTLIN_NESTING: &[&str] = &[
	"if", "else if", "else", "for", "while", "when", "catch", "try",
];

/// Kotlin null-coalescing and safe-call operators.
const KOTLIN_NULLISH: &[&str] = &["?:", "?."];

/// Dart string rules.
///
/// Triple-quoted forms must be tried before single-quoted ones, and every form interpolates
/// because `$` appears inside all of them.
const DART_STRINGS: &[StringRule] = &[
	StringRule::multiline_interpolated("'''", "'''"),
	StringRule::multiline_interpolated("\"\"\"", "\"\"\""),
	StringRule::interpolated("'", "'"),
	StringRule::interpolated("\"", "\""),
];

/// Dart's raw-string prefix.
const DART_STRING_PREFIXES: &[char] = &['r'];

/// Dart decision points, which add `do`, `on`, and `switch` arms.
const DART_DECISIONS: &[&str] = &[
	"if", "else if", "for", "while", "do", "switch", "case", "catch", "on", "default",
];

/// Dart nesting keywords.
const DART_NESTING: &[&str] = &[
	"if", "else if", "else", "for", "while", "do", "switch", "case", "catch", "on", "try",
];

/// Swift string rules.
const SWIFT_STRINGS: &[StringRule] = &[
	StringRule::multiline_interpolated("\"\"\"", "\"\"\""),
	StringRule::interpolated("\"", "\""),
];

/// Swift decision points, which add `guard` and `repeat`.
const SWIFT_DECISIONS: &[&str] = &[
	"if", "else if", "guard", "for", "while", "repeat", "case", "catch", "default",
];

/// Swift nesting keywords.
const SWIFT_NESTING: &[&str] = &[
	"if", "else if", "else", "guard", "for", "while", "repeat", "switch", "case", "catch", "do",
];

/// Swift null-coalescing and optional-chaining operators.
const SWIFT_NULLISH: &[&str] = &["??", "?."];

/// Go string rules, including backtick raw strings.
const GO_STRINGS: &[StringRule] = &[
	StringRule::multiline_raw("`", "`"),
	StringRule::escaped("\"", "\""),
	StringRule::escaped("'", "'"),
];

/// Go decision points, which add `select`.
const GO_DECISIONS: &[&str] = &["if", "else if", "for", "case", "select", "default"];

/// Go nesting keywords.
const GO_NESTING: &[&str] = &["if", "else if", "else", "for", "switch", "case", "select"];

/// PHP line comments, which also include `#`.
const PHP_LINE_COMMENTS: &[&str] = &["//", "#"];

/// PHP string rules.
const PHP_STRINGS: &[StringRule] = &[
	StringRule::multiline_interpolated("\"", "\""),
	StringRule::escaped("'", "'"),
];

/// PHP decision points.
const PHP_DECISIONS: &[&str] = &[
	"if", "elseif", "else if", "for", "foreach", "while", "case", "catch", "default",
];

/// PHP nesting keywords.
const PHP_NESTING: &[&str] = &[
	"if", "elseif", "else if", "else", "for", "foreach", "while", "switch", "case", "catch", "try",
];

/// PHP null-coalescing and null-safe operators.
const PHP_NULLISH: &[&str] = &["??", "?->"];

/// PHP heredocs, which start with `<<<`.
const PHP_HEREDOC: HeredocSyntax = HeredocSyntax {
	marker: "<<<",
	allows_dash: false,
	allows_tilde: false,
};

/// Scala string rules.
const SCALA_STRINGS: &[StringRule] = &[
	StringRule::multiline_interpolated("\"\"\"", "\"\"\""),
	StringRule::interpolated("\"", "\""),
	StringRule::escaped("'", "'"),
];

/// Scala decision points.
const SCALA_DECISIONS: &[&str] = &["if", "else if", "for", "while", "case", "catch", "default"];

/// Scala nesting keywords.
const SCALA_NESTING: &[&str] = &[
	"if", "else if", "else", "for", "while", "match", "case", "catch", "try",
];

/// Python line comments.
const PYTHON_LINE_COMMENTS: &[&str] = &["#"];

/// Python string rules, including f-strings and triple-quoted forms.
const PYTHON_STRINGS: &[StringRule] = &[
	StringRule::multiline_interpolated("\"\"\"", "\"\"\""),
	StringRule::multiline_interpolated("'''", "'''"),
	StringRule::interpolated("\"", "\""),
	StringRule::interpolated("'", "'"),
];

/// Python string prefixes.
const PYTHON_STRING_PREFIXES: &[char] = &['r', 'b', 'f', 'u', 'R', 'B', 'F', 'U'];

/// Python decision points.
const PYTHON_DECISIONS: &[&str] = &["if", "elif", "for", "while", "except", "case", "match"];

/// Python nesting keywords, which include `with`.
const PYTHON_NESTING: &[&str] = &[
	"if", "elif", "else", "for", "while", "except", "with", "try", "match", "case",
];

/// Python word-form boolean operators, which each add a path.
const PYTHON_LOGICAL: &[&str] = &["and", "or"];

/// Ruby line comments.
const RUBY_LINE_COMMENTS: &[&str] = &["#"];

/// Ruby block comments.
const RUBY_BLOCK_COMMENTS: &[(&str, &str)] = &[("=begin", "=end")];

/// Ruby string rules.
const RUBY_STRINGS: &[StringRule] = &[
	StringRule::multiline_interpolated("\"", "\""),
	StringRule::escaped("'", "'"),
	StringRule::multiline_raw("`", "`"),
];

/// Ruby decision points, which add `unless`, `until`, and `rescue`.
const RUBY_DECISIONS: &[&str] = &[
	"if", "elsif", "unless", "for", "while", "until", "when", "rescue", "case",
];

/// Ruby nesting keywords.
const RUBY_NESTING: &[&str] = &[
	"if", "elsif", "else", "unless", "for", "while", "until", "case", "when", "rescue", "begin",
	"do",
];

/// Ruby boolean operators in both symbolic and word form.
const RUBY_LOGICAL: &[&str] = &["&&", "||", " and ", " or "];

/// Ruby heredocs, which also accept `<<-` and `<<~`.
const RUBY_HEREDOC: HeredocSyntax = HeredocSyntax {
	marker: "<<",
	allows_dash: true,
	allows_tilde: true,
};

/// The keyword that closes most Ruby blocks.
const RUBY_END: &[&str] = &["end"];

/// Lua line comments.
const LUA_LINE_COMMENTS: &[&str] = &["--"];

/// Lua block comments.
const LUA_BLOCK_COMMENTS: &[(&str, &str)] = &[("--[[", "]]")];

/// Lua string rules, including long strings.
const LUA_STRINGS: &[StringRule] = &[
	StringRule::multiline_raw("[[", "]]"),
	StringRule::escaped("\"", "\""),
	StringRule::escaped("'", "'"),
];

/// Lua decision points.
const LUA_DECISIONS: &[&str] = &["if", "elseif", "for", "while", "repeat"];

/// Lua nesting keywords.
const LUA_NESTING: &[&str] = &[
	"if", "elseif", "else", "for", "while", "repeat", "do", "function",
];

/// Lua word-form boolean operators.
const LUA_LOGICAL: &[&str] = &[" and ", " or "];

/// The keywords that close Lua blocks.
const LUA_END: &[&str] = &["end", "until"];

/// Elixir line comments.
const ELIXIR_LINE_COMMENTS: &[&str] = &["#"];

/// Elixir string rules.
const ELIXIR_STRINGS: &[StringRule] = &[
	StringRule::multiline_interpolated("\"\"\"", "\"\"\""),
	StringRule::interpolated("\"", "\""),
];

/// Elixir decision points.
const ELIXIR_DECISIONS: &[&str] = &["if", "unless", "case", "cond", "for", "rescue", "else"];

/// Elixir nesting keywords.
const ELIXIR_NESTING: &[&str] = &[
	"if", "unless", "case", "cond", "for", "try", "rescue", "else",
];

/// Elixir boolean operators in both symbolic and word form.
const ELIXIR_LOGICAL: &[&str] = &["and", "or", "&&", "||"];

/// The keyword that closes Elixir blocks.
const ELIXIR_END: &[&str] = &["end"];

/// Haskell line comments.
const HASKELL_LINE_COMMENTS: &[&str] = &["--"];

/// Haskell block comments, which nest.
const HASKELL_BLOCK_COMMENTS: &[(&str, &str)] = &[("{-", "-}")];

/// Haskell string rules.
const HASKELL_STRINGS: &[StringRule] = &[StringRule::escaped("\"", "\"")];

/// Haskell decision points.
const HASKELL_DECISIONS: &[&str] = &["if", "case", "where"];

/// Haskell nesting keywords.
const HASKELL_NESTING: &[&str] = &["if", "case", "where", "let", "do"];

/// Nix line comments.
const NIX_LINE_COMMENTS: &[&str] = &["#"];

/// Nix block comments.
const NIX_BLOCK_COMMENTS: &[(&str, &str)] = &[("/*", "*/")];

/// Escape sequences that are not closers inside a Nix indented string.
///
/// Nix is why [`StringRule::extra_escapes`] exists at all: `''$`, `'''`, and `''\` are
/// escapes, so a bare `''` only sometimes terminates the literal. Without these the scanner
/// ends a Nix string early and reads the remainder as code — which matters especially in a
/// repository whose Nix files are the main build surface.
const NIX_INDENTED_ESCAPES: &[&str] = &["''$", "'''", "''\\"];

/// Escape sequences inside a Nix double-quoted string.
const NIX_QUOTED_ESCAPES: &[&str] = &["\\\"", "\\$"];

/// Nix string rules.
const NIX_STRINGS: &[StringRule] = &[
	StringRule::multiline_interpolated("''", "''").with_extra_escapes(NIX_INDENTED_ESCAPES),
	StringRule::multiline_interpolated("\"", "\"").with_extra_escapes(NIX_QUOTED_ESCAPES),
];

/// Nix decision points.
const NIX_DECISIONS: &[&str] = &["if", "assert"];

/// Nix nesting keywords.
const NIX_NESTING: &[&str] = &["if", "let", "with", "assert"];

/// Shell line comments.
const SHELL_LINE_COMMENTS: &[&str] = &["#"];

/// Shell string rules.
const SHELL_STRINGS: &[StringRule] = &[
	StringRule::multiline_interpolated("\"", "\""),
	StringRule::escaped("'", "'"),
	StringRule::escaped("`", "`"),
];

/// Shell decision points.
const SHELL_DECISIONS: &[&str] = &["if", "elif", "for", "while", "until", "case"];

/// Shell nesting keywords.
const SHELL_NESTING: &[&str] = &["if", "elif", "else", "for", "while", "until", "case"];

/// Shell heredocs, which accept `<<-` for tab stripping.
const SHELL_HEREDOC: HeredocSyntax = HeredocSyntax {
	marker: "<<",
	allows_dash: true,
	allows_tilde: false,
};

/// The keywords that close Shell blocks.
const SHELL_END: &[&str] = &["fi", "done", "esac"];

// ---------------------------------------------------------------------------
// Profiles
// ---------------------------------------------------------------------------

/// The base profile every C-family language specializes.
fn c_family(language: Language) -> LanguageProfile {
	LanguageProfile {
		language,
		line_comments: C_LINE_COMMENTS,
		block_comments: C_BLOCK_COMMENTS,
		nestable_comments: false,
		strings: C_STRINGS,
		string_prefixes: C_STRING_PREFIXES,
		hashed_raw_strings: false,
		raw_string_prefix: None,
		interpolation: None,
		heredoc: None,
		regex_literals: false,
		decision_keywords: C_DECISIONS,
		nesting_keywords: C_NESTING,
		logical_operators: C_LOGICAL,
		jump_keywords: GOTO,
		ternary_operator: Some('?'),
		null_coalescing: NONE,
		block_style: BlockStyle::Brace,
		end_keywords: NONE,
		parameter_list_start: Some('('),
		variables_use_sigil: false,
	}
}

/// Returns the profile for `language`.
///
/// Unknown languages fall back to the C-family base, the most permissive superset, so
/// unfamiliar source is still analyzed rather than refused.
#[must_use]
pub fn profile_for(language: Language) -> LanguageProfile {
	match language {
		Language::Rust => rust(language),
		Language::Python => python(language),
		Language::Dart => dart(language),
		Language::Go => go(language),
		Language::Swift => swift(language),
		Language::Ruby => ruby(language),
		Language::Php => php(language),
		Language::Scala => scala(language),
		Language::Shell => shell(language),
		Language::Lua => lua(language),
		Language::Elixir => elixir(language),
		Language::Haskell => haskell(language),
		Language::Nix => nix(language),
		Language::TypeScript | Language::Tsx => typescript(language),
		Language::JavaScript | Language::Mozjs => javascript(language),
		Language::CSharp => csharp(language),
		Language::Kotlin => kotlin(language),
		Language::Markdown => markdown(language),
		Language::C | Language::Cpp | Language::Java => c_family(language),
	}
}

/// Rust adds `match` arms, `loop`, nested block comments, and hashed raw strings.
///
/// The nested-comment flag is load-bearing: `/* /* */ */` closes at the *second* `*/`, and
/// a scanner that closes at the first silently reads the following lines as code.
fn rust(language: Language) -> LanguageProfile {
	LanguageProfile {
		strings: RUST_STRINGS,
		string_prefixes: RUST_STRING_PREFIXES,
		hashed_raw_strings: true,
		raw_string_prefix: Some('r'),
		nestable_comments: true,
		decision_keywords: RUST_DECISIONS,
		nesting_keywords: RUST_NESTING,
		jump_keywords: NONE,
		ternary_operator: None,
		..c_family(language)
	}
}

/// TypeScript and TSX add optional chaining, nullish coalescing, regex literals, and
/// template literals whose `${…}` can nest quotes and braces.
fn typescript(language: Language) -> LanguageProfile {
	LanguageProfile {
		strings: TS_STRINGS,
		interpolation: Some(Interpolation::DollarBrace),
		regex_literals: true,
		decision_keywords: TS_DECISIONS,
		null_coalescing: NULLISH,
		..c_family(language)
	}
}

/// JavaScript shares TypeScript's operators without the type syntax.
fn javascript(language: Language) -> LanguageProfile {
	LanguageProfile {
		strings: TS_STRINGS,
		interpolation: Some(Interpolation::DollarBrace),
		regex_literals: true,
		null_coalescing: NULLISH,
		..c_family(language)
	}
}

/// C# adds `foreach`, pattern `when` clauses, verbatim strings, and raw strings.
fn csharp(language: Language) -> LanguageProfile {
	LanguageProfile {
		strings: CSHARP_STRINGS,
		string_prefixes: CSHARP_STRING_PREFIXES,
		interpolation: Some(Interpolation::Brace),
		decision_keywords: CSHARP_DECISIONS,
		null_coalescing: NULLISH,
		..c_family(language)
	}
}

/// Kotlin adds `when` expressions, safe calls, the Elvis operator, and nestable comments.
fn kotlin(language: Language) -> LanguageProfile {
	LanguageProfile {
		strings: KOTLIN_STRINGS,
		interpolation: Some(Interpolation::Dollar { braced: true }),
		nestable_comments: true,
		decision_keywords: KOTLIN_DECISIONS,
		nesting_keywords: KOTLIN_NESTING,
		null_coalescing: KOTLIN_NULLISH,
		..c_family(language)
	}
}

/// Dart adds `switch` patterns, `on` clauses, cascades, and `$`/`${}` interpolation.
fn dart(language: Language) -> LanguageProfile {
	LanguageProfile {
		strings: DART_STRINGS,
		string_prefixes: DART_STRING_PREFIXES,
		interpolation: Some(Interpolation::Dollar { braced: true }),
		decision_keywords: DART_DECISIONS,
		nesting_keywords: DART_NESTING,
		null_coalescing: NULLISH,
		..c_family(language)
	}
}

/// Swift adds `guard`, `repeat`, multiline strings, and nestable comments.
fn swift(language: Language) -> LanguageProfile {
	LanguageProfile {
		strings: SWIFT_STRINGS,
		interpolation: Some(Interpolation::Dollar { braced: true }),
		nestable_comments: true,
		decision_keywords: SWIFT_DECISIONS,
		nesting_keywords: SWIFT_NESTING,
		null_coalescing: SWIFT_NULLISH,
		..c_family(language)
	}
}

/// Go adds `select` and backtick raw strings.
fn go(language: Language) -> LanguageProfile {
	LanguageProfile {
		strings: GO_STRINGS,
		decision_keywords: GO_DECISIONS,
		nesting_keywords: GO_NESTING,
		..c_family(language)
	}
}

/// PHP adds `elseif`, `foreach`, `<<<` heredocs, and `$`-sigil variables.
///
/// PHP heredocs are why [`HeredocSyntax`] is separate from string rules: the body is data
/// until a line equal to the opening identifier, which no delimiter pair can express.
fn php(language: Language) -> LanguageProfile {
	LanguageProfile {
		line_comments: PHP_LINE_COMMENTS,
		strings: PHP_STRINGS,
		string_prefixes: NO_CHARS,
		interpolation: Some(Interpolation::Dollar { braced: true }),
		heredoc: Some(PHP_HEREDOC),
		decision_keywords: PHP_DECISIONS,
		nesting_keywords: PHP_NESTING,
		null_coalescing: PHP_NULLISH,
		variables_use_sigil: true,
		..c_family(language)
	}
}

/// Scala adds `match` and multiline strings that interpolate with `$`.
fn scala(language: Language) -> LanguageProfile {
	LanguageProfile {
		strings: SCALA_STRINGS,
		interpolation: Some(Interpolation::Dollar { braced: true }),
		decision_keywords: SCALA_DECISIONS,
		nesting_keywords: SCALA_NESTING,
		..c_family(language)
	}
}

/// Python is indentation-based and uses `and`/`or` rather than symbols.
///
/// f-strings are why interpolation tracks brace depth: `{…}` nests and `{{` is a literal
/// brace, and the nesting rules changed in 3.12.
fn python(language: Language) -> LanguageProfile {
	LanguageProfile {
		line_comments: PYTHON_LINE_COMMENTS,
		block_comments: NO_BLOCK_COMMENTS,
		strings: PYTHON_STRINGS,
		string_prefixes: PYTHON_STRING_PREFIXES,
		interpolation: Some(Interpolation::Brace),
		decision_keywords: PYTHON_DECISIONS,
		nesting_keywords: PYTHON_NESTING,
		logical_operators: PYTHON_LOGICAL,
		jump_keywords: NONE,
		ternary_operator: None,
		block_style: BlockStyle::Indentation,
		..c_family(language)
	}
}

/// Ruby terminates blocks with `end` and uses `#{…}` interpolation plus heredocs.
fn ruby(language: Language) -> LanguageProfile {
	LanguageProfile {
		line_comments: RUBY_LINE_COMMENTS,
		block_comments: RUBY_BLOCK_COMMENTS,
		strings: RUBY_STRINGS,
		string_prefixes: NO_CHARS,
		interpolation: Some(Interpolation::Hash),
		heredoc: Some(RUBY_HEREDOC),
		regex_literals: true,
		decision_keywords: RUBY_DECISIONS,
		nesting_keywords: RUBY_NESTING,
		logical_operators: RUBY_LOGICAL,
		jump_keywords: NONE,
		block_style: BlockStyle::EndKeyword,
		end_keywords: RUBY_END,
		variables_use_sigil: true,
		..c_family(language)
	}
}

/// Lua terminates blocks with `end` and opens long strings and comments with `[[`.
fn lua(language: Language) -> LanguageProfile {
	LanguageProfile {
		line_comments: LUA_LINE_COMMENTS,
		block_comments: LUA_BLOCK_COMMENTS,
		strings: LUA_STRINGS,
		string_prefixes: NO_CHARS,
		decision_keywords: LUA_DECISIONS,
		nesting_keywords: LUA_NESTING,
		logical_operators: LUA_LOGICAL,
		ternary_operator: None,
		block_style: BlockStyle::EndKeyword,
		end_keywords: LUA_END,
		..c_family(language)
	}
}

/// Elixir terminates blocks with `end` and leans on pattern-matching clauses.
fn elixir(language: Language) -> LanguageProfile {
	LanguageProfile {
		line_comments: ELIXIR_LINE_COMMENTS,
		block_comments: NO_BLOCK_COMMENTS,
		strings: ELIXIR_STRINGS,
		string_prefixes: NO_CHARS,
		interpolation: Some(Interpolation::Hash),
		decision_keywords: ELIXIR_DECISIONS,
		nesting_keywords: ELIXIR_NESTING,
		logical_operators: ELIXIR_LOGICAL,
		ternary_operator: None,
		block_style: BlockStyle::EndKeyword,
		end_keywords: ELIXIR_END,
		..c_family(language)
	}
}

/// Haskell is layout-sensitive, so nesting follows indentation.
fn haskell(language: Language) -> LanguageProfile {
	LanguageProfile {
		line_comments: HASKELL_LINE_COMMENTS,
		block_comments: HASKELL_BLOCK_COMMENTS,
		nestable_comments: true,
		strings: HASKELL_STRINGS,
		string_prefixes: NO_CHARS,
		decision_keywords: HASKELL_DECISIONS,
		nesting_keywords: HASKELL_NESTING,
		ternary_operator: None,
		block_style: BlockStyle::Indentation,
		..c_family(language)
	}
}

/// Nix uses `#` line comments and `''…''` indented strings.
fn nix(language: Language) -> LanguageProfile {
	LanguageProfile {
		line_comments: NIX_LINE_COMMENTS,
		block_comments: NIX_BLOCK_COMMENTS,
		strings: NIX_STRINGS,
		string_prefixes: NO_CHARS,
		interpolation: Some(Interpolation::Dollar { braced: true }),
		decision_keywords: NIX_DECISIONS,
		nesting_keywords: NIX_NESTING,
		ternary_operator: None,
		..c_family(language)
	}
}

/// Shell dialects terminate blocks with `fi`, `done`, and `esac`.
///
/// Heredocs carry the same weight as in PHP: `<<'EOF'` disables interpolation and `<<-EOF`
/// strips leading tabs, so the body must be skipped as data until the delimiter line.
fn shell(language: Language) -> LanguageProfile {
	LanguageProfile {
		line_comments: SHELL_LINE_COMMENTS,
		block_comments: NO_BLOCK_COMMENTS,
		strings: SHELL_STRINGS,
		string_prefixes: NO_CHARS,
		interpolation: Some(Interpolation::Dollar { braced: true }),
		heredoc: Some(SHELL_HEREDOC),
		decision_keywords: SHELL_DECISIONS,
		nesting_keywords: SHELL_NESTING,
		ternary_operator: None,
		block_style: BlockStyle::EndKeyword,
		end_keywords: SHELL_END,
		parameter_list_start: None,
		variables_use_sigil: true,
		..c_family(language)
	}
}

/// Markdown has no decisions of its own; its fences are analyzed as their own languages.
///
/// The profile exists so the type stays total and so fence-free prose can still be measured
/// for paragraph structure by the Markdown crate.
fn markdown(language: Language) -> LanguageProfile {
	LanguageProfile {
		line_comments: NONE,
		block_comments: NO_BLOCK_COMMENTS,
		strings: NO_STRINGS,
		string_prefixes: NO_CHARS,
		decision_keywords: NONE,
		nesting_keywords: NONE,
		logical_operators: NONE,
		jump_keywords: NONE,
		ternary_operator: None,
		null_coalescing: NONE,
		end_keywords: NONE,
		parameter_list_start: None,
		..c_family(language)
	}
}
