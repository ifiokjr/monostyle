//! Cyclomatic complexity.
//!
//! Cyclomatic complexity is the number of linearly independent paths through a body of
//! code: one path for entering it, plus one per decision point. It is a lower bound on
//! the number of test cases needed for branch coverage, which is why it is worth
//! reporting even though it says nothing about how *hard* the code is to read.
//!
//! The counting rules here follow the same definitions `rust-code-analysis` uses so that
//! numbers from the two tools are comparable:
//!
//! | Construct | Adds |
//! | --- | --- |
//! | Function entry | 1 |
//! | `if` / `else if` / `elif` | 1 each |
//! | `else` | 0 (no new path) |
//! | `for` / `while` / `loop` | 1 each |
//! | `case` / `when` arm | 1 each |
//! | `catch` / `except` / `rescue` | 1 each |
//! | `&&`, `\|\|`, `and`, `or` | 1 each |
//! | Ternary | 1 |
//! | `??`, `?.` | 1 each |
//! | `goto` | 1 each |
//!
//! Notably absent: `else` branches, `switch` statements themselves (their `case` arms
//! carry the paths), and `try` blocks.

use monostyle_lexer::LexedFile;
use monostyle_lexer::LexedLine;
use serde::Serialize;

/// A cyclomatic complexity measurement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct CyclomaticComplexity {
	/// The total complexity, always at least 1.
	pub total: usize,
	/// Decision points counted, broken out so reports can explain the total.
	pub breakdown: ComplexityBreakdown,
}

/// Where each counted decision came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct ComplexityBreakdown {
	/// `if` and `else if` decisions.
	pub branches: usize,
	/// Loop decisions.
	pub loops: usize,
	/// `case` and `when` arms.
	pub cases: usize,
	/// Exception handlers.
	pub catches: usize,
	/// Short-circuiting boolean operators.
	pub logical: usize,
	/// Ternaries, null-coalescing, and optional chaining.
	pub conditionals: usize,
	/// Jump statements.
	pub jumps: usize,
}

impl CyclomaticComplexity {
	/// A measurement for a body with no decisions.
	pub const BASELINE: usize = 1;

	/// A qualitative band for a complexity value.
	///
	/// The thresholds follow the widely used NIST/McCabe guidance of 10 as the upper
	/// bound for a maintainable function.
	#[must_use]
	pub fn risk(&self) -> Risk {
		match self.total {
			0..=5 => Risk::Simple,
			6..=10 => Risk::Moderate,
			11..=20 => Risk::Complex,
			21..=50 => Risk::Untestable,
			_ => Risk::Unmaintainable,
		}
	}
}

/// The risk band for a complexity value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Risk {
	/// Straightforward, easy to test exhaustively.
	Simple,
	/// Still understandable, tests are becoming table-driven.
	Moderate,
	/// Refactoring is warranted.
	Complex,
	/// Too many paths to test by hand.
	Untestable,
	/// Effectively impossible to verify.
	Unmaintainable,
}

impl Risk {
	/// A short human-readable label.
	#[must_use]
	pub const fn label(self) -> &'static str {
		match self {
			Self::Simple => "simple",
			Self::Moderate => "moderate",
			Self::Complex => "complex",
			Self::Untestable => "untestable",
			Self::Unmaintainable => "unmaintainable",
		}
	}
}

/// Computes cyclomatic complexity for every line in `file`.
///
/// The total is for the whole file; per-unit values come from
/// [`find_units`](crate::unit::find_units), which slices the same lines.
#[must_use]
pub fn cyclomatic_complexity(file: &LexedFile) -> CyclomaticComplexity {
	complexity_of_lines(&file.lines)
}

/// Computes cyclomatic complexity over a slice of lines.
#[must_use]
pub fn complexity_of_lines(lines: &[LexedLine]) -> CyclomaticComplexity {
	let mut breakdown = ComplexityBreakdown::default();

	for line in lines {
		for decision in &line.decisions {
			match decision.as_str() {
				"if" | "else if" | "elif" | "elseif" | "unless" | "guard" => {
					breakdown.branches += 1;
				}
				"for" | "foreach" | "while" | "until" | "repeat" | "loop" | "do" => {
					breakdown.loops += 1;
				}
				"case" | "when" | "on" => breakdown.cases += 1,
				"catch" | "except" | "rescue" => breakdown.catches += 1,
				"switch" | "match" | "select" | "cond" | "default" | "assert" | "where" => {
					// These open a construct whose arms carry their own paths, so the
					// construct itself adds nothing.
				}
				_ => breakdown.branches += 1,
			}
		}

		breakdown.logical += line.logical_operators;
		breakdown.conditionals += line.null_coalescing;
		breakdown.jumps += line.jumps;

		if line.has_ternary {
			breakdown.conditionals += 1;
		}
	}

	let decisions = breakdown.branches
		+ breakdown.loops
		+ breakdown.cases
		+ breakdown.catches
		+ breakdown.logical
		+ breakdown.conditionals
		+ breakdown.jumps;

	CyclomaticComplexity {
		total: decisions + CyclomaticComplexity::BASELINE,
		breakdown,
	}
}
