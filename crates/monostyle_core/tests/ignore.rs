//! Tests for the ignore system.
//!
//! Ignoring is where a tool is most likely to be quietly wrong: a filter that skips one file too many
//! produces a score that looks fine and describes the wrong code. These tests cover every filter
//! independently and the precedence between them.

use std::path::Path;
use std::path::PathBuf;

use monostyle_core::ignore::IgnoreConfig;
use monostyle_core::ignore::is_generated;
use monostyle_core::ignore::is_ignored_directory;

/// Builds a configuration with the given patterns and includes.
fn config(patterns: &[&str], include: &[&str]) -> IgnoreConfig {
	IgnoreConfig {
		patterns: patterns
			.iter()
			.map(|pattern| (*pattern).to_string())
			.collect(),
		include: include
			.iter()
			.map(|pattern| (*pattern).to_string())
			.collect(),

		..IgnoreConfig::default()
	}
}

// ---------------------------------------------------------------------------
// Defaults
// ---------------------------------------------------------------------------

#[test]
fn dependency_caches_are_ignored_at_any_depth() {
	for directory in [
		"node_modules",
		"vendor",
		".venv",
		"__pycache__",
		".dart_tool",
	] {
		assert!(
			is_ignored_directory(directory),
			"{directory} should be ignored"
		);

		let nested = format!("a/b/{directory}/c/file.rs");

		assert!(
			!IgnoreConfig::default().allows(Path::new(&nested)),
			"{nested} should be ignored at any depth"
		);
	}
}

#[test]
fn build_output_is_ignored() {
	for directory in ["target", "dist", "build", "out", "coverage", ".next"] {
		assert!(
			is_ignored_directory(directory),
			"{directory} should be ignored"
		);
	}
}

#[test]
fn tool_state_is_ignored() {
	for directory in [
		".git",
		".gradle",
		".cache",
		".devenv",
		".direnv",
		".terraform",
	] {
		assert!(
			is_ignored_directory(directory),
			"{directory} should be ignored"
		);
	}
}

#[test]
fn ordinary_source_paths_are_allowed() {
	let config = IgnoreConfig::default();

	for path in [
		"src/main.rs",
		"crates/core/src/lib.rs",
		"lib/model.dart",
		"packages/app/index.ts",
	] {
		assert!(config.allows(Path::new(path)), "{path} should be analyzed");
	}
}

#[test]
fn defaults_can_be_disabled() {
	let config = IgnoreConfig {
		defaults: false,

		..IgnoreConfig::default()
	};

	assert!(
		config.allows(Path::new("node_modules/pkg/index.js")),
		"turning defaults off should allow a dependency cache"
	);
}

// ---------------------------------------------------------------------------
// Generated detection
// ---------------------------------------------------------------------------

#[test]
fn dart_codegen_suffixes_are_recognized() {
	for path in [
		"model.g.dart",
		"model.freezed.dart",
		"model.gr.dart",
		"model.mocks.dart",
	] {
		assert!(
			is_generated(Path::new(path)),
			"{path} should look generated"
		);
	}
}

#[test]
fn protobuf_and_grpc_output_is_recognized() {
	for path in [
		"schema.pb.rs",
		"api.pb.go",
		"service.pb.cc",
		"service.pb.h",
		"service.pb.swift",
		"types_pb2.py",
		"types_pb2_grpc.py",
	] {
		assert!(
			is_generated(Path::new(path)),
			"{path} should look generated"
		);
	}
}

#[test]
fn lock_files_are_recognized() {
	for path in [
		"Cargo.lock",
		"pnpm-lock.yaml",
		"yarn.lock",
		"package-lock.json",
		"flake.lock",
		"devenv.lock",
	] {
		assert!(is_generated(Path::new(path)), "{path} should be skipped");
	}
}

#[test]
fn minified_assets_are_recognized() {
	for path in ["app.min.js", "styles.min.css", "bundle.js", "main.chunk.js"] {
		assert!(
			is_generated(Path::new(path)),
			"{path} should look generated"
		);
	}
}

#[test]
fn hand_written_source_is_not_generated() {
	for path in ["main.rs", "model.dart", "index.ts", "app.py", "server.go"] {
		assert!(!is_generated(Path::new(path)), "{path} is hand-written");
	}
}

#[test]
fn generated_detection_is_case_insensitive() {
	assert!(is_generated(Path::new("Model.G.Dart")));
	assert!(is_generated(Path::new("CARGO.LOCK")));
}

#[test]
fn generated_files_can_be_included() {
	let config = IgnoreConfig {
		generated: false,

		..IgnoreConfig::default()
	};

	assert!(config.allows(Path::new("lib/model.g.dart")));
}

// ---------------------------------------------------------------------------
// Patterns
// ---------------------------------------------------------------------------

