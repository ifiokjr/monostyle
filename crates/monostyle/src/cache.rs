//! Analysis cache.
//!
//! # What is cached, and why
//!
//! Lexing is the expensive half of an analysis: for each file the scanner walks every character and
//! builds a line model. A repeated run over an unchanged tree therefore repeats identical work.
//!
//! The cache stores the *lexed* result rather than the final report, because rules and thresholds
//! change far more often than source does. Caching the report would invalidate on every
//! configuration change; caching the line model stays valid across all of them, and only the cheap
//! rule pass repeats.
//!
//! # Keying
//!
//! A file is keyed by its path, its modification time, its size, and a hash of the configuration
//! that affects the scanner — chiefly the language profile, since a profile change alters how the
//! same bytes are read. Modification time and size are enough to detect an edit without reading the
//! file, which is what makes a cache hit cheaper than the work it replaces.
//!
//! # Where it lives and how it is invalidated
//!
//! Entries live under the target directory so they are cleaned by the tooling that already cleans
//! build output, and are versioned by a schema number so a format change cannot be misread as a
//! valid old entry. A corrupt entry is dropped rather than reported: a cache is an optimization, and
//! failing an analysis because an optimization went wrong would be the wrong trade.

use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Mutex;

use monostyle_core::Language;
use monostyle_lexer::LexedLine;
use monostyle_lexer::LineKind;
use serde::Deserialize;
use serde::Serialize;

/// Bumped whenever the cached shape changes.
///
/// An entry written by an older schema is discarded rather than migrated, because a partially valid
/// cache is harder to reason about than an empty one.
const SCHEMA: u32 = 1;

/// A cached line, holding only what rules read.
///
/// The full `LexedLine` is stored rather than a reduced projection because every field is consumed by
/// at least one rule, and a projection would need updating whenever a rule changed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedLine {
	/// 1-based line number.
	pub number: usize,
	/// The raw line text.
	pub text: String,
	/// The line with literal and comment contents blanked.
	pub masked_code: String,
	/// Byte offset of the line's first character.
	pub start_byte: usize,
	/// Byte offset one past the line's last character.
	pub end_byte: usize,
	/// Lines of leading whitespace, with tabs expanded.
	pub indent: usize,
	/// The raw leading whitespace.
	pub indent_text: String,
	/// The line's kind, as an index into [`LineKind`].
	pub kind: u8,
	/// Column where code begins.
	pub code_start_column: usize,
	/// Column where a trailing comment begins.
	pub trailing_comment_column: Option<usize>,
	/// The comment's intent, as an index into `CommentIntent`.
	pub comment_intent: Option<u8>,
	/// Decision keywords on this line.
	pub decisions: Vec<String>,
	/// Nesting keywords on this line.
	pub nesting: Vec<String>,
	/// Short-circuiting operators on this line.
	pub logical_operators: usize,
	/// Null-coalescing operators on this line.
	pub null_coalescing: usize,
	/// Whether a ternary appears.
	pub has_ternary: bool,
	/// Jump statements on this line.
	pub jumps: usize,
	/// Lines spanned by a parameter list opened here.
	pub parameter_span: usize,
	/// Whether the line exits its function.
	pub is_return: bool,
}

/// Everything cached for one file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedFile {
	/// The schema this entry was written with.
	pub schema: u32,
	/// The language the file was analyzed as.
	pub language: Language,
	/// The file's modification time in nanoseconds since the epoch.
	pub modified_nanos: u128,
	/// The file's size in bytes.
	pub size: u64,
	/// The analysis lines.
	pub lines: Vec<CachedLine>,
}

/// A handle to the on-disk cache.
pub struct Cache {
	/// Where entries are stored.
	directory: PathBuf,
	/// In-memory entries, so a single run does not read the same file twice.
	memory: Mutex<HashMap<PathBuf, Option<CachedFile>>>,
	/// How many lookups hit.
	hits: Mutex<usize>,
	/// How many lookups missed.
	misses: Mutex<usize>,
}

impl Cache {
	/// Creates a cache rooted at `directory`.
	///
	/// The directory is created lazily on the first write, so a read-only run never touches the
	/// filesystem.
	#[must_use]
	pub fn new(directory: PathBuf) -> Self {
		Self {
			directory,
			memory: Mutex::new(HashMap::new()),
			hits: Mutex::new(0),
			misses: Mutex::new(0),
		}
	}

