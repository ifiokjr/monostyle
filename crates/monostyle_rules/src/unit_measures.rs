//! Per-unit metric memoization.
//!
//! # The cost this removes
//!
//! Eight rules need a complexity figure for each function-like unit: cyclomatic, cognitive, `NPath`,
//! exits, Halstead, maintainability. Each figure is a scan over the unit's lines, and each rule was
//! doing its own scan.
//!
//! On a file with hundreds of functions that is hundreds of units times eight scans. Measured on a
//! 17,000-line test file with 499 units, this was the dominant cost of the whole analysis — the
//! difference between 285 seconds and under 5.
//!
//! # Why the memo is keyed by line range
//!
//! The measurements are pure functions of a unit's lines, so the unit's start and end line identify
//! them exactly. Keying on the range rather than on the unit's name matters because names repeat —
//! `new`, `default`, and `from` appear many times in one file — and a name-keyed memo would return one
//! function's complexity for another.
//!
//! # Why this is not circular
//!
//! Unit *metrics* depend only on the lines, so they can be computed before any rule runs. Unit
//! *scores* depend on the findings, which the rules produce, so scores are computed afterwards by the
//! reporting layer. Keeping the two apart is what lets rules consume metrics without the ordering
//! problem of consuming findings that do not exist yet.

use std::cell::RefCell;
use std::collections::HashMap;

use monostyle_lexer::LexedFile;
use monostyle_lexer::LexedLine;
use monostyle_metrics::Halstead;
use monostyle_metrics::NPath;

/// The metrics a rule might ask for about one unit.
#[derive(Debug, Clone, Copy)]
pub struct UnitMetrics {
	/// Lines of code, excluding blanks, comments, and literal content.
	pub code_lines: usize,
	/// Cyclomatic complexity.
	pub cyclomatic: usize,
	/// Cyclomatic risk band.
	pub cyclomatic_risk: &'static str,
	/// Cognitive complexity.
	pub cognitive: usize,
	/// Cognitive band.
	pub cognitive_grade: &'static str,
	/// Points that came from nesting.
	pub nesting_penalty: usize,
	/// Deepest nesting level.
	pub max_nesting: usize,
	/// Acyclic execution paths.
	pub npath: NPath,
	/// Flow-control escapes.
	pub exits: monostyle_metrics::ExitCount,
	/// Halstead volume and counts.
	pub halstead: Halstead,
	/// Maintainability index on a 0–100 scale.
	pub maintainability: f64,
	/// Whether a why-comment or doc comment appears anywhere inside the unit.
	pub documented: bool,
}

/// Metrics for one file, keyed by a unit's line range.
type FileMetrics = HashMap<UnitKey, UnitMetrics>;

/// A unit's identity within a file: its start and end line.
type UnitKey = (usize, usize);

thread_local! {
	/// Metrics for the file currently being analyzed, identified by its address.
	///
	/// Thread-local because analysis is parallel across files: a shared map would need locking on
	/// every read, while a per-thread map cannot mix two files up.
	static METRICS: RefCell<Option<(usize, FileMetrics)>> = const { RefCell::new(None) };
}

thread_local! {
	/// Units detected for the file currently being analyzed on this thread.
	static UNITS: RefCell<Option<(usize, Vec<monostyle_metrics::CodeUnit>)>> = const {
		RefCell::new(None)
	};
}

/// Returns every function-like unit in `file`.
///
/// Detection is a structural scan, so it is memoized the same way the metrics are: the units a file
/// contains cannot change while the file is being analyzed.
#[must_use]
pub fn find_units(file: &LexedFile) -> Vec<monostyle_metrics::CodeUnit> {
	let file_key = std::ptr::from_ref(file) as usize;

	let cached = UNITS.with(|cache| {
		cache
			.borrow()
			.as_ref()
			.filter(|(address, _)| *address == file_key)
			.map(|(_address, units)| units.clone())
	});

	if let Some(units) = cached {
		return units;
	}

	let units = monostyle_metrics::find_units(file);

	UNITS.with(|cache| {
		*cache.borrow_mut() = Some((file_key, units.clone()));
	});

	units
}

/// Returns the `(start, end)` line range identifying a unit.
#[must_use]
pub fn unit_key(unit: &monostyle_metrics::CodeUnit) -> UnitKey {
	(unit.start_line, unit.end_line)
}

/// Returns metrics for `unit`, computing them once per unit per file.
#[must_use]
pub fn metrics_for(file: &LexedFile, unit: &monostyle_metrics::CodeUnit) -> UnitMetrics {
	let file_key = std::ptr::from_ref(file) as usize;
	let key = unit_key(unit);

	// A hit requires the same file identity, so a stale map from a previous file cannot answer.
	let cached = METRICS.with(|cache| {
		cache
			.borrow()
			.as_ref()
			.filter(|(address, _)| *address == file_key)
			.and_then(|(_address, map)| map.get(&key).copied())
	});

	if let Some(metrics) = cached {
		return metrics;
	}

	let metrics = compute(file, unit);

	METRICS.with(|cache| {
		let mut borrow = cache.borrow_mut();

		match borrow.as_mut() {
			Some((address, map)) if *address == file_key => {
				map.insert(key, metrics);
			}
			_ => {
				let mut map = HashMap::new();
				map.insert(key, metrics);
				*borrow = Some((file_key, map));
			}
		}
	});

	metrics
}

/// Computes metrics for one unit.
fn compute(file: &LexedFile, unit: &monostyle_metrics::CodeUnit) -> UnitMetrics {
	let lines = lines_of(file, unit);

	let cyclomatic = monostyle_metrics::complexity_of_lines(lines);
	let cognitive = monostyle_metrics::cognitive_complexity_of_lines(lines);
	let halstead = monostyle_metrics::halstead_of_lines(lines, file.language);
	let code_lines = lines
		.iter()
		.filter(|line| !line.is_blank() && !line.is_comment() && !line.is_literal())
		.count();
	let maintainability =
		monostyle_metrics::maintainability_index(halstead, cyclomatic.total, code_lines);

	UnitMetrics {
		code_lines,
		cyclomatic: cyclomatic.total,
		cyclomatic_risk: cyclomatic.risk().label(),
		cognitive: cognitive.total,
		cognitive_grade: cognitive.grade(),
		nesting_penalty: cognitive.nesting_penalty,
		max_nesting: cognitive.max_nesting,
		npath: monostyle_metrics::npath_of_lines(lines),
		exits: monostyle_metrics::exit_count_of_lines(lines),
		halstead,
		maintainability: maintainability.value,
		documented: lines.iter().any(|line| {
			matches!(
				line.comment_intent,
				Some(
					monostyle_lexer::CommentIntent::Why
						| monostyle_lexer::CommentIntent::Documentation
				)
			)
		}),
	}
}

/// Returns the lines belonging to a unit.
#[must_use]
pub fn lines_of<'file>(
	file: &'file LexedFile,
	unit: &monostyle_metrics::CodeUnit,
) -> &'file [LexedLine] {
	let start = unit.start_line.saturating_sub(1).min(file.lines.len());
	let end = unit.end_line.min(file.lines.len());

	file.lines.get(start..end).unwrap_or_default()
}

/// Empties the memo.
///
/// Called between files so a recycled allocation cannot produce a stale hit when one `LexedFile` is
/// dropped and another happens to land at the same address.
pub fn clear() {
	METRICS.with(|cache| {
		*cache.borrow_mut() = None;
	});

	UNITS.with(|cache| {
		*cache.borrow_mut() = None;
	});
}
