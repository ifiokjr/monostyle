//! Complexity rules.
//!
//! These rules turn the raw complexity metrics into explained findings. The metrics crate computes
//! numbers; this module says whether a number is a problem and what to do about it.
//!
//! Cyclomatic and cognitive complexity are reported separately and weighted differently, because they
//! answer different questions: cyclomatic complexity tells you how many tests you need, while
//! cognitive complexity tells you how hard the function is to follow.
//!
//! Each finding carries a suggestion that names the concrete change. A complexity number on its own
//! tells a reader that something is wrong without telling them what to do, which is the difference
//! between a report people act on and one they ignore.

use monostyle_core::Category;
use monostyle_core::Finding;
use monostyle_core::FindingBuilder;
use monostyle_core::Severity;
use monostyle_core::Span;
use monostyle_lexer::LexedFile;

use crate::config::RulesConfig;
use crate::unit_measures;

/// Reports units whose cyclomatic complexity exceeds the configured limit.
///
/// The message names the risk band as well as the number, because "complexity 14" means little on its
/// own while "untestable without a table-driven suite" means a lot.
#[must_use]
pub fn cyclomatic_per_unit(file: &LexedFile, config: &RulesConfig) -> Vec<Finding> {
	let units = unit_measures::UnitSet::new(file);
	let mut findings = Vec::new();

	for (unit, metrics) in units.iter() {
		let metrics = *metrics;

		if metrics.cyclomatic <= config.max_cyclomatic_per_unit {
			continue;
		}

		let excess = metrics.cyclomatic - config.max_cyclomatic_per_unit;

		findings.push(
			FindingBuilder::new(
				"complexity/cyclomatic-per-unit",
				Category::Complexity,
				Span::new(0, 0, unit.start_line, unit.start_line),
			)
			.severity(if excess > config.max_cyclomatic_per_unit {
				Severity::Critical
			} else {
				Severity::Major
			})
			// Weight scales with the excess so that a function at 30 costs far more than one at 11,
			// rather than both crossing the same cliff.
			.weight(1.0 + f64_from(excess) * 0.25)
			.message(format!(
				"`{}` has a cyclomatic complexity of {} ({} risk), over the limit of {}",
				unit.name,
				metrics.cyclomatic,
				metrics.cyclomatic_risk,
				config.max_cyclomatic_per_unit
			))
			.suggestion(format!(
				"This function has {} independent paths, {excess} over the limit of {}. Extract \
				 cohesive groups of branches into named helpers so each one can be tested on its own.",
				metrics.cyclomatic, config.max_cyclomatic_per_unit
			))
			.build(),
		);
	}

	findings
}

/// Reports units whose cognitive complexity exceeds the configured limit.
///
/// This is the readability-facing half of complexity, so it is weighted more heavily than the
/// cyclomatic rule: a function that is hard to follow hurts more than one that merely needs more
/// tests.
///
/// The suggestion distinguishes the two ways to reduce it. When the score is driven by nesting, the
/// advice is to flatten — extraction resets the nesting penalty without removing any branch. When it
/// is not, the advice is to split along responsibilities, because there is nothing to flatten.
///
/// The loop body branches on the three ways a unit can clear its limit — total, nesting share,
/// or worst single unit — because each drives a different suggestion, and folding them into one
/// condition would lose which advice the reader should take.
#[must_use]
pub fn cognitive_per_unit(file: &LexedFile, config: &RulesConfig) -> Vec<Finding> {
	let units = unit_measures::UnitSet::new(file);
	let mut findings = Vec::new();

	for (unit, metrics) in units.iter() {
		let metrics = *metrics;

		if metrics.cognitive <= config.max_cognitive_per_unit {
			continue;
		}

		let excess = metrics.cognitive - config.max_cognitive_per_unit;

		findings.push(
			FindingBuilder::new(
				"complexity/cognitive-per-unit",
				Category::Complexity,
				Span::new(0, 0, unit.start_line, unit.start_line),
			)
			.severity(if excess > config.max_cognitive_per_unit {
				Severity::Critical
			} else {
				Severity::Major
			})
			.weight(1.25 + f64_from(excess) * 0.3)
			.message(format!(
				"`{}` has a cognitive complexity of {} ({}), over the limit of {}",
				unit.name,
				metrics.cognitive,
				metrics.cognitive_grade,
				config.max_cognitive_per_unit
			))
			.suggestion(if metrics.nesting_penalty > 0 {
				format!(
					"{} of these points come from nesting, at a maximum depth of {}. Flatten with \
					 early returns or extract the inner blocks: extraction resets the nesting penalty \
					 without reducing the number of branches.",
					metrics.nesting_penalty, metrics.max_nesting
				)
			} else {
				"Split this function along its distinct responsibilities so each branch group can be \
				 read in isolation."
					.to_string()
			})
			.build(),
		);
	}

	findings
}

