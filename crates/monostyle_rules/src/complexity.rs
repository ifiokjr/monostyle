//! Complexity rules.
//!
//! These rules turn the raw complexity metrics into explained findings. The metrics crate
//! computes numbers; this module says whether a number is a problem and what to do about it.
//!
//! Cyclomatic and cognitive complexity are reported separately and weighted differently,
//! because they answer different questions: cyclomatic complexity tells you how many tests
//! you need, while cognitive complexity tells you how hard the function is to follow.

use monostyle_core::Category;
use monostyle_core::Finding;
use monostyle_core::FindingBuilder;
use monostyle_core::Severity;
use monostyle_core::Span;
use monostyle_lexer::LexedFile;

use crate::config::RulesConfig;
use crate::metrics_bridge;

/// Reports units whose cyclomatic complexity exceeds the configured limit.
///
/// The message names the risk band as well as the number, because "complexity 14" means
/// little on its own while "untestable without a table-driven suite" means a lot.
pub fn cyclomatic_per_unit(file: &LexedFile, config: &RulesConfig) -> Vec<Finding> {
	let units = metrics_bridge::units(file);
	let mut findings = Vec::new();

	for unit in units {
		let lines = metrics_bridge::unit_lines(file, &unit);
		let complexity = monostyle_metrics::complexity_of_lines(lines);

		if complexity.total <= config.max_cyclomatic_per_unit {
			continue;
		}

		let excess = complexity.total - config.max_cyclomatic_per_unit;

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
			// Weight scales with the excess so that a function at 30 costs far more than one
			// at 11, rather than both crossing the same cliff.
			.weight(1.0 + f64_from(excess) * 0.25)
			.message(format!(
				"`{}` has a cyclomatic complexity of {} ({} risk), over the limit of {}",
				unit.name,
				complexity.total,
				complexity.risk().label(),
				config.max_cyclomatic_per_unit
			))
			.suggestion(format!(
				"This function has {} independent paths: {} branches, {} loops, {} cases, and \
				 {} logical or conditional operators. Extract cohesive groups of branches into \
				 named helpers to bring each back under {}.",
				complexity.total,
				complexity.breakdown.branches,
				complexity.breakdown.loops,
				complexity.breakdown.cases,
				complexity.breakdown.logical + complexity.breakdown.conditionals,
				config.max_cyclomatic_per_unit
			))
			.build(),
		);
	}

	findings
}

/// Reports units whose cognitive complexity exceeds the configured limit.
///
/// This is the readability-facing half of complexity, so it is weighted more heavily than
/// the cyclomatic rule: a function that is hard to follow hurts more than one that merely
/// needs more tests.
pub fn cognitive_per_unit(file: &LexedFile, config: &RulesConfig) -> Vec<Finding> {
	let units = metrics_bridge::units(file);
	let mut findings = Vec::new();

	for unit in units {
		let lines = metrics_bridge::unit_lines(file, &unit);
		let cognitive = monostyle_metrics::cognitive_complexity_of_lines(lines);

		if cognitive.total <= config.max_cognitive_per_unit {
			continue;
		}

		let excess = cognitive.total - config.max_cognitive_per_unit;

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
				cognitive.total,
				cognitive.grade(),
				config.max_cognitive_per_unit
			))
			.suggestion(if cognitive.nesting_penalty > 0 {
				format!(
					"{} of these points come from nesting, at a maximum depth of {}. Flatten \
					 with early returns or extract the inner blocks, which resets the nesting \
					 penalty without reducing the number of branches.",
					cognitive.nesting_penalty, cognitive.max_nesting
				)
			} else {
				"Split this function along its distinct responsibilities so each branch group \
				 can be read in isolation."
					.to_string()
			})
			.build(),
		);
	}

	findings
}

/// Reports files whose total complexity is high enough to make the whole file hard to hold
/// in mind, even when no single function is over its limit.
pub fn complexity_per_file(file: &LexedFile, config: &RulesConfig) -> Vec<Finding> {
	/// Decisions per 100 lines above which a file is considered dense.
	const DENSITY_LIMIT: f64 = 25.0;

	/// Minimum lines of code before a decision density is meaningful.
	const MIN_LINES_FOR_DENSITY: usize = 40;

	let complexity = monostyle_metrics::cyclomatic_complexity(file);
	let lines = file.source_line_count();

	// A file needs enough code for a density figure to mean anything. Below this the ratio is
	// dominated by rounding — a twelve-line file with three branches looks "dense" purely
	// because it is short — and the per-unit rules are the right place to catch real problems.

	if lines < MIN_LINES_FOR_DENSITY {
		return Vec::new();
	}

	let density = f64_from(complexity.total) / f64_from(lines) * 100.0;

	if density <= DENSITY_LIMIT {
		return Vec::new();
	}

	let span = file.lines.first().map_or(Span::new(0, 0, 1, 1), |line| {
		Span::new(0, line.text.len(), line.number, line.number)
	});

	vec![
		FindingBuilder::new("complexity/cyclomatic-per-file", Category::Complexity, span)
			.severity(Severity::Minor)
			.weight(0.75)
			.message(format!(
				"this file averages {density:.1} decisions per 100 lines, over the \
				 {DENSITY_LIMIT:.0} limit"
			))
			.suggestion(format!(
				"The file has {} decision points across {lines} lines of code (limit {} per \
				 file). Split it along its natural seams into smaller modules.",
				complexity.total, config.max_file_lines
			))
			.build(),
	]
}

/// Converts a `usize` to `f64` for weighting.
///
/// Weights are presentation values rather than measurements, so the precision loss is
/// irrelevant; the conversion is confined here so the intent is visible.
fn f64_from(value: usize) -> f64 {
	value as f64
}
