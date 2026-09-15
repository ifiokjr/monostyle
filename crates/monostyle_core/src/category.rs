//! Score categories.
//!
//! A [`Category`] names which of the two headline scores a finding feeds into.
//! Keeping this distinct from [`Severity`](crate::Severity) matters: a finding can be
//! minor in weight yet still belong to the readability score.

use serde::Serialize;

/// The headline score a finding contributes to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
	/// Cyclomatic and cognitive complexity pressure.
	Complexity,
	/// Visual and structural readability.
	Readability,
}

impl Category {
	/// Every category, in report order.
	pub const ALL: [Self; 2] = [Self::Complexity, Self::Readability];

	/// A short human-readable label.
	#[must_use]
	pub const fn label(self) -> &'static str {
		match self {
			Self::Complexity => "complexity",
			Self::Readability => "readability",
		}
	}
}
