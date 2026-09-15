//! Narrow adapters over the metrics crate.
//!
//! Rules need to slice a file by unit and re-measure it, and they need that repeatedly. This
//! module keeps the slicing logic in one place so every rule measures a unit the same way —
//! which matters because two rules disagreeing about a unit's line range would produce two
//! complexity numbers for the same function.

use monostyle_lexer::LexedFile;
use monostyle_lexer::LexedLine;
use monostyle_metrics::CodeUnit;

/// Returns every function-like unit in `file`.
#[must_use]
pub fn units(file: &LexedFile) -> Vec<CodeUnit> {
	monostyle_metrics::find_units(file)
}

/// Returns the lines belonging to `unit`, inclusive of both ends.
#[must_use]
pub fn unit_lines<'file>(file: &'file LexedFile, unit: &CodeUnit) -> &'file [LexedLine] {
	let start = unit.start_line.saturating_sub(1).min(file.lines.len());
	let end = unit.end_line.min(file.lines.len());

	file.lines.get(start..end).unwrap_or_default()
}