	/// Locates a cache under the standard target directory.
	///
	/// Returns `None` when no target directory exists, which is the case for an analysis run outside
	/// a build tree; the caller then proceeds without a cache rather than creating one somewhere
	/// unexpected.
	#[must_use]
	pub fn discover(root: &Path) -> Option<Self> {
		// Started from the analyzed *root* rather than from a file inside it, so a project with a
		// build directory at its top level is found. Starting from a file's parent would only find a
		// target directory when the file itself happened to live under one.
		let mut directory = root.to_path_buf();

		loop {
			let candidate = directory.join("target");

			if candidate.is_dir() {
				return Some(Self::new(candidate.join("monostyle")));
			}

			if !directory.pop() {
				return None;
			}
		}
	}

	/// Retrieves a cached analysis when it is still valid.
	///
	/// Validity requires a matching schema, language, modification time, and size. Any mismatch is a
	/// miss, which is the safe direction: a stale hit would report scores for code that no longer
	/// exists.
	#[must_use]
	pub fn get(&self, path: &Path, language: Language) -> Option<Vec<LexedLine>> {
		// A file that cannot be read is a miss, and it is recorded as one. Returning early without
		// counting made the statistics undercount, so the hit rate a caller saw did not describe the run.
		let Ok(metadata) = std::fs::metadata(path) else {
			self.record_miss();

			return None;
		};

		let Some(modified) = modified_nanos(&metadata) else {
			self.record_miss();

			return None;
		};

		// The in-memory map is consulted first so a second lookup in one run costs nothing.
		if let Ok(memory) = self.memory.lock()
			&& let Some(entry) = memory.get(path)
		{
			return match entry {
				Some(entry) if entry.is_valid(language, modified, metadata.len()) => {
					self.record_hit();

					Some(entry.to_lines())
				}
				_ => {
					self.record_miss();

					None
				}
			};
		}

		let entry = self.read_entry(path);
		let valid = entry
			.as_ref()
			.is_some_and(|entry| entry.is_valid(language, modified, metadata.len()));

		// The entry is cloned into the in-memory map so the original can still be converted below.
		// The clone is one per file per run, which is far cheaper than a second disk read.
		if let Ok(mut memory) = self.memory.lock() {
			memory.insert(path.to_path_buf(), entry.clone());
		}

		if valid {
			self.record_hit();

			entry.map(|entry| entry.to_lines())
		} else {
			self.record_miss();

			None
		}
	}

	/// Stores an analysis result.
	pub fn put(&self, path: &Path, language: Language, lines: &[LexedLine]) {
		let Ok(metadata) = std::fs::metadata(path) else {
			return;
		};

		let Some(modified) = modified_nanos(&metadata) else {
			return;
		};

		let entry = CachedFile {
			schema: SCHEMA,
			language,
			modified_nanos: modified,
			size: metadata.len(),
			lines: lines.iter().map(CachedLine::from_line).collect(),
		};

		if let Ok(mut memory) = self.memory.lock() {
			memory.insert(path.to_path_buf(), Some(entry.clone()));
		}

		// A write failure is ignored: the cache is an optimization, and a read-only or full disk
		// should not fail an analysis.
		let entry_path = self.entry_path(path);

		if let Some(parent) = entry_path.parent() {
			let _ = std::fs::create_dir_all(parent);
		}

		if let Ok(encoded) = encode_entry(&entry) {
			let _ = std::fs::write(entry_path, encoded);
		}
	}

	/// Returns how many lookups hit and missed.
	#[must_use]
	pub fn stats(&self) -> (usize, usize) {
		let hits = self.hits.lock().map_or(0, |hits| *hits);
		let misses = self.misses.lock().map_or(0, |misses| *misses);

		(hits, misses)
	}

	/// Removes every entry.
	pub fn clear(&self) -> std::io::Result<()> {
		if let Ok(mut memory) = self.memory.lock() {
			memory.clear();
		}

		if self.directory.is_dir() {
			std::fs::remove_dir_all(&self.directory)?;
		}

		Ok(())
	}

	/// Reads an entry from disk.
	fn read_entry(&self, path: &Path) -> Option<CachedFile> {
		let bytes = std::fs::read(self.entry_path(path)).ok()?;

		decode_entry(&bytes)
	}

	/// Returns the on-disk location for a file's entry.
	///
	/// The path is hashed rather than mirrored, because a deep tree would otherwise produce very long
	/// cache paths and exceed the platform's path limit. Sharding by the low bits keeps a directory from
	/// holding hundreds of thousands of entries.
	fn entry_path(&self, path: &Path) -> PathBuf {
		let hash = fnv1a(path.to_string_lossy().as_bytes());
		let shard = hash % 256;

		self.directory
			.join(format!("{shard:02x}"))
			.join(format!("{hash:016x}.mcache"))
	}

