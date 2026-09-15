//! Applying fixes.
//!
//! # How edits are applied safely
//!
//! Fixes are byte-range replacements over the original source. Applying them in ascending order
//! would invalidate every offset after the first edit, so they are applied in **descending** order
//! by start offset: an edit near the end of the file shifts nothing that a later edit refers to.
//!
//! Two further rules keep the result correct:
//!
//! 1. **Overlapping fixes are dropped, not merged.** Two rules editing the same bytes cannot both
//!    be right, and picking one arbitrarily could produce code that neither rule intended. The
//!    first fix in file order wins and the conflict is reported.
//! 2. **A file is only written when every fix applied cleanly.** A partially fixed file is worse
//!    than an unfixed one, because the reader cannot tell which findings were addressed.

use std::path::Path;
use std::path::PathBuf;

use monostyle_core::Fix;

/// The outcome of applying fixes to one file.
#[derive(Debug, Clone)]
pub struct AppliedFixes {
	/// The file that was fixed.
	pub path: PathBuf,
	/// How many fixes were applied.
	pub applied: usize,
	/// How many were skipped because they overlapped another fix.
	pub conflicts: usize,
	/// Whether the file was written, or only planned.
	pub written: bool,
}

/// Applies `fixes` to `source`, returning the new text.
///
/// Returns the number of skipped conflicts alongside the text so a caller can report them rather
/// than silently dropping edits.
#[must_use]
pub fn apply_fixes(source: &str, fixes: &[Fix]) -> (String, usize) {
	// Conflicts are resolved before anything is applied, so the winner is decided by file order rather
	// than by the order the edits happen to be applied in. Resolving during application meant the edit
	// starting *latest* won, which contradicts what a reader expects from "the first fix in file
	// order" and would change if the application order ever changed.
	let mut ordered: Vec<&Fix> = fixes.iter().filter(|fix| !fix.is_empty()).collect();
	ordered.sort_by_key(|fix| (fix.span.start_byte, fix.span.end_byte));

	let mut accepted: Vec<&Fix> = Vec::new();
	let mut conflicts = 0;
	let mut last_end = 0;

	for fix in ordered {
		let start = fix.span.start_byte;
		let end = fix.span.end_byte;

		// A range outside the file is a rule bug rather than a fixable condition; skipping it is safer
		// than panicking on a slice.
		if end > source.len() || start > end {
			conflicts += 1;
			continue;
		}

		// Edits are sorted, so anything reaching past the previous edit's end overlaps it.
		if start < last_end {
			conflicts += 1;
			continue;
		}

		last_end = end;
		accepted.push(fix);
	}

	// Applying back to front is what keeps the offsets valid: each edit is further from the end than
	// the next one applied, so no earlier offset has moved yet.
	let mut result = source.to_string();

	for fix in accepted.iter().rev() {
		result.replace_range(fix.span.start_byte..fix.span.end_byte, &fix.replacement);
	}

	(result, conflicts)
}

/// Writes `fixes` to `path`, or reports what would be written.
///
/// A dry run performs every step except the write, so the reported counts are the ones a real run
/// would produce.
pub fn fix_file(path: &Path, fixes: &[Fix], dry_run: bool) -> std::io::Result<AppliedFixes> {
	let source = std::fs::read_to_string(path)?;
	let (rewritten, conflicts) = apply_fixes(&source, fixes);

	let applied = fixes.iter().filter(|fix| !fix.is_empty()).count() - conflicts;

	// Writing an unchanged file would touch its modification time, which defeats a build system
	// watching for changes.
	let changed = rewritten != source;

	if !dry_run && changed {
		std::fs::write(path, &rewritten)?;
	}

	Ok(AppliedFixes {
		path: path.to_path_buf(),
		applied,
		conflicts,
		written: !dry_run && changed,
	})
}
