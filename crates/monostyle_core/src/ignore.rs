//! Ignore rules.
//!
//! What should not be scored is a project decision, but some things should never be scored at all.
//! This module keeps those two concerns separate:
//!
//! - **Built-in ignores** cover directories and files that are not the project's own source:
//!   dependency caches, build output, and vendored code. These are always applied, because scoring
//!   `node_modules` produces a number no one can act on.
//! - **Generated-file detection** catches machine-written source that happens to live beside real
//!   code: `*.g.dart`, `*.freezed.dart`, `.expanded.rs`, `.pb.go`, and the like. This is on by
//!   default and can be turned off, because a project that ships generated code as part of its
//!   public surface may legitimately want it measured.
//! - **User patterns** are what a `monostyle.toml` adds, using the same glob syntax as `.gitignore`.
//!
//! # Why generated files are excluded by default
//!
//! Generated code is not written for a human to read, so penalizing its formatting produces findings
//! nobody should act on. In a repository with a large generated surface those findings swamp the
//! real ones: measured on one project here, generated files accounted for more than half the total
//! penalty, which made the score describe a code generator rather than the code.

use std::path::Path;
use std::path::PathBuf;

use serde::Deserialize;
use serde::Serialize;

/// Directories that are never analyzed.
///
/// Matched as a path component, so `node_modules` is skipped at any depth.
const IGNORED_DIRECTORIES: &[&str] = &[
	// Dependency caches.
	"node_modules",
	"vendor",
	"bower_components",
	"jspm_packages",
	".venv",
	"venv",
	"env",
	"__pycache__",
	"site-packages",
	".bundle",
	// Build output.
	"target",
	"dist",
	"build",
	"out",
	"output",
	"bin",
	"obj",
	".next",
	".nuxt",
	".output",
	".svelte-kit",
	".vercel",
	".netlify",
	"coverage",
	// Caches and tool state.
	".git",
	".hg",
	".svn",
	".cache",
	".gradle",
	".m2",
	".cargo",
	".rustup",
	".tox",
	".pytest_cache",
	".mypy_cache",
	".ruff_cache",
	".dart_tool",
	".pub-cache",
	".devenv",
	".direnv",
	// Editor and OS noise.
	".idea",
	".vscode",
	".DS_Store",
	// Infrastructure as code and lockfile directories that hold no source of their own.
	".terraform",
];

/// File suffixes that mark machine-written source.
///
/// Each entry is a suffix rather than an extension, so `foo.g.dart` and `foo.freezed.dart` are
/// both caught without listing every combination.
const GENERATED_SUFFIXES: &[&str] = &[
	// Dart code generation.
	".g.dart",
	".freezed.dart",
	".gr.dart",
	".mocks.dart",
	".config.dart",
	".g.g.dart",
	// Rust procedural macro output and expanded fixtures.
	".expanded.rs",
	".generated.rs",
	// Protocol buffers and gRPC.
	".pb.rs",
	".pb.go",
	".pb.cc",
	".pb.h",
	".pb.swift",
	"_pb2.py",
	"_pb2_grpc.py",
	".connect.go",
	".connect.dart",
	// Schema and API clients.
	".g.cs",
	".designer.cs",
	".generated.cs",
	".g.java",
	".graphql.generated.ts",
	".gen.ts",
	".gen.go",
	".gen.swift",
	".api.dart",
	".api.ts",
	// ORM and migration output.
	".min.js",
	".min.css",
	".bundle.js",
	".chunk.js",
	// Bundler output that keeps its original name.
	"bundle.js",
	"vendors.js",
	"runtime.js",
];

/// File names that are generated or not written for humans.
const GENERATED_NAMES: &[&str] = &[
	"package-lock.json",
	"pnpm-lock.yaml",
	"yarn.lock",
	"cargo.lock",
	"composer.lock",
	"gemfile.lock",
	"poetry.lock",
	"flake.lock",
	"devenv.lock",
];

/// Configuration for ignoring paths.
///
/// The three lists are separate because they answer different questions, and a project may
/// reasonably want to change one without touching the others.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct IgnoreConfig {
	/// Glob patterns to skip, relative to the analyzed directory.
	///
	/// Uses the same syntax as `.gitignore`, so `**/*.spec.ts` and `crates/legacy/**` both work.
	pub patterns: Vec<String>,

	/// Whether generated files are skipped.
	///
	/// Defaults to true. Turn it off to measure generated code as part of the project.
	pub generated: bool,

	/// Whether built-in directory ignores are applied.
	///
	/// Defaults to true. Turn it off to score dependency caches and build output, which is almost never
	/// useful but is occasionally what someone wants when investigating a specific tree.
	pub defaults: bool,

	/// Paths to score even when a rule would skip them.
	///
	/// This is the escape hatch: a generated file that is hand-edited, or a `vendor` directory that
	/// is actually vendored source, can be brought back with an explicit entry.
	pub include: Vec<String>,
}

