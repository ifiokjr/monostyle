//! The language profile type.
//!
//! Profiles are pure data. Adding a language to monostyle means adding a profile entry,
//! not writing parsing code, which is what keeps the support surface wide without a
//! grammar compile step.

use monostyle_core::Language;

/// How a language delimiters a block of code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlockStyle {
	/// Braces: `{ ... }`. C, Rust, Dart, Go, and friends.
	Brace,
	/// Indentation, with nesting tracked by column. Python and Haskell.
	Indentation,
	/// A closing keyword terminates the block. Ruby (`end`), Lua, Shell (`fi`, `done`).
	EndKeyword,
}

/// How a language embeds expressions inside string literals.
///
/// Interpolation matters to the scanner because the delimiters inside an interpolated
/// expression are structural, not literal — a quote inside `"${foo("x")}"` does not end
/// the outer string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Interpolation {
	/// A `$` sigil: `$name` or, when `braced`, `${expression}`. Dart, Shell, PHP, Nix.
	Dollar {
		/// Whether the `${expression}` form is supported.
		braced: bool,
	},
	/// A `#{expression}` sequence. Ruby.
	Hash,
	/// A `${expression}` sequence only. JavaScript template literals.
	DollarBrace,
	/// A `{expression}` sequence with `{{`/`}}` escapes. Python f-strings.
	Brace,
}

/// How a language introduces a heredoc string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HeredocSyntax {
	/// The operator that starts a heredoc: `<<` for shell and Ruby, `<<<` for PHP.
	pub marker: &'static str,
	/// Whether `<<-EOF` (leading tab stripping) is accepted.
	pub allows_dash: bool,
	/// Whether `<<~EOF` (indentation stripping) is accepted, as in Ruby.
	pub allows_tilde: bool,
}

/// A string literal rule.
///
/// Every field earns its place from a concrete mis-scan it prevents; see the failure-mode
/// table in `ARCHITECTURE.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StringRule {
	/// The opening delimiter.
	pub open: &'static str,
	/// The closing delimiter.
	pub close: &'static str,
	/// Whether backslash escapes apply inside the literal.
	pub escapes: bool,
	/// Whether the literal may span physical lines.
	///
	/// This is what stops an unterminated single-quoted string from swallowing the rest of
	/// the file: languages that forbid multiline `'…'` close it at end of line.
	pub multiline: bool,
	/// Whether the literal embeds expressions.
	pub interpolates: bool,
	/// Delimiter-like sequences that are escapes rather than terminators.
	///
	/// Nix is why this exists: inside an indented string, `''$`, `'''`, and `''\` are
	/// escapes, so a bare `''` only sometimes closes the literal. Without this the scanner
	/// ends a Nix string early and treats the remainder as code.
	pub extra_escapes: &'static [&'static str],
}

impl StringRule {
	/// A single-line literal that honours backslash escapes.
	#[must_use]
	pub const fn escaped(open: &'static str, close: &'static str) -> Self {
		Self {
			open,
			close,
			escapes: true,
			multiline: false,
			interpolates: false,
			extra_escapes: &[],
		}
	}

	/// A single-line literal that ignores backslash escapes.
	#[must_use]
	pub const fn raw(open: &'static str, close: &'static str) -> Self {
		Self {
			open,
			close,
			escapes: false,
			multiline: false,
			interpolates: false,
			extra_escapes: &[],
		}
	}

	/// A literal that may span lines.
	#[must_use]
	pub const fn multiline(open: &'static str, close: &'static str) -> Self {
		Self {
			open,
			close,
			escapes: true,
			multiline: true,
			interpolates: false,
			extra_escapes: &[],
		}
	}

	/// A multiline literal that ignores backslash escapes.
	#[must_use]
	pub const fn multiline_raw(open: &'static str, close: &'static str) -> Self {
		Self {
			open,
			close,
			escapes: false,
			multiline: true,
			interpolates: false,
			extra_escapes: &[],
		}
	}

	/// A single-line literal that embeds expressions.
	#[must_use]
	pub const fn interpolated(open: &'static str, close: &'static str) -> Self {
		Self {
			open,
			close,
			escapes: true,
			multiline: false,
			interpolates: true,
			extra_escapes: &[],
		}
	}

	/// A multiline literal that embeds expressions.
	#[must_use]
	pub const fn multiline_interpolated(open: &'static str, close: &'static str) -> Self {
		Self {
			open,
			close,
			escapes: true,
			multiline: true,
			interpolates: true,
			extra_escapes: &[],
		}
	}

	/// Declares delimiter-like escape sequences for this literal.
	#[must_use]
	pub const fn with_extra_escapes(mut self, escapes: &'static [&'static str]) -> Self {
		self.extra_escapes = escapes;
		self
	}
}

