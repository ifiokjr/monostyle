//! Tests for the analysis cache.
//!
//! The cache is an optimization with a correctness requirement: a hit must never report scores for code
//! that has changed. These tests cover invalidation, corruption, and the discovery of where to write.

use std::path::Path;

use monostyle::cache::Cache;
use monostyle_core::Language;
use monostyle_lexer::lex;

/// Writes a file and returns its path, kept alive by the returned directory.
fn source(contents: &str) -> (tempfile::TempDir, std::path::PathBuf) {
	let directory = tempfile::tempdir().expect("a temporary directory");
	let path = directory.path().join("sample.rs");

	std::fs::write(&path, contents).expect("write");

	(directory, path)
}

/// Creates a cache rooted in `path`.
fn cache_in(path: &Path) -> Cache {
	Cache::new(path.join("cache"))
}

#[test]
fn a_stored_entry_is_returned() {
	let (directory, path) = source("fn a() {}\n");
	let cache = cache_in(directory.path());

	let lexed = lex("fn a() {}\n", Language::Rust);
	cache.put(&path, Language::Rust, &lexed.lines);

	let restored = cache.get(&path, Language::Rust);

	assert!(restored.is_some(), "the entry should be found");
	assert_eq!(restored.unwrap().len(), lexed.lines.len());
}

#[test]
fn a_changed_file_misses() {
	// A stale hit would report scores for code that no longer exists, which is the one failure a cache
	// must not have.
	let (directory, path) = source("fn a() {}\n");
	let cache = cache_in(directory.path());

	let lexed = lex("fn a() {}\n", Language::Rust);
	cache.put(&path, Language::Rust, &lexed.lines);

	// The modification time has second granularity on some filesystems, so the size change is what this
	// test relies on.
	std::fs::write(&path, "fn a() { work(); }\n").expect("rewrite");

	assert!(
		cache.get(&path, Language::Rust).is_none(),
		"a changed file must miss"
	);
}

#[test]
fn a_different_language_misses() {
	// The same bytes read as Python and as Rust produce different line models, so the language is part
	// of the key.
	let (directory, path) = source("fn a() {}\n");
	let cache = cache_in(directory.path());

	let lexed = lex("fn a() {}\n", Language::Rust);
	cache.put(&path, Language::Rust, &lexed.lines);

	assert!(cache.get(&path, Language::Python).is_none());
}

#[test]
fn a_missing_file_misses() {
	let directory = tempfile::tempdir().expect("a temporary directory");
	let cache = cache_in(directory.path());

	assert!(
		cache
			.get(Path::new("/nonexistent/sample.rs"), Language::Rust)
			.is_none()
	);
}

#[test]
fn entries_survive_a_new_cache_instance() {
	// The point of the cache is to survive across runs, so a second instance reading the same directory
	// must find what the first wrote.
	let (directory, path) = source("fn a() {}\n");
	let written = {
		let cache = cache_in(directory.path());
		let lexed = lex("fn a() {}\n", Language::Rust);
		cache.put(&path, Language::Rust, &lexed.lines);

		lexed.lines.len()
	};

	let cache = cache_in(directory.path());
	let restored = cache
		.get(&path, Language::Rust)
		.expect("the entry should persist");

	assert_eq!(restored.len(), written);
}

#[test]
fn statistics_track_hits_and_misses() {
	let (directory, path) = source("fn a() {}\n");
	let cache = cache_in(directory.path());

	let lexed = lex("fn a() {}\n", Language::Rust);
	cache.put(&path, Language::Rust, &lexed.lines);

	let _ = cache.get(&path, Language::Rust);
	let _ = cache.get(Path::new("/nonexistent.rs"), Language::Rust);

	let (hits, misses) = cache.stats();

	assert!(hits >= 1, "a stored entry should count as a hit");
	assert!(misses >= 1, "a missing file should count as a miss");
}