/// Reports units with more execution paths than the configured limit.
///
/// `NPath` grows multiplicatively where cyclomatic complexity grows additively, so a unit can pass the
/// cyclomatic limit while having thousands of paths. Reporting it separately catches the function
/// that is not deeply nested but is exhaustively unverifiable.
#[must_use]
pub fn npath_per_unit(file: &LexedFile, config: &RulesConfig) -> Vec<Finding> {
	let units = unit_measures::UnitSet::new(file);
	let mut findings = Vec::new();

	for (unit, metrics) in units.iter() {
		let metrics = *metrics;
		let npath = metrics.npath;

		if npath.value <= config.max_npath_per_unit {
			continue;
		}

		let displayed = if npath.capped {
			format!("{}+", monostyle_metrics::NPath::CAP)
		} else {
			npath.value.to_string()
		};

		findings.push(
			FindingBuilder::new(
				"complexity/npath-per-unit",
				Category::Complexity,
				Span::new(0, 0, unit.start_line, unit.start_line),
			)
			.severity(Severity::Major)
			// Weight grows with the logarithm of the count, because the difference between a thousand
			// paths and ten thousand matters far less than the difference between ten and a hundred.
			.weight(1.0 + f64_from(npath.value).log10() * 0.4)
			.message(format!(
				"`{}` has {displayed} execution paths ({}), over the limit of {}",
				unit.name,
				npath.grade(),
				config.max_npath_per_unit
			))
			.suggestion(format!(
				"Sequential branches multiply here rather than add, so every guard clause compounds. \
				 Reduce the count to {} by extracting cohesive branch groups into named helpers, and \
				 by replacing sequential tests with a single match or lookup.",
				config.max_npath_per_unit
			))
			.build(),
		);
	}

	findings
}

/// Reports units that return from too many places.
///
/// Early returns are preferred to nesting, so this only fires well past the point where guards are
/// idiomatic. A function with many exits is hard to reason about because the reader must hold every
/// escape in mind to know what it guarantees.
///
/// The counter is structural rather than syntactic — it counts exit *shapes* a lexer can see, so a
/// language without a `return` keyword is still measured; the branches enumerate the shapes.
#[must_use]
pub fn exits_per_unit(file: &LexedFile, config: &RulesConfig) -> Vec<Finding> {
	let units = unit_measures::UnitSet::new(file);
	let mut findings = Vec::new();

	for (unit, metrics) in units.iter() {
		let metrics = *metrics;
		let exits = metrics.exits;

		if exits.exits <= config.max_exits_per_unit {
			continue;
		}

		let mut breakdown = Vec::new();

		if exits.throws > 0 {
			breakdown.push(format!("{} that raise", exits.throws));
		}

		if exits.jumps > 0 {
			breakdown.push(format!("{} that jump", exits.jumps));
		}

		let detail = if breakdown.is_empty() {
			String::new()
		} else {
			format!(" ({})", breakdown.join(", "))
		};

		findings.push(
			FindingBuilder::new(
				"complexity/exits-per-unit",
				Category::Complexity,
				Span::new(0, 0, unit.start_line, unit.start_line),
			)
			.severity(Severity::Minor)
			.weight(0.75)
			.message(format!(
				"`{}` returns from {} places{detail}, over the limit of {}",
				unit.name, exits.exits, config.max_exits_per_unit
			))
			.suggestion(
				"Consolidate the exits into a single result, or extract the guard clauses into their \
				 own function so each has one way out.",
			)
			.build(),
		);
	}

	findings
}

