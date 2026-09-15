//! Automatic fixes.
//!
//! A fix describes a single text edit that resolves a finding. Keeping it as a byte-range
//! replacement rather than as a rule-specific operation means the fixer never needs to understand
//! what a rule was looking for — it applies edits and nothing more.
//!
//! # Why byte offsets rather than line numbers
//!
//! A fix must be applied to the original source, and edits shift every later line. Expressing a fix
//! as a byte range over the original text lets the fixer sort edits back-to-front and apply them in
//! one pass without recomputing positions, which is what makes multiple fixes in one file safe.

use serde::Serialize;

use crate::Span;

/// A text edit that resolves a finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Fix {
	/// The byte range in the original source to replace.
	pub span: Span,
	/// What to put in its place.
	pub replacement: String,
	/// A short description of the edit, shown by `monostyle fix --dry-run`.
	pub description: String,
}

impl Fix {
	/// Creates a fix that replaces `span` with `replacement`.
	#[must_use]
	pub fn replace(
		span: Span,
		replacement: impl Into<String>,
		description: impl Into<String>,
	) -> Self {
		Self {
			span,
			replacement: replacement.into(),
			description: description.into(),
		}
	}

	/// Creates a fix that inserts `text` at the start of `span`.
	///
	/// Insertion is expressed as a zero-width replacement, so the fixer has one code path rather
	/// than separate insert and replace handling.
	#[must_use]
	pub fn insert(at: Span, text: impl Into<String>, description: impl Into<String>) -> Self {
		Self {
			span: Span::new(at.start_byte, at.start_byte, at.start_line, at.start_line),
			replacement: text.into(),
			description: description.into(),
		}
	}

	/// Creates a fix that deletes `span` entirely.
	#[must_use]
	pub fn delete(span: Span, description: impl Into<String>) -> Self {
		Self::replace(span, "", description)
	}

	/// Whether this fix changes anything.
	#[must_use]
	pub fn is_empty(&self) -> bool {
		self.span.start_byte == self.span.end_byte && self.replacement.is_empty()
	}
}