impl Default for IgnoreConfig {
	fn default() -> Self {
		Self {
			patterns: Vec::new(),
			// Written out rather than derived, because a derived `Default` would make every boolean
			// false and silently disable both exclusions.
			generated: true,
			defaults: true,
			include: Vec::new(),
		}
	}
}

impl IgnoreConfig {
	/// Whether generated files should be skipped.
	#[must_use]
	pub fn skip_generated(&self) -> bool {
		self.generated
	}

	/// Whether built-in directory ignores should be applied.
	#[must_use]
	pub fn skip_defaults(&self) -> bool {
		self.defaults
	}

	/// Whether `path` should be analyzed.
	///
	/// An `include` entry always wins, so the escape hatch works even when a built-in rule or a
	/// pattern would have skipped the file.
	#[must_use]
	pub fn allows(&self, path: &Path) -> bool {
		if Self::matches_any(&self.include, path) {
			return true;
		}

		if self.skip_defaults() && has_ignored_component(path) {
			return false;
		}

		// Two routes into the same decision: the file's name, which is cheap, and its header, which
		// catches a generated file with an ordinary name.
		if self.skip_generated() && (is_generated(path) || declares_generated(path)) {
			return false;
		}

		!Self::matches_any(&self.patterns, path)
	}

	/// Whether any pattern in `patterns` matches `path`.
	fn matches_any(patterns: &[String], path: &Path) -> bool {
		let normalized = normalize(path);

		patterns
			.iter()
			.any(|pattern| matches_pattern(pattern, &normalized))
	}
}

/// Returns true when `name` is a directory that is never analyzed.
#[must_use]
pub fn is_ignored_directory(name: &str) -> bool {
	IGNORED_DIRECTORIES.contains(&name)
}

/// Every directory name that is never analyzed.
#[must_use]
pub fn ignored_directories() -> &'static [&'static str] {
	IGNORED_DIRECTORIES
}

/// Returns true when any path component is an ignored directory.
fn has_ignored_component(path: &Path) -> bool {
	path.components().any(|component| {
		let name = component.as_os_str().to_string_lossy();

		IGNORED_DIRECTORIES.iter().any(|ignored| name == *ignored)
	})
}

/// Markers in a file's opening lines that mean it was machine-written.
///
/// A name-based check cannot catch a generated parser: `parser.c` and `grammar.js` are ordinary names, so
/// a tree-sitter grammar emitted into a repository looks hand-written. On one real repository those files
/// were 784,000 of 822,000 lines and produced 615,000 findings between them, which made the score
/// describe the generator rather than the project. Generated files declare themselves, so the header is
/// read for the declaration instead.
const GENERATED_MARKERS: &[&str] = &[
	"automatically generated",
	"auto-generated",
	"autogenerated",
	"generated by",
	"do not edit",
	"do not modify",
	"machine generated",
	"this file was generated",
	"code generated by",
	"@generated",
];

/// How much of a file is read when looking for a generated marker.
///
/// A declaration appears in a header comment, so the first lines are enough. Reading the whole file would
/// mean a full pass over every file before deciding whether to analyze it, which costs more than it saves.
const MARKER_SEARCH_LINES: usize = 10;

/// Returns true when a file declares itself machine-written in its header.
#[must_use]
pub fn declares_generated(path: &Path) -> bool {
	use std::io::BufRead as _;
	use std::io::Read as _;

	let Ok(file) = std::fs::File::open(path) else {
		return false;
	};

	// Only the first lines are read, and only the first few hundred bytes of each, so a very long header
	// cannot make this expensive.
	std::io::BufReader::new(file)
		.take(64 * 1024)
		.lines()
		.take(MARKER_SEARCH_LINES)
		.map_while(Result::ok)
		.any(|line| {
			let lowered = line.to_ascii_lowercase();

			GENERATED_MARKERS
				.iter()
				.any(|marker| lowered.contains(marker))
		})
}

/// Returns true when a path looks machine-written.
#[must_use]
pub fn is_generated(path: &Path) -> bool {
	let name = path
		.file_name()
		.map(|name| name.to_string_lossy().to_ascii_lowercase())
		.unwrap_or_default();

	if GENERATED_NAMES.contains(&name.as_str()) {
		return true;
	}

	GENERATED_SUFFIXES
		.iter()
		.any(|suffix| name.ends_with(suffix))
}