/// Reports units whose maintainability index is too low.
///
/// The index combines Halstead volume, cyclomatic complexity, and line count, so it catches a unit
/// that is dense with arithmetic rather than branches — a case the other complexity rules miss.
#[must_use]
pub fn maintainability_per_unit(file: &LexedFile, config: &RulesConfig) -> Vec<Finding> {
	/// Lines below which a maintainability index is not meaningful.
	const MIN_MEASURABLE_LINES: usize = 10;

	let units = unit_measures::UnitSet::new(file);
	let mut findings = Vec::new();

	for (unit, metrics) in units.iter() {
		let metrics = *metrics;

		// A handful of lines cannot produce a meaningful index, so a short unit is skipped rather
		// than reported for having a low one.
		if metrics.code_lines < MIN_MEASURABLE_LINES {
			continue;
		}

		if metrics.maintainability >= config.min_maintainability {
			continue;
		}

		findings.push(
			FindingBuilder::new(
				"complexity/low-maintainability",
				Category::Complexity,
				Span::new(0, 0, unit.start_line, unit.start_line),
			)
			.severity(Severity::Major)
			.weight(1.25)
			.message(format!(
				"`{}` has a maintainability index of {:.0}, below the limit of {:.0}",
				unit.name, metrics.maintainability, config.min_maintainability
			))
			.suggestion(format!(
				"This unit holds {} distinct operators, {} distinct operands, and a Halstead volume of \
				 {:.0} across {} lines. Splitting it lowers the volume faster than any other change, \
				 because volume grows with both the vocabulary and the length.",
				metrics.halstead.distinct_operators,
				metrics.halstead.distinct_operands,
				metrics.halstead.volume,
				metrics.code_lines
			))
			.build(),
		);
	}

	findings
}

/// Reports files whose total complexity is high enough to make the whole file hard to hold in mind,
/// even when no single function is over its limit.
#[must_use]
pub fn complexity_per_file(file: &LexedFile, _config: &RulesConfig) -> Vec<Finding> {
	/// Minimum lines of code before a decision density is meaningful.
	const MIN_LINES_FOR_DENSITY: usize = 40;

	/// Decisions per 100 lines above which a file is considered dense.
	const DENSITY_LIMIT: f64 = 25.0;

	let complexity = monostyle_metrics::cyclomatic_complexity(file);
	let lines = file.source_line_count();

	// A file needs enough code for a density figure to mean anything. Below this the ratio is
	// dominated by rounding, and the per-unit rules are the right place to catch real problems.
	if lines < MIN_LINES_FOR_DENSITY {
		return Vec::new();
	}

	let density = f64_from(complexity.total) / f64_from(lines) * 100.0;

	if density <= DENSITY_LIMIT {
		return Vec::new();
	}

	let span = file.lines.first().map_or(Span::new(0, 0, 1, 1), |line| {
		Span::new(line.start_byte, line.end_byte, line.number, line.number)
	});

	vec![
		FindingBuilder::new("complexity/cyclomatic-per-file", Category::Complexity, span)
			.severity(Severity::Minor)
			.weight(0.75)
			.message(format!(
				"this file averages {density:.1} decisions per 100 lines, over the {DENSITY_LIMIT:.0} \
				 limit"
			))
			.suggestion(format!(
				"The file has {} decision points across {lines} lines of code. Split it along its \
				 natural seams into smaller modules, so each one can be understood on its own.",
				complexity.total
			))
			.build(),
	]
}

/// Converts a `usize` to `f64` for weighting.
///
/// Weights are presentation values rather than measurements, so the precision loss is irrelevant; the
/// conversion is confined here so the intent is visible.
fn f64_from(value: usize) -> f64 {
	value as f64
}
