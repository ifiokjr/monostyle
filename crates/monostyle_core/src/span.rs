//! Source locations.

use serde::Serialize;

/// A byte range within a single source file.
///
/// Line and column are 1-based so they can be printed directly to a terminal;
/// byte offsets remain 0-based to match the lexer's indexing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Span {
	/// Byte offset of the first byte of the span.
	pub start_byte: usize,
	/// Byte offset one past the last byte of the span.
	pub end_byte: usize,
	/// 1-based line of the span start.
	pub start_line: usize,
	/// 1-based line of the span end.
	pub end_line: usize,
}

impl Span {
	/// Creates a span from explicit offsets.
	#[must_use]
	pub const fn new(
		start_byte: usize,
		end_byte: usize,
		start_line: usize,
		end_line: usize,
	) -> Self {
		Self {
			start_byte,
			end_byte,
			start_line,
			end_line,
		}
	}

	/// Returns the number of lines the span covers, inclusive of both ends.
	#[must_use]
	pub const fn line_count(&self) -> usize {
		self.end_line.saturating_sub(self.start_line) + 1
	}
}