#[test]
fn a_bare_pattern_matches_at_any_depth() {
	let config = config(&["*.spec.ts"], &[]);

	assert!(!config.allows(Path::new("app.spec.ts")));
	assert!(!config.allows(Path::new("src/deep/app.spec.ts")));
	assert!(config.allows(Path::new("src/app.ts")));
}

#[test]
fn an_anchored_pattern_matches_from_the_root() {
	let config = config(&["src/legacy/**"], &[]);

	assert!(!config.allows(Path::new("src/legacy/old.rs")));
	assert!(!config.allows(Path::new("src/legacy/deep/old.rs")));
	assert!(config.allows(Path::new("other/src/legacy/old.rs")));
}

#[test]
fn a_double_star_matches_any_number_of_segments() {
	let config = config(&["**/generated/*.rs"], &[]);

	assert!(!config.allows(Path::new("generated/a.rs")));
	assert!(!config.allows(Path::new("a/b/generated/a.rs")));
	assert!(config.allows(Path::new("a/b/other/a.rs")));
}

#[test]
fn a_single_star_matches_within_one_segment() {
	let config = config(&["src/*.rs"], &[]);

	assert!(!config.allows(Path::new("src/main.rs")));
	assert!(
		config.allows(Path::new("src/deep/main.rs")),
		"`*` should not cross a separator"
	);
}

#[test]
fn a_question_mark_matches_one_character() {
	let config = config(&["test?.rs"], &[]);

	assert!(!config.allows(Path::new("test1.rs")));
	assert!(config.allows(Path::new("test12.rs")));
}

#[test]
fn a_trailing_slash_is_ignored() {
	let config = config(&["legacy/"], &[]);

	assert!(!config.allows(Path::new("legacy/old.rs")));
}

#[test]
fn a_leading_dot_slash_is_ignored() {
	let config = config(&["./src/old.rs"], &[]);

	assert!(!config.allows(Path::new("src/old.rs")));
}

#[test]
fn empty_and_comment_patterns_are_skipped() {
	let config = config(&["", "  ", "# a comment"], &[]);

	assert!(config.allows(Path::new("src/main.rs")));
}

#[test]
fn multiple_patterns_are_all_applied() {
	let config = config(&["*.spec.ts", "legacy/**", "**/*.tmp.rs"], &[]);

	assert!(!config.allows(Path::new("a.spec.ts")));
	assert!(!config.allows(Path::new("legacy/a.rs")));
	assert!(!config.allows(Path::new("deep/a.tmp.rs")));
	assert!(config.allows(Path::new("src/main.rs")));
}

// ---------------------------------------------------------------------------
// Precedence
// ---------------------------------------------------------------------------

#[test]
fn an_include_entry_beats_a_pattern() {
	let config = config(&["src/**"], &["src/keep.rs"]);

	assert!(config.allows(Path::new("src/keep.rs")));
	assert!(!config.allows(Path::new("src/other.rs")));
}

#[test]
fn an_include_entry_beats_a_default_directory() {
	// The escape hatch has to work against the built-in filters, or a project with a genuinely
	// vendored directory cannot bring it back.
	let config = config(&[], &["vendor/ours/**"]);

	assert!(config.allows(Path::new("vendor/ours/lib.rs")));
	assert!(!config.allows(Path::new("vendor/theirs/lib.rs")));
}

#[test]
fn an_include_entry_beats_generated_detection() {
	let config = config(&[], &["lib/model.g.dart"]);

	assert!(config.allows(Path::new("lib/model.g.dart")));
	assert!(!config.allows(Path::new("lib/other.g.dart")));
}

// ---------------------------------------------------------------------------
// Bulk helpers
// ---------------------------------------------------------------------------

#[test]
fn apply_filters_a_list_of_paths() {
	let paths = vec![
		PathBuf::from("src/main.rs"),
		PathBuf::from("node_modules/pkg/index.js"),
		PathBuf::from("lib/model.g.dart"),
	];

	let kept = monostyle_core::ignore::apply(paths.clone(), &IgnoreConfig::default());

	assert_eq!(kept, vec![PathBuf::from("src/main.rs")]);
}

#[test]
fn rejected_reports_what_was_skipped() {
	let paths = vec![
		PathBuf::from("src/main.rs"),
		PathBuf::from("target/build.rs"),
	];

	let skipped = monostyle_core::ignore::rejected(&paths, &IgnoreConfig::default());

	assert_eq!(skipped, vec![PathBuf::from("target/build.rs")]);
}

#[test]
fn windows_style_separators_are_normalized() {
	let config = config(&["src/legacy/**"], &[]);

	assert!(
		!config.allows(Path::new(r"src\legacy\old.rs")),
		"a backslash path should match a forward-slash pattern"
	);
}
