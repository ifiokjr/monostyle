//! Additional structural metrics.
//!
//! These complement cyclomatic and cognitive complexity rather than replacing them. Each captures a
//! pressure the others miss:
//!
//! | Metric | What it captures |
//! | --- | --- |
//! | [`NPath`] | The number of acyclic execution paths, which grows multiplicatively with sequential branching |
//! | [`ExitCount`] | How many places a function can return from |
//! | [`NestingProfile`] | The depth distribution, not just the maximum |
//!
//! `NPath` matters because cyclomatic complexity is additive while execution paths are multiplicative.
//! A function with six sequential `if`s and no nesting has a cyclomatic complexity of seven and
//! sixty-four paths, and the second number is what a reader actually has to reason about.

use monostyle_lexer::LexedFile;
use monostyle_lexer::LexedLine;
use serde::Serialize;

/// The number of acyclic execution paths through a body of code.
///
/// Computed with the standard `NPath` rules: a sequence multiplies its members' counts, and a
/// conditional adds its branches' counts. The value is capped while accumulating, because it grows
/// faster than exponentially and an uncapped `usize` overflows on realistic input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct NPath {
	/// The path count, capped at [`NPath::CAP`].
	pub value: usize,
	/// Whether the count hit the cap rather than being exact.
	pub capped: bool,
}

impl NPath {
	/// The value beyond which the count stops being useful.
	///
	/// Ten million paths is far past the point where anyone can reason about the code, so
	/// distinguishing beyond it adds no information while risking overflow.
	pub const CAP: usize = 10_000_000;
	/// The minimum, for a body with no branches.
	pub const MINIMUM: usize = 1;

	/// A qualitative band for the count.
	#[must_use]
	pub fn grade(&self) -> &'static str {
		match self.value {
			0..=8 => "simple",
			9..=64 => "moderate",
			65..=1024 => "complex",
			1025..=100_000 => "severe",
			_ => "untestable",
		}
	}
}

/// Where and how often a body exits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct ExitCount {
	/// Number of `return`, `yield`, and explicit exit statements.
	pub exits: usize,
	/// Number of places that raise or throw.
	pub throws: usize,
	/// Number of `break` and `continue` statements.
	pub jumps: usize,
}

impl ExitCount {
	/// Total flow-control escapes.
	#[must_use]
	pub fn total(&self) -> usize {
		self.exits + self.throws + self.jumps
	}
}

/// The distribution of nesting depth, not only its maximum.
///
/// # What "depth" counts
///
/// A function's own body is depth one, so a function containing a single `if` reaches depth two. This
/// matches how indentation is counted and how the other rules report nesting, and it means a file of
/// flat top-level statements reports a maximum of one rather than zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct NestingProfile {
	/// The deepest nesting reached, counting a function's own body as depth one.
	pub max_depth: usize,
	/// How many lines sit at each depth, indexed by depth.
	///
	/// The first entry is depth zero. This is what distinguishes a function whose deepest point is
	/// one nested block from one that spends most of its length five levels down.
	pub depth_histogram: [usize; Self::MAX_TRACKED],
}

impl NestingProfile {
	/// Depths tracked in the histogram.
	pub const MAX_TRACKED: usize = 12;

	/// The share of lines sitting deeper than `depth`.
	#[must_use]
	pub fn share_deeper_than(&self, depth: usize) -> f64 {
		let total: usize = self.depth_histogram.iter().sum();

		if total == 0 {
			return 0.0;
		}

		let deep: usize = self.depth_histogram.iter().skip(depth + 1).sum();

		deep as f64 / total as f64
	}

	/// The mean depth across all code lines.
	#[must_use]
	pub fn mean_depth(&self) -> f64 {
		let total: usize = self.depth_histogram.iter().sum();

		if total == 0 {
			return 0.0;
		}

		let weighted: usize = self
			.depth_histogram
			.iter()
			.enumerate()
			.map(|(depth, count)| depth * count)
			.sum();

		weighted as f64 / total as f64
	}
}

/// Computes the acyclic path count for `file`.
#[must_use]
pub fn npath(file: &LexedFile) -> NPath {
	npath_of_lines(&file.lines)
}