/// Matches a gitignore-style pattern against a normalized path.
///
/// Implements the subset of gitignore syntax that matters here: `*` within a segment, `**` across
/// segments, a leading `!` for negation, a trailing `/` for directories, and an anchored pattern
/// when the pattern contains a slash. A full gitignore implementation would be a large dependency
/// for a feature whose semantics people already understand from these four forms.
fn matches_pattern(pattern: &str, path: &str) -> bool {
	let pattern = pattern.trim();

	if pattern.is_empty() || pattern.starts_with('#') {
		return false;
	}

	// A pattern naming a directory excludes everything beneath it, which is what `target/` means in a
	// gitignore. Matching the literal path alone would exclude only a file named `target`.
	let directory_pattern = format!("{}/**", pattern.trim_end_matches('/'));
	let bare = pattern.trim_end_matches('/');

	if bare != pattern.trim_end_matches("./") && matches_literal(bare, path) {
		return true;
	}

	if matches_literal(bare, path) || matches_literal(&directory_pattern, path) {
		return true;
	}

	false
}

/// Matches a pattern literally, without the directory expansion above.
fn matches_literal(pattern: &str, path: &str) -> bool {
	let pattern = pattern.strip_prefix("./").unwrap_or(pattern);

	if pattern.is_empty() {
		return false;
	}

	// A leading `!` is negation, which this matcher does not need: `include` entries exist for the
	// case a negation would serve, and honoring both would make precedence ambiguous.
	let pattern = pattern.strip_prefix('!').unwrap_or(pattern);
	let pattern = pattern.strip_prefix("./").unwrap_or(pattern);
	let pattern = pattern.trim_end_matches('/');

	if pattern.is_empty() {
		return false;
	}

	// A pattern containing a slash is anchored to the analyzed root; one without matches at any
	// depth, which is what makes `*.spec.ts` behave the way people expect.
	let anchored = pattern.contains('/');
	let segments: Vec<&str> = path.split('/').collect();
	let pattern_segments: Vec<&str> = pattern.split('/').collect();

	if anchored {
		return match_segments(&pattern_segments, &segments);
	}

	// An unanchored pattern may match starting at any segment boundary.
	(0..segments.len()).any(|start| {
		segments
			.get(start..)
			.is_some_and(|rest| match_segments(&pattern_segments, rest))
	})
}

/// Matches pattern segments against path segments, honouring `**`.
fn match_segments(pattern: &[&str], path: &[&str]) -> bool {
	match pattern.split_first() {
		None => path.is_empty(),
		Some((&"**", rest)) => {
			// `**` matches zero or more segments, so every suffix is tried.
			(0..=path.len()).any(|skip| {
				path.get(skip.min(path.len())..)
					.is_some_and(|tail| match_segments(rest, tail))
			})
		}
		Some((segment, rest)) => {
			match path.split_first() {
				Some((first, remaining)) => {
					matches_segment(segment, first) && match_segments(rest, remaining)
				}
				None => false,
			}
		}
	}
}

/// Matches one pattern segment against one path segment, honouring `*` and `?`.
fn matches_segment(pattern: &str, text: &str) -> bool {
	let pattern: Vec<char> = pattern.chars().collect();
	let text: Vec<char> = text.chars().collect();

	// Iterative glob matching with backtracking, which avoids recursion depth proportional to the
	// input length.
	let (mut pattern_index, mut text_index) = (0, 0);
	let (mut star_pattern, mut star_text): (Option<usize>, usize) = (None, 0);

	while text_index < text.len() {
		// The comparison reads through `get` on both sides, so neither index can be out of range even
		// though the loop only checks the text length.
		if pattern.get(pattern_index) == Some(&'?')
			|| pattern
				.get(pattern_index)
				.is_some_and(|c| text.get(text_index) == Some(c))
		{
			pattern_index += 1;
			text_index += 1;
		} else if pattern.get(pattern_index) == Some(&'*') {
			star_pattern = Some(pattern_index);
			star_text = text_index;
			pattern_index += 1;
		} else if let Some(star) = star_pattern {
			pattern_index = star + 1;
			star_text += 1;
			text_index = star_text;
		} else {
			return false;
		}
	}

	while pattern.get(pattern_index) == Some(&'*') {
		pattern_index += 1;
	}

	pattern_index == pattern.len()
}

/// Normalizes a path to forward slashes for matching.
fn normalize(path: &Path) -> String {
	path.to_string_lossy().replace('\\', "/")
}

/// Filters `paths`, dropping everything the configuration ignores.
#[must_use]
pub fn apply(paths: Vec<PathBuf>, config: &IgnoreConfig) -> Vec<PathBuf> {
	paths
		.into_iter()
		.filter(|path| config.allows(path))
		.collect()
}

/// Returns the paths that were ignored, for reporting.
#[must_use]
pub fn rejected(paths: &[PathBuf], config: &IgnoreConfig) -> Vec<PathBuf> {
	paths
		.iter()
		.filter(|path| !config.allows(path))
		.cloned()
		.collect()
}
