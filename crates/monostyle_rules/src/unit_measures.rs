//! Per-unit measurement, computed once per file.
//!
//! # The cost this removes
//!
//! Eight rules need a complexity figure for each function-like unit: cyclomatic, cognitive, `NPath`, exits,
//! Halstead, and maintainability. Each figure is a scan over the unit's lines, and each rule was doing its
//! own scan. On a 17,000-line file with 499 functions that was 499 units times eight scans, which was the
//! dominant cost of the whole analysis.
//!
//! # Why this is a value rather than a cache
//!
//! The first version memoized by the [`LexedFile`]'s memory address in a thread-local map. That was
//! unsound, and the symptom was severe: the same input produced complexity scores between 10 and 79 across
//! runs. Rayon drops a file's `LexedFile` when its task finishes, and the next file allocated at the same
//! address inherited the previous file's metrics — so a file was scored with another file's numbers.
//! **Address identity is not file identity.**
//!
//! This module now computes a [`UnitSet`] as a plain value and hands it to the rules. There is no key, no
//! thread-local state, and no way for one file's measurements to reach another: the lifetime belongs to the
//! caller, so a stale read is not expressible.

use monostyle_lexer::LexedFile;
use monostyle_lexer::LexedLine;
use monostyle_metrics::CodeUnit;
use monostyle_metrics::ExitCount;
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
	pub exits: ExitCount,
	/// Halstead volume and counts.
	pub halstead: Halstead,
	/// Maintainability index on a 0–100 scale.
	pub maintainability: f64,
	/// Whether a why-comment or doc comment appears anywhere inside the unit.
	pub documented: bool,
}

/// A unit and its measurements.
#[derive(Debug, Clone)]
pub struct MeasuredUnit {
	/// The detected unit.
	pub unit: CodeUnit,
	/// The unit's measurements.
	pub metrics: UnitMetrics,
}

/// Every unit in one file, already measured.
///
/// Built once per file and handed to the rules, so the work happens exactly once however many rules ask for
/// a figure.
#[derive(Debug, Clone)]
pub struct UnitSet {
	units: Vec<MeasuredUnit>,
}

impl UnitSet {
	/// Measures every unit in `file`.
	#[must_use]
	pub fn new(file: &LexedFile) -> Self {
		let units = monostyle_metrics::find_units(file)
			.into_iter()
			.map(|unit| {
				let metrics = measure(file, &unit);

				MeasuredUnit { unit, metrics }
			})
			.collect();

		Self { units }
	}

	/// Iterates over each unit with its measurements.
	pub fn iter(&self) -> impl Iterator<Item = (&CodeUnit, &UnitMetrics)> {
		self.units
			.iter()
			.map(|measured| (&measured.unit, &measured.metrics))
	}

	/// Returns the measurement for `unit`, or `None` when it is not part of this set.
	///
	/// A rule that needs one unit's figures looks it up by line range, which identifies it within the file
	/// the set was built from.
	#[must_use]
	pub fn metrics_for(&self, unit: &CodeUnit) -> Option<&UnitMetrics> {
		self.units
			.iter()
			.find(|measured| measured.unit.start_line == unit.start_line)
			.map(|measured| &measured.metrics)
	}

	/// Whether the file declared no units at all.
	#[must_use]
	pub fn is_empty(&self) -> bool {
		self.units.is_empty()
	}

	/// The number of units.
	#[must_use]
	pub fn len(&self) -> usize {
		self.units.len()
	}
}

/// Measures one unit.
fn measure(file: &LexedFile, unit: &CodeUnit) -> UnitMetrics {
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
pub fn lines_of<'file>(file: &'file LexedFile, unit: &CodeUnit) -> &'file [LexedLine] {
	let start = unit.start_line.saturating_sub(1).min(file.lines.len());
	let end = unit.end_line.min(file.lines.len());

	file.lines.get(start..end).unwrap_or_default()
}