#[test]
fn clearing_removes_every_entry() {
	let (directory, path) = source("fn a() {}\n");
	let cache = cache_in(directory.path());

	let lexed = lex("fn a() {}\n", Language::Rust);
	cache.put(&path, Language::Rust, &lexed.lines);

	cache.clear().expect("clear should succeed");

	// The in-memory map is cleared too, so a clear is immediately visible.
	assert!(cache.get(&path, Language::Rust).is_none());
}

#[test]
fn a_corrupt_entry_is_discarded_rather_than_reported() {
	// A cache is an optimization, so a damaged file should cause a re-scan rather than a failure.
	let (directory, path) = source("fn a() {}\n");
	let cache = cache_in(directory.path());

	let lexed = lex("fn a() {}\n", Language::Rust);
	cache.put(&path, Language::Rust, &lexed.lines);

	// Overwrite every entry with nonsense.
	for entry in std::fs::read_dir(directory.path().join("cache")).expect("read") {
		let entry = entry.expect("entry");
		for shard in std::fs::read_dir(entry.path()).expect("read shard") {
			std::fs::write(shard.expect("file").path(), b"not a cache entry").expect("write");
		}
	}

	// A fresh instance has no in-memory state, so it reads the corrupt file.
	let fresh = cache_in(directory.path());

	assert!(
		fresh.get(&path, Language::Rust).is_none(),
		"a corrupt entry should miss"
	);
}

#[test]
fn discovery_finds_a_target_directory_upward() {
	let directory = tempfile::tempdir().expect("a temporary directory");
	let nested = directory.path().join("a/b/c");

	std::fs::create_dir_all(&nested).expect("mkdir");
	std::fs::create_dir_all(directory.path().join("target")).expect("mkdir");

	let found = Cache::discover(&nested);

	assert!(
		found.is_some(),
		"the walk should find the target directory above"
	);
}

#[test]
fn discovery_returns_nothing_without_a_target_directory() {
	let directory = tempfile::tempdir().expect("a temporary directory");

	// A path with no target directory anywhere above it, which is the case outside a build tree.
	assert!(Cache::discover(&directory.path().join("nothing")).is_none());
}

#[test]
fn an_unchanged_file_hits_twice() {
	// The in-memory map exists so a second lookup in one run costs nothing.
	let (directory, path) = source("fn a() {}\n");
	let cache = cache_in(directory.path());

	let lexed = lex("fn a() {}\n", Language::Rust);
	cache.put(&path, Language::Rust, &lexed.lines);

	assert!(cache.get(&path, Language::Rust).is_some());
	assert!(cache.get(&path, Language::Rust).is_some());
	assert!(cache.get(&path, Language::Rust).is_some());
}

#[test]
fn every_line_field_round_trips() {
	// A field that is dropped in the cache would change a score between runs without changing the file.
	let (directory, path) =
		source("// A comment explaining why.\nfn a() {\n    if x {\n        work();\n    }\n}\n");
	let cache = cache_in(directory.path());

	let original = lex(
		"// A comment explaining why.\nfn a() {\n    if x {\n        work();\n    }\n}\n",
		Language::Rust,
	);
	cache.put(&path, Language::Rust, &original.lines);

	let restored = cache.get(&path, Language::Rust).expect("a hit");
	let fresh = Cache::new(directory.path().join("fresh"));

	// The restored lines must compare equal on every field the rules read.
	fresh.put(&path, Language::Rust, &restored);
	let twice = fresh.get(&path, Language::Rust).expect("a second hit");

	for (left, right) in original.lines.iter().zip(twice.iter()) {
		assert_eq!(left.number, right.number);
		assert_eq!(left.text, right.text);
		assert_eq!(left.masked_code, right.masked_code);
		assert_eq!(left.indent, right.indent);
		assert_eq!(left.kind, right.kind);
		assert_eq!(left.start_byte, right.start_byte);
		assert_eq!(left.end_byte, right.end_byte);
		assert_eq!(left.decisions, right.decisions);
		assert_eq!(left.nesting, right.nesting);
		assert_eq!(left.logical_operators, right.logical_operators);
		assert_eq!(left.is_return, right.is_return);
		assert_eq!(left.comment_intent, right.comment_intent);
		assert_eq!(left.parameter_span, right.parameter_span);
	}
}