/// Everything the scanner needs to know about one language.
///
/// The several boolean fields describe independent syntax features rather than a state machine,
/// so they are kept flat: `#[allow]` is deliberate here rather than a shortcut.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Copy)]
pub struct LanguageProfile {
	/// The language this profile describes.
	pub language: Language,
	/// Tokens that start a comment running to end of line.
	pub line_comments: &'static [&'static str],
	/// Delimiter pairs that start and end block comments.
	pub block_comments: &'static [(&'static str, &'static str)],
	/// Whether block comments nest, as in Rust, Swift, and Kotlin.
	pub nestable_comments: bool,
	/// String literal rules, ordered so the longest openers come first.
	pub strings: &'static [StringRule],
	/// Characters that may immediately precede a string delimiter.
	pub string_prefixes: &'static [char],
	/// Whether Rust-style `r#"…"#` literals with a hash count are supported.
	pub hashed_raw_strings: bool,
	/// If the language uses a distinct sigil for raw strings, the prefix that introduces
	/// a Rust-style literal, such as `r`.
	pub raw_string_prefix: Option<char>,
	/// How string interpolation is written, when supported.
	pub interpolation: Option<Interpolation>,
	/// How heredocs are introduced, when supported.
	pub heredoc: Option<HeredocSyntax>,
	/// Whether `/` can begin a regular expression literal, as in JavaScript and Ruby.
	pub regex_literals: bool,
	/// Keywords that each add an independent path through a function.
	pub decision_keywords: &'static [&'static str],
	/// Keywords that open a nestable block.
	pub nesting_keywords: &'static [&'static str],
	/// Operators that each add an independent path (short-circuiting).
	pub logical_operators: &'static [&'static str],
	/// Keywords that break linear control flow.
	pub jump_keywords: &'static [&'static str],
	/// The ternary operator character, when the language has one.
	pub ternary_operator: Option<char>,
	/// Null-coalescing and optional-chaining operators, which add paths.
	pub null_coalescing: &'static [&'static str],
	/// How blocks are delimited.
	pub block_style: BlockStyle,
	/// Keywords that close a block under [`BlockStyle::EndKeyword`].
	pub end_keywords: &'static [&'static str],
	/// The character that opens a parameter list, when the language has one.
	pub parameter_list_start: Option<char>,
	/// Whether variables are written with a sigil, as in PHP, Ruby, and Shell.
	pub variables_use_sigil: bool,
}

impl LanguageProfile {
	/// Returns the block-comment pair that `text` starts with, if any.
	///
	/// Longer delimiters are matched first so that Lua's `--[[` beats `--`.
	#[must_use]
	pub fn block_comment_at(&self, text: &str) -> Option<(&'static str, &'static str)> {
		let mut matches: Vec<(&'static str, &'static str)> = self
			.block_comments
			.iter()
			.copied()
			.filter(|(open, _)| text.starts_with(open))
			.collect();

		matches.sort_by_key(|(open, _)| std::cmp::Reverse(open.len()));

		matches.into_iter().next()
	}

	/// Returns the line-comment token that `text` starts with, if any.
	///
	/// Longer tokens are matched first so that `///` is recognized before `//`.
	#[must_use]
	pub fn line_comment_at(&self, text: &str) -> Option<&'static str> {
		let mut matches: Vec<&'static str> = self
			.line_comments
			.iter()
			.copied()
			.filter(|token| text.starts_with(token))
			.collect();

		matches.sort_by_key(|token| std::cmp::Reverse(token.len()));

		matches.into_iter().next()
	}

	/// Whether this language documents items with string literals rather than comments.
	///
	/// Python is the notable case: a docstring is the first statement in a function or module, not a
	/// comment, so recognizing it requires looking at the literal rather than at comment syntax. Shell
	/// is excluded despite its `##` convention, which is a comment form and is handled below.
	#[must_use]
	pub const fn uses_doc_strings(self) -> bool {
		matches!(self.language, Language::Python | Language::Elixir)
	}

	/// Returns true when `text`, already known to start a line comment, is documentation.
	#[must_use]
	pub fn is_documentation_comment(&self, text: &str) -> bool {
		let trimmed = text.trim_start();

		// Rust and C# mark documentation with a third slash or an exclamation mark.
		if trimmed.starts_with("///") || trimmed.starts_with("//!") {
			return true;
		}

		// JSDoc-style block documentation, used by TypeScript, JavaScript, Java, PHP, Kotlin,
		// Swift, and Scala. This was missing at first, which meant the most common documentation
		// form in those languages was keyword-judged as if it were a casual inline comment.
		if trimmed.starts_with("/**") {
			return true;
		}

		// Python, Elixir, and Shell mark documentation with a doubled hash.
		if trimmed.starts_with("##") {
			return true;
		}

		trimmed.starts_with("//<") || trimmed.starts_with("///<")
	}

	/// Returns the string rule whose opening delimiter `text` starts with.
	///
	/// Longest opener wins so that Dart's `'''` is preferred over `'`.
	#[must_use]
	pub fn string_at(&self, text: &str) -> Option<&'static StringRule> {
		let mut matches: Vec<&'static StringRule> = self
			.strings
			.iter()
			.filter(|rule| text.starts_with(rule.open))
			.collect();

		matches.sort_by_key(|rule| std::cmp::Reverse(rule.open.len()));

		matches.into_iter().next()
	}

	/// Whether a character in operand position makes `/` a regular expression.
	///
	/// This is the JavaScript regex-versus-division ambiguity: `/` starts a literal only
	/// where an operand is expected, so `a / b` divides while `(/b/)` matches.
	#[must_use]
	pub fn regex_allowed_after(&self, previous: char) -> bool {
		if !self.regex_literals {
			return false;
		}

		matches!(
			previous,
			'(' | ','
				| '=' | ':' | '['
				| '!' | '&' | '|'
				| '?' | '{' | '}'
				| ';' | '+' | '-'
				| '*' | '%' | '<'
				| '>' | '^' | '~'
		)
	}
}
