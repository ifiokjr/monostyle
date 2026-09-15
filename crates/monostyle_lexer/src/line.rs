//! A single lexed line.

use crate::comment::CommentIntent;

/// What a line is primarily doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LineKind {
	/// Whitespace only.
	Blank,
	/// Entirely a comment.
	Comment,
	/// Code with no trailing comment.
	Code,
	/// Code followed by a trailing comment.
	CodeWithComment,
	/// Content of a multi-line string, heredoc, or block comment.
	///
	/// These lines are prose or data, not code, so layout rules must not count them as
	/// statements while still not treating them as comments either.
	Literal,
}

/// One line of source, with the signals the readability rules need.
#[derive(Debug, Clone)]
pub struct LexedLine {
	/// 1-based line number.
	pub number: usize,
	/// The raw line text, without its terminator.
	pub text: String,
	/// The line's code with string and comment contents blanked out.
	///
	/// Keeping this means every later pass — keyword counting, parameter spans, brace
	/// tracking — sees the same masked view, so a `?` inside a Dart string can never be
	/// mistaken for a ternary by one pass after another pass masked it.
	pub masked_code: String,
	/// Characters of leading whitespace, with tabs expanded.
	pub indent: usize,
	/// The raw leading whitespace, preserved so indentation style can be measured.
	pub indent_text: String,
	/// What the line primarily is.
	pub kind: LineKind,
	/// Column where the code begins, ignoring leading whitespace.
	pub code_start_column: usize,
	/// Column where a trailing comment begins, when there is one.
	pub trailing_comment_column: Option<usize>,
	/// How the trailing comment, or a whole-line comment, reads.
	pub comment_intent: Option<CommentIntent>,
	/// Decision keywords found on this line.
	pub decisions: Vec<String>,
	/// Nesting keywords found on this line.
	pub nesting: Vec<String>,
	/// Number of short-circuiting operators on this line.
	pub logical_operators: usize,
	/// Number of null-coalescing or optional-chaining operators on this line.
	pub null_coalescing: usize,
	/// Whether a ternary operator appears on this line.
	pub has_ternary: bool,
	/// Number of jump statements on this line.
	pub jumps: usize,
	/// How many lines the parameter list opened here spans, or 0 when none opens.
	pub parameter_span: usize,
	/// Whether the line contains a `return`-like exit.
	pub is_return: bool,
}

impl LexedLine {
	/// Whether the line contributes executable code.
	#[must_use]
	pub fn is_code(&self) -> bool {
		matches!(self.kind, LineKind::Code | LineKind::CodeWithComment)
	}

	/// Whether the line's code begins a statement rather than continuing one.
	///
	/// A continuation line — a chained method call, a wrapped argument list, a multi-line
	/// expression — carries its indentation from the formatter's alignment rules rather than
	/// from the file's indentation convention. Rules that judge indentation must skip these, or
	/// they report the difference between "one tab" and "two tabs plus alignment spaces" as
	/// inconsistent indentation.
	#[must_use]
	pub fn starts_statement(&self) -> bool {
		self.is_code()
			&& !self
				.masked_code
				.trim_start()
				.starts_with(['.', ')', ']', ','])
	}

	/// Whether the line is blank.
	#[must_use]
	pub fn is_blank(&self) -> bool {
		self.kind == LineKind::Blank
	}

	/// Whether the line is a comment line.
	#[must_use]
	pub fn is_comment(&self) -> bool {
		self.kind == LineKind::Comment
	}

	/// Whether the line is inside a multi-line literal.
	#[must_use]
	pub fn is_literal(&self) -> bool {
		self.kind == LineKind::Literal
	}

	/// Total decision points contributed by this line.
	#[must_use]
	pub fn decision_count(&self) -> usize {
		self.decisions.len()
	}

	/// The number of characters of code on the line, ignoring indentation and comments.
	#[must_use]
	pub fn code_len(&self) -> usize {
		let end = self
			.trailing_comment_column
			.unwrap_or(self.text.chars().count());

		end.saturating_sub(self.code_start_column)
	}

	/// Whether the line's only content is a trailing comment after code.
	///
	/// Used when grouping comment blocks: a trailing comment belongs with the block above it only
	/// when there is no code between them.
	#[must_use]
	pub fn is_trailing_comment_only(&self) -> bool {
		matches!(self.kind, LineKind::CodeWithComment) && self.code_len() == 0
	}

	/// Returns the text of this line's comment.
	///
	/// The lexer keeps comment text inside `text` rather than a separate field, so this strips the
	/// leading marker so a caller can classify the prose without the syntax.
	#[must_use]
	pub fn comment_body(&self) -> String {
		let trimmed = self.text.trim_start();

		let body = trimmed
			.strip_prefix("///")
			.or_else(|| trimmed.strip_prefix("//!"))
			.or_else(|| trimmed.strip_prefix("/**"))
			.or_else(|| trimmed.strip_prefix("//"))
			.or_else(|| trimmed.strip_prefix("##"))
			.or_else(|| trimmed.strip_prefix('#'))
			.or_else(|| trimmed.strip_prefix("/*"))
			.or_else(|| trimmed.strip_prefix('*'))
			.or_else(|| trimmed.strip_prefix("--"))
			.unwrap_or(trimmed);

		body.trim_end_matches("*/").trim().to_string()
	}

	/// A short preview of the line's code, for report output.
	#[must_use]
	pub fn preview(&self, width: usize) -> String {
		let trimmed = self.text.trim();
		let mut preview: String = trimmed.chars().take(width).collect();

		if trimmed.chars().count() > width {
			preview.push('…');
		}

		preview
	}

	/// Whether this line opens a block of code.
	///
	/// Used to exempt the first statement inside a block from rules that would otherwise ask
	/// for a blank line before it, since there is nothing above it to separate from.
	#[must_use]
	pub fn opens_block(&self) -> bool {
		let trimmed = self.masked_code.trim_end();

		trimmed.ends_with('{')
			|| trimmed.ends_with("then")
			|| trimmed.ends_with("do")
			|| trimmed.ends_with(':')
			|| trimmed.ends_with("=>")
	}
}
