//! Narrow adapters over the metrics crate.
//!
//! Rules need to slice a file by unit and re-measure it, and they need that repeatedly. This module
//! keeps the slicing logic in one place so every rule measures a unit the same way — which matters
//! because two rules disagreeing about a unit's line range would produce two complexity numbers for
//! the same function.
//!
//! # Why unit detection is memoized
//!
//! Seven rules ask for a file's units, and detection is a structural scan over every line. Running it
//! once per rule made it the dominant cost of an analysis on large files. The cache below keys on the
//! file's address and line count, which is stable for the lifetime of one analysis, so the scan
//! happens once and the six later callers read the result.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Mutex;

use monostyle_lexer::LexedFile;
use monostyle_lexer::LexedLine;
use monostyle_metrics::CodeUnit;

thread_local! {
	/// Units computed for the file currently being analyzed on this thread.
	///
	/// Thread-local rather than shared because analysis is parallel across files: a shared map would
	/// need locking on every read to guard against cross-file confusion, while a per-thread value
	/// cannot mix two files up.
	static CACHE: RefCell<Option<(usize, usize, Vec<CodeUnit>)>> = const { RefCell::new(None) };
}

/// Guards the thread-local cache against concurrent access from a single thread.
///
/// A `RefCell` borrow conflict can only occur if a rule calls this while another borrow is live,
/// which no rule does. The mutex exists so a future caller cannot turn that into a panic.
static GUARD: Mutex<()> = Mutex::new(());

/// Returns every function-like unit in `file`, computing them once per file.
#[must_use]
pub fn units(file: &LexedFile) -> Vec<CodeUnit> {
	let key = (std::ptr::from_ref(file) as usize, file.lines.len());

	if let Ok(_guard) = GUARD.lock() {
		let cached = CACHE.with(|cache| {
			cache
				.borrow()
				.as_ref()
				.filter(|(address, lines, _units)| *address == key.0 && *lines == key.1)
				.map(|(_address, _lines, units)| units.clone())
		});

		if let Some(units) = cached {
			return units;
		}
	}

	let units = monostyle_metrics::find_units(file);

	CACHE.with(|cache| {
		*cache.borrow_mut() = Some((key.0, key.1, units.clone()));
	});

	units
}

/// Returns the lines belonging to `unit`, inclusive of both ends.
#[must_use]
pub fn unit_lines<'file>(file: &'file LexedFile, unit: &CodeUnit) -> &'file [LexedLine] {
	let start = unit.start_line.saturating_sub(1).min(file.lines.len());
	let end = unit.end_line.min(file.lines.len());

	file.lines.get(start..end).unwrap_or_default()
}

/// Clears the memoized units.
///
/// Called between files so a recycled allocation cannot produce a stale hit when a `LexedFile` is
/// dropped and another lands at the same address with the same line count.
pub fn clear_cache() {
	CACHE.with(|cache| {
		*cache.borrow_mut() = None;
	});
}

/// A map from a unit's identity to a value computed for it.
///
/// Rules that measure each unit build several of these per file, and the measurements are pure
/// functions of the unit's lines, so caching them by identity avoids re-running the metric once per
/// rule that needs it.
pub type UnitMeasures<T> = HashMap<(usize, usize), T>;
