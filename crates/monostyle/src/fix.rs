//! Applying fixes.
//!
//! # How edits are applied safely
//!
//! Fixes are byte-range replacements over the original source. Applying them in ascending order
//! would invalidate every offset after the first edit, so they are applied in **descending** order
//! by start offset: an edit near the end of the file shifts nothing that a later edit refers to.
//!
//! Four further rules keep the result correct:
//!
//! 1. **Overlapping fixes are dropped, not merged.** Two rules editing the same bytes cannot both
//!    be right, and picking one arbitrarily could produce code that neither rule intended. The
//!    first fix in file order wins and the conflict is reported.
//! 2. **A file is only written when every fix applied cleanly.** A partially fixed file is worse
//!    than an unfixed one, because the reader cannot tell which findings were addressed.
//! 3. **Nothing is edited inside a literal or a comment.** The lexer reports the byte ranges of
//!    strings, heredocs, and comments; a fix whose range enters one is rejected, because changing
//!    those bytes changes the program's data rather than its layout.
//! 4. **A whitespace-only rewrite is checked before it is written.** Moving blank lines cannot
//!    change the code's shape, so the fixed text is re-lexed and compared with the original: the
//!    same lines in the same kinds, the same literal contents, the same unterminated constructs.
//!    A mismatch means a rule mis-saw the file, and the file is left untouched.

use std::path::Path;
use std::path::PathBuf;

use monostyle_core::Fix;
use monostyle_lexer::LexedFile;
use monostyle_lexer::LineKind;
use monostyle_lexer::ProtectedRange;
use monostyle_lexer::lex;

/// The outcome of applying fixes to one file.
#[derive(Debug, Clone)]
pub struct AppliedFixes {
	/// The file that was fixed.
	pub path: PathBuf,
	/// How many fixes were applied.
	pub applied: usize,
	/// How many were skipped because they overlapped another fix.
	pub conflicts: usize,
	/// How many were refused because they would edit inside a literal or comment.
	pub rejected: usize,
	/// Whether the file was written, or only planned.
	pub written: bool,
	/// Whether the rewrite failed the structural check and was thrown away.
	pub reverted: bool,
	/// Whether the file was skipped because its scan hit an unterminated construct.
	pub skipped_untrusted: bool,
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

		// An edit that reaches past the previous edit's end is a genuine overlap and cannot be
		// merged, so it loses and the winner stands.
		if start < last_end {
			conflicts += 1;
			continue;
		}