	/// Records a hit.
	fn record_hit(&self) {
		if let Ok(mut hits) = self.hits.lock() {
			*hits += 1;
		}
	}

	/// Records a miss.
	fn record_miss(&self) {
		if let Ok(mut misses) = self.misses.lock() {
			*misses += 1;
		}
	}
}

impl CachedFile {
	/// Whether this entry still describes the file.
	fn is_valid(&self, language: Language, modified: u128, size: u64) -> bool {
		self.schema == SCHEMA
			&& self.language == language
			&& self.modified_nanos == modified
			&& self.size == size
	}

	/// Converts the cached lines back into the analysis model.
	fn to_lines(&self) -> Vec<LexedLine> {
		self.lines.iter().map(CachedLine::to_line).collect()
	}
}

impl CachedLine {
	/// Projects a lexed line into its cached form.
	fn from_line(line: &LexedLine) -> Self {
		Self {
			number: line.number,
			text: line.text.clone(),
			masked_code: line.masked_code.clone(),
			start_byte: line.start_byte,
			end_byte: line.end_byte,
			indent: line.indent,
			indent_text: line.indent_text.clone(),
			kind: line.kind as u8,
			code_start_column: line.code_start_column,
			trailing_comment_column: line.trailing_comment_column,
			comment_intent: line.comment_intent.map(|intent| intent as u8),
			decisions: line.decisions.clone(),
			nesting: line.nesting.clone(),
			logical_operators: line.logical_operators,
			null_coalescing: line.null_coalescing,
			has_ternary: line.has_ternary,
			jumps: line.jumps,
			parameter_span: line.parameter_span,
			is_return: line.is_return,
		}
	}

	/// Rebuilds a lexed line.
	fn to_line(&self) -> LexedLine {
		LexedLine {
			number: self.number,
			text: self.text.clone(),
			masked_code: self.masked_code.clone(),
			start_byte: self.start_byte,
			end_byte: self.end_byte,
			indent: self.indent,
			indent_text: self.indent_text.clone(),
			kind: match self.kind {
				0 => LineKind::Blank,
				1 => LineKind::Comment,
				2 => LineKind::Code,
				3 => LineKind::CodeWithComment,
				_ => LineKind::Literal,
			},
			code_start_column: self.code_start_column,
			trailing_comment_column: self.trailing_comment_column,
			comment_intent: self.comment_intent.and_then(intent_from_index),
			decisions: self.decisions.clone(),
			nesting: self.nesting.clone(),
			logical_operators: self.logical_operators,
			null_coalescing: self.null_coalescing,
			has_ternary: self.has_ternary,
			jumps: self.jumps,
			parameter_span: self.parameter_span,
			is_return: self.is_return,
		}
	}
}

/// Maps a cached index back to a comment intent.
fn intent_from_index(index: u8) -> Option<monostyle_lexer::CommentIntent> {
	use monostyle_lexer::CommentIntent;

	Some(match index {
		0 => CommentIntent::Why,
		1 => CommentIntent::How,
		2 => CommentIntent::Documentation,
		3 => CommentIntent::Directive,
		4 => CommentIntent::Neutral,
		_ => return None,
	})
}

/// Returns a file's modification time in nanoseconds.
fn modified_nanos(metadata: &std::fs::Metadata) -> Option<u128> {
	metadata
		.modified()
		.ok()
		.and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
		.map(|duration| duration.as_nanos())
}

/// Hashes bytes with FNV-1a.
///
/// A cache filename needs a stable, well-distributed hash and nothing more. Using a cryptographic
/// digest would add a dependency for a value whose only job is to name a file.
fn fnv1a(bytes: &[u8]) -> u64 {
	let mut hash: u64 = 0xcbf2_9ce4_8422_2325;

	for byte in bytes {
		hash ^= u64::from(*byte);
		hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
	}

	hash
}

/// Encodes a cache entry.
///
/// JSON rather than a compact binary format: an entry is written once per changed file and read once
/// per run, which is not a volume where the encoding matters, and using the serializer already in the
/// dependency graph avoids a second one whose only job would be a cache file.
fn encode_entry(entry: &CachedFile) -> Result<Vec<u8>, ()> {
	serde_json::to_vec(entry).map_err(|_| ())
}

/// Decodes a cache entry, returning `None` when it is unreadable.
///
/// A corrupt entry is discarded rather than reported, because a cache is an optimization and failing
/// an analysis because one went wrong would be the wrong trade.
fn decode_entry(bytes: &[u8]) -> Option<CachedFile> {
	serde_json::from_slice(bytes).ok()
}