/// Computes the acyclic path count for a slice of lines.
///
/// The estimate is driven by decision points and nesting rather than by a control-flow graph, which
/// keeps it linear in file size. Branches that share a nesting level multiply together, because they
/// are sequential independent tests, while branches nested inside another branch multiply with it.
#[must_use]
pub fn npath_of_lines(lines: &[LexedLine]) -> NPath {
	let mut total = NPath::MINIMUM;
	let mut depth: usize = 0;
	let mut capped = false;

	for line in lines {
		if !line.is_code() {
			continue;
		}

		if line.decision_count() > 0 {
			total = grow(total, line.decision_count() + 1, &mut capped);
		}

		// Each open block adds a nesting level; a nested branch multiplies rather than adds, which is the
		// difference between this metric and cyclomatic complexity.
		depth = depth.saturating_add(line.masked_code.matches('{').count());
		depth = depth.saturating_sub(line.masked_code.matches('}').count());
	}

	NPath {
		value: total,
		capped,
	}
}

/// Multiplies `total` by `factor`, stopping at the cap rather than overflowing.
///
/// The count grows faster than exponentially, so an uncapped multiplication overflows on realistic input.
/// Reaching the cap is recorded because a caller reporting "10000000+" needs to know the figure is a floor.
fn grow(total: usize, factor: usize, capped: &mut bool) -> usize {
	match total.checked_mul(factor) {
		Some(value) if value <= NPath::CAP => value,
		_ => {
			*capped = true;

			NPath::CAP
		}
	}
}

/// Counts the exits in `file`.
#[must_use]
pub fn exit_count(file: &LexedFile) -> ExitCount {
	exit_count_of_lines(&file.lines)
}

/// Counts the exits in a slice of lines.
#[must_use]
pub fn exit_count_of_lines(lines: &[LexedLine]) -> ExitCount {
	/// Words that raise or propagate an error.
	const THROW_WORDS: &[&str] = &["throw", "raise", "panic", "error", "fail"];
	/// Words that jump within a loop.
	const JUMP_WORDS: &[&str] = &["break", "continue", "goto"];

	let mut counts = ExitCount::default();

	for line in lines {
		if !line.is_code() {
			continue;
		}

		if line.is_return {
			counts.exits += 1;
		}

		counts.throws += count_words(&line.masked_code, THROW_WORDS);
		counts.jumps += count_words(&line.masked_code, JUMP_WORDS);
	}

	counts
}

/// Counts how many words in `text` appear in `words`.
fn count_words(text: &str, words: &[&str]) -> usize {
	text.split(|character: char| !character.is_alphanumeric() && character != '_')
		.filter(|word| words.contains(word))
		.count()
}

/// Computes the nesting profile for `file`.
#[must_use]
pub fn nesting_profile(file: &LexedFile) -> NestingProfile {
	nesting_profile_of_lines(&file.lines)
}

/// Computes the nesting profile for a slice of lines.
#[must_use]
pub fn nesting_profile_of_lines(lines: &[LexedLine]) -> NestingProfile {
	let mut depth: usize = 0;
	let mut max_depth = 0;
	let mut depth_histogram = [0usize; NestingProfile::MAX_TRACKED];

	for line in lines {
		if !line.is_code() {
			continue;
		}

		// A line is recorded at the depth it sits at, before its own braces open, so a line that
		// opens a block is counted at the shallower level. The line's *contents* sit one level
		// deeper, which is the depth the line creates and therefore the depth to compare against the
		// maximum; comparing before the update meant `max_depth` reported one level less than the
		// deepest line actually reached.
		// The bucket is clamped to the array's range, and the modulo repeats that guarantee in a form the
		// bounds check can see.
		let bucket = depth.min(NestingProfile::MAX_TRACKED - 1) % NestingProfile::MAX_TRACKED;

		if let Some(count) = depth_histogram.get_mut(bucket) {
			*count = count.saturating_add(1);
		}

		let opens = line.masked_code.matches('{').count();
		let closes = line.masked_code.matches('}').count();

		depth = depth.saturating_add(opens);

		// Only a line that opened a block contributes a new depth, so a closing brace does not raise
		// the maximum on its way out.
		if opens > 0 {
			max_depth = max_depth.max(depth);
		}

		depth = depth.saturating_sub(closes);
	}

	NestingProfile {
		max_depth,
		depth_histogram,
	}
}
