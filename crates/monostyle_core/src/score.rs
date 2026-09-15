//! Score computation.
//!
//! # How a score is derived
//!
//! Every rule emits [`Finding`](crate::Finding)s. Each finding carries a `weight`, which
//! [`Severity`](crate::Severity) scales into a penalty. Penalties are summed per
//! [`Category`] and divided by the amount of code they were found in to produce a
//! *penalty density* — findings per 100 lines. Density, not raw count, is what the
//! score uses, so a large well-written file is not punished for its size.
//!
//! Density is then mapped onto 0–100 with an exponential decay:
//!
//! ```text
//! score = 100 * 2 ^ (-density / half_life)
//! ```
//!
//! The half-life is the density at which a category scores exactly 50. Expressing the
//! curve this way keeps it tunable with a single understandable number: raising the
//! half-life makes the tool more forgiving, lowering it makes it stricter. Because
//! the curve is monotonic and never reaches zero, scores stay comparable across
//! files instead of collapsing to a floor.
//!
//! Findings may carry a **negative** weight. That is how the comment-quality rules
//! reward a well-placed explanation: the credit offsets other penalties rather than
//! being tracked somewhere separate, so a single density number always explains the
//! final score.

use serde::Deserialize;
use serde::Serialize;

use crate::Category;
use crate::Finding;

/// The smallest amount of code a score is normalized against.
///
/// Without a floor, a five-line function containing one finding would produce an
/// enormous density and score near zero. Twenty lines is roughly the smallest unit
/// where "findings per 100 lines" is a meaningful statement.
const MIN_NORMALIZATION_LINES: f64 = 20.0;

/// Default penalty density at which a category scores 50.
const DEFAULT_HALF_LIFE: f64 = 12.0;

/// Tunable knobs for turning findings into a number.
///
/// Serialized with kebab-case keys to match `RulesConfig`, so a configuration file uses one naming
/// convention throughout.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ScoringConfig {
	/// Penalty density per 100 lines that yields a score of 50.
	pub half_life: f64,
	/// Lines used as the denominator floor when normalizing.
	pub min_normalization_lines: f64,
}

impl Default for ScoringConfig {
	fn default() -> Self {
		Self {
			half_life: DEFAULT_HALF_LIFE,
			min_normalization_lines: MIN_NORMALIZATION_LINES,
		}
	}
}

impl ScoringConfig {
	/// Returns a config that scores more leniently.
	#[must_use]
	pub fn lenient() -> Self {
		Self {
			half_life: DEFAULT_HALF_LIFE * 2.0,
			..Self::default()
		}
	}

	/// Returns a config that scores more strictly.
	#[must_use]
	pub fn strict() -> Self {
		Self {
			half_life: DEFAULT_HALF_LIFE / 2.0,
			..Self::default()
		}
	}
}

/// A score out of 100, where 100 is clean and 0 is unusable.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Score {
	/// The score itself, always within `0.0..=100.0`.
	pub value: f64,
	/// Raw sum of penalties, including any negative credit.
	pub penalty: f64,
	/// Penalties normalized to findings per 100 lines.
	pub density: f64,
}

impl Score {
	/// The best possible score.
	pub const PERFECT: f64 = 100.0;

	/// Computes a score from the findings in one category.
	///
	/// `lines` is the amount of code the findings were collected from, used only as
	/// the normalization denominator.
	#[must_use]
	pub fn from_findings(
		findings: &[Finding],
		category: Category,
		lines: usize,
		config: ScoringConfig,
	) -> Self {
		let penalty: f64 = findings
			.iter()
			.filter(|finding| finding.category == category)
			.map(Finding::penalty)
			.sum();

		// The denominator is floored so tiny units still produce a stable number, and
		// scaled to "per 100 lines" so the half-life reads as a percentage-ish unit.
		let denominator = (lines as f64).max(config.min_normalization_lines);
		let density = penalty / denominator * 100.0;

		// A negative density means credit outweighed penalties; that is a valid state
		// which simply clamps to a perfect score.
		let half_life = if config.half_life > 0.0 {
			config.half_life
		} else {
			DEFAULT_HALF_LIFE
		};
		let raw = Score::PERFECT * 2f64.powf(-density / half_life);

		Self {
			value: raw.clamp(0.0, Score::PERFECT),
			penalty,
			density,
		}
	}

	/// Computes a score directly from a penalty density.
	///
	/// Aggregation needs this because a project's density is a *weighted mean of per-file
	/// densities*, not a single penalty divided by a single line count. Exposing the curve here
	/// keeps one implementation of it, so a file's score and a project's score cannot drift apart.
	#[must_use]
	pub fn from_density(density: f64, penalty: f64, config: ScoringConfig) -> Self {
		let half_life = if config.half_life > 0.0 {
			config.half_life
		} else {
			DEFAULT_HALF_LIFE
		};
		let raw = Score::PERFECT * 2f64.powf(-density / half_life);

		Self {
			value: raw.clamp(0.0, Score::PERFECT),
			penalty,
			density,
		}
	}

	/// A perfect score with no findings behind it.
	#[must_use]
	pub fn perfect() -> Self {
		Self {
			value: Score::PERFECT,
			penalty: 0.0,
			density: 0.0,
		}
	}

	/// A short qualitative band for the score, useful in reports and CI output.
	#[must_use]
	pub fn grade(&self) -> &'static str {
		match self.value {
			value if value >= 90.0 => "excellent",
			value if value >= 75.0 => "good",
			value if value >= 60.0 => "fair",
			value if value >= 40.0 => "poor",
			_ => "bad",
		}
	}
}

impl std::fmt::Display for Score {
	fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(formatter, "{:.1}", self.value)
	}
}