		// Two rules may anchor the same blank-line insertion at one byte — a `return` that
		// contains a control-flow keyword is both a return and a branch, and both rules ask
		// for the space above it. The edits agree, so one application satisfies both; applying
		// both wrote two blank lines, which formatters then collapsed.
		if start == end
			&& let Some(previous) = accepted.last()
			&& previous.span.start_byte == previous.span.end_byte
			&& previous.span.start_byte == start
		{
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

/// Whether `fix` would edit bytes strictly inside a protected region.
///
/// An insertion (a zero-width range) is allowed at either boundary of a region — placing a blank
/// line immediately before a string that starts a line is a layout change, not an edit to the
/// string — but not in its interior. Any non-zero-width range that overlaps a region's interior
/// is refused.
#[must_use]
pub fn enters_protected(fix: &Fix, ranges: &[ProtectedRange]) -> bool {
	let start = fix.span.start_byte;
	let end = fix.span.end_byte;

	ranges.iter().any(|range| {
		if start == end {
			range.start_byte < start && start < range.end_byte
		} else {
			start < range.end_byte && range.start_byte < end
		}
	})
}

/// Writes `fixes` to `path`, or reports what would be written.
///
/// A dry run performs every step except the write, so the reported counts are the ones a real run
/// would produce.
pub fn fix_file(path: &Path, fixes: &[Fix], dry_run: bool) -> std::io::Result<AppliedFixes> {
	let source = std::fs::read_to_string(path)?;

	// A scan that needed recovery means the lexer guessed where the constructs end. Its protected
	// ranges may be wrong in exactly the way that would let a fix corrupt a literal, so the file
	// is left for a human.
	let scan = crate::analysis::language_for_path(path).map(|language| lex(&source, language));
	let scan_clean = scan.as_ref().is_none_or(LexedFile::is_clean);

	let outcome = |applied: usize,
	               conflicts: usize,
	               rejected: usize,
	               written: bool,
	               reverted: bool,
	               skipped_untrusted: bool| {
		AppliedFixes {
			path: path.to_path_buf(),
			applied,
			conflicts,
			rejected,
			written,
			reverted,
			skipped_untrusted,
		}
	};

	if !scan_clean {
		return Ok(outcome(0, 0, 0, false, false, true));
	}

	let ranges = scan
		.as_ref()
		.map(|lexed| lexed.protected.clone())
		.unwrap_or_default();
	let rejected = fixes
		.iter()
		.filter(|fix| enters_protected(fix, &ranges))
		.count();
	let fixable: Vec<Fix> = fixes
		.iter()
		.filter(|fix| !fix.is_empty() && !enters_protected(fix, &ranges))
		.cloned()
		.collect();

	let (rewritten, conflicts) = apply_fixes(&source, &fixable);
	let applied = fixable.len().saturating_sub(conflicts);

	// The structural check runs over the whole fixed text, so a mismatch is a property of the file
	// rather than of one edit: everything is thrown away, not just the suspicious edit.
	let reverted =
		whitespace_only(&fixable) && !structure_preserved(&source, &rewritten, scan.as_ref());

	// Writing an unchanged file would touch its modification time, which defeats a build system
	// watching for changes. A pure-CRLF file has no bare line feeds of its own, so any the fixes
	// introduced are re-terminated rather than mixed into the file's line endings.
	let pure_crlf = is_pure_crlf(&source);
	let rewritten = if pure_crlf {
		normalize_to_crlf(&rewritten)
	} else {
		rewritten
	};
	let changed = rewritten != source && !reverted;

	if !dry_run && changed {
		std::fs::write(path, &rewritten)?;
	}

	Ok(outcome(
		applied,
		conflicts,
		rejected,
		!dry_run && changed,
		reverted,
		false,
	))
}

/// Whether every fix's replacement is whitespace, which is what the layout rules produce.
fn whitespace_only(fixes: &[Fix]) -> bool {
	fixes
		.iter()
		.all(|fix| fix.replacement.chars().all(char::is_whitespace))
}

/// Whether every line feed in `source` is preceded by a carriage return.
fn is_pure_crlf(source: &str) -> bool {
	let bytes = source.as_bytes();

	bytes
		.iter()
		.zip(bytes.iter().skip(1))
		.all(|(previous, byte)| *byte != b'\n' || *previous == b'\r')
		&& bytes.first() != Some(&b'\n')
}

/// Re-terminates bare line feeds with carriage returns.
fn normalize_to_crlf(text: &str) -> String {
	let mut normalized = Vec::with_capacity(text.len());
	let mut previous = 0u8;

	for byte in text.bytes() {
		if byte == b'\n' && previous != b'\r' {
			normalized.push(b'\r');
		}

		normalized.push(byte);
		previous = byte;
	}

	// Inserting an ASCII byte into a UTF-8 stream cannot invalidate it, so the lossy fallback is
	// unreachable in practice and only keeps the function total.
	String::from_utf8(normalized)
		.unwrap_or_else(|error| String::from_utf8_lossy(error.as_bytes()).into_owned())
}

/// Whether a whitespace-only rewrite left the code's structure intact.
///
/// Blank lines are invisible to all three comparisons: line kinds are counted over non-blank
/// lines only, the masked skeleton drops whitespace by construction, and literal contents are
/// compared as sorted multisets so a reordering could never mask a change.
fn structure_preserved(source: &str, rewritten: &str, scan: Option<&LexedFile>) -> bool {
	let Some(before) = scan else {
		return true;
	};

	let after = lex(rewritten, before.language);

	if line_kinds(before) != line_kinds(&after) {
		return false;
	}

	if unterminated_counts(before) != unterminated_counts(&after) {
		return false;
	}

	protected_contents(source, before) == protected_contents(rewritten, &after)
}

/// The sorted multiset of kinds over a file's non-blank lines.
fn line_kinds(file: &LexedFile) -> [usize; 4] {
	let mut counts = [0; 4];

	for line in &file.lines {
		match line.kind {
			LineKind::Blank => {}
			LineKind::Comment => counts[0] += 1,
			LineKind::Code => counts[1] += 1,
			LineKind::CodeWithComment => counts[2] += 1,
			LineKind::Literal => counts[3] += 1,
		}
	}

	counts
}

/// The sorted multiset of constructs that were still open at end of file.
fn unterminated_counts(file: &LexedFile) -> [usize; 4] {
	let mut counts = [0; 4];

	for record in &file.unterminated {
		match record.kind {
			monostyle_lexer::UnterminatedKind::BlockComment => counts[0] += 1,
			monostyle_lexer::UnterminatedKind::Literal => counts[1] += 1,
			monostyle_lexer::UnterminatedKind::Heredoc => counts[2] += 1,
			monostyle_lexer::UnterminatedKind::Regex => counts[3] += 1,
		}
	}

	counts
}

/// The sorted raw contents of a file's protected regions, sliced from its own text.
fn protected_contents<'a>(text: &'a str, file: &LexedFile) -> Vec<&'a str> {
	let mut contents: Vec<&str> = file
		.protected
		.iter()
		.filter_map(|range| text.get(range.start_byte..range.end_byte))
		.collect();

	contents.sort_unstable();

	contents
}
