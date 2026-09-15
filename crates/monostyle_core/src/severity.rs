//! Finding severity.

use serde::Serialize;

/// How much a finding should worry the reader.
///
/// Severity expresses confidence and impact together. `Info` exists so that
/// genuinely useful observations which should not move a score still have a way
/// to surface in reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
	/// Context worth reporting that does not change any score.
	Info,
	/// A small readability or complexity cost.
	Minor,
	/// A meaningful cost that a reviewer should address.
	Major,
	/// A cost severe enough to make the code hard to trust or maintain.
	Critical,
}

impl Severity {
	/// A short human-readable label.
	#[must_use]
	pub const fn label(self) -> &'static str {
		match self {
			Self::Info => "info",
			Self::Minor => "minor",
			Self::Major => "major",
			Self::Critical => "critical",
		}
	}

	/// The penalty multiplier applied to a rule's base weight.
	///
	/// Multiplying (rather than using the severity as the weight itself) lets a rule
	/// control its own magnitude while severity scales it — so `Severity` stays a
	/// small, stable vocabulary across every language.
	#[must_use]
	pub fn penalty_factor(self) -> f64 {
		match self {
			Self::Info => 0.0,
			Self::Minor => 1.0,
			Self::Major => 2.5,
			Self::Critical => 5.0,
		}
	}
}
