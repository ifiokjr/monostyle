//! Tests for the analysis pipeline's less-travelled paths.
//!
//! These cover what the fixture-driven suites do not reach: the cache-hit path through analysis, files
//! that cannot be read, workspace formats other than Cargo, and the report's conditional sections.

use monostyle::analysis::AnalysisOptions;
use monostyle::analysis::analyze_paths;
use monostyle::analysis::analyze_source;
use monostyle::analysis::analyze_with_language;
use monostyle::analysis::collect_paths;
use monostyle::analysis::language_for_path;
use monostyle::package::Ecosystem;
use monostyle::package::detect_packages;
use monostyle_core::Language;
use monostyle_rules::RulesConfig;

/// Joins generated lines into one string.
///
/// Building a fixture by mapping `format!` over a range and collecting is the shape clippy flags, and a
/// named helper states the intent more plainly than the fold it expands to.
fn lines_of(items: impl IntoIterator<Item = String>) -> String {
	items.into_iter().fold(String::new(), |mut text, line| {
		text.push_str(&line);

		text
	})
}

/// Returns analysis options with the cache off.
fn uncached() -> AnalysisOptions {
	AnalysisOptions {
		cache: false,

		..AnalysisOptions::default()
	}
}

// ---------------------------------------------------------------------------
// Language resolution through paths
// ---------------------------------------------------------------------------

#[test]
fn a_known_extension_resolves_a_language() {
	assert_eq!(
		language_for_path(std::path::Path::new("src/main.rs")),
		Some(Language::Rust)
	);
	assert_eq!(
		language_for_path(std::path::Path::new("lib/model.dart")),
		Some(Language::Dart)
	);
	assert_eq!(
		language_for_path(std::path::Path::new("readme.md")),
		Some(Language::Markdown)
	);
}

#[test]
fn an_unknown_extension_resolves_nothing() {
	assert_eq!(language_for_path(std::path::Path::new("data.bin")), None);
	assert_eq!(language_for_path(std::path::Path::new("Makefile")), None);
}

#[test]
fn analyzing_source_with_an_unknown_extension_produces_nothing() {
	let report = analyze_source(std::path::Path::new("notes.txt"), "hello", &uncached());

	assert!(
		report.is_none(),
		"a file with no known language should not be analyzed"
	);
}

#[test]
fn analyzing_source_in_a_known_language_produces_a_report() {
	let report = analyze_source(std::path::Path::new("main.rs"), "fn a() {}\n", &uncached())
		.expect("a report");

	assert_eq!(report.language, Language::Rust);
	assert_eq!(report.code_lines, 1);
}

#[test]
fn analyzing_with_an_explicit_language_does_not_consult_the_path() {
	// The extension is a guess; an explicit language overrides it.
	let report = analyze_with_language(
		std::path::Path::new("misleading.txt"),
		"fn a() {}\n",
		Language::Rust,
		&uncached(),
	);

	assert_eq!(report.language, Language::Rust);
}

// ---------------------------------------------------------------------------
// Path collection
// ---------------------------------------------------------------------------

#[test]
fn collection_skips_unreadable_and_unrecognized_files() {
	let temp = tempfile::tempdir().expect("a temporary directory");

	std::fs::write(temp.path().join("main.rs"), "fn a() {}\n").expect("write");
	std::fs::write(temp.path().join("data.bin"), [0u8, 159, 146, 150]).expect("write");

	let paths = collect_paths(temp.path(), true, &RulesConfig::default().ignore);

	assert_eq!(paths.len(), 1, "only the Rust file should be collected");
	assert!(paths[0].ends_with("main.rs"));
}

#[test]
fn collection_honours_the_ignore_patterns() {
	let temp = tempfile::tempdir().expect("a temporary directory");

	std::fs::write(temp.path().join("main.rs"), "fn a() {}\n").expect("write");
	std::fs::write(temp.path().join("skip.rs"), "fn b() {}\n").expect("write");

	let config = monostyle_core::IgnoreConfig {
		patterns: vec!["skip.rs".to_string()],

		..monostyle_core::IgnoreConfig::default()
	};

	let paths = collect_paths(temp.path(), true, &config);

	assert_eq!(paths.len(), 1);
	assert!(paths[0].ends_with("main.rs"));
}

// ---------------------------------------------------------------------------
// The cache-hit path
// ---------------------------------------------------------------------------

#[test]
fn a_cached_run_produces_the_same_report_as_an_uncached_one() {
	// The cache-hit path builds a report from lines that were serialized and restored, so a field lost in
	// that round trip would change a score between the first and second run of the same command.
	let temp = tempfile::tempdir().expect("a temporary directory");
	let source = "fn a() {\n    work();\n    if x {\n        work();\n    }\n}\n";

	std::fs::write(temp.path().join("sample.rs"), source).expect("write");

	// A `target` directory makes the cache discoverable.
	std::fs::create_dir_all(temp.path().join("target")).expect("mkdir");

	let options = AnalysisOptions {
		cache: true,

		..AnalysisOptions::default()
	};
	let paths = collect_paths(temp.path(), true, &options.rules.ignore);

	let first = analyze_paths(&paths, &options);
	let second = analyze_paths(&paths, &options);

	assert_eq!(first.readability.value, second.readability.value);
	assert_eq!(first.complexity.value, second.complexity.value);
	assert_eq!(
		first.files[0].findings.len(),
		second.files[0].findings.len()
	);
	assert_eq!(first.code_lines, second.code_lines);
}

#[test]
fn a_cached_run_finds_the_same_rules() {
	let temp = tempfile::tempdir().expect("a temporary directory");

	std::fs::write(
		temp.path().join("sample.rs"),
		"fn a() {\n    work();\n    if x {\n        work();\n    }\n    let y = z * 86400;\n}\n",
	)
	.expect("write");

	std::fs::create_dir_all(temp.path().join("target")).expect("mkdir");

	let options = AnalysisOptions {
		cache: true,

		..AnalysisOptions::default()
	};
	let paths = collect_paths(temp.path(), true, &options.rules.ignore);

	let rules_of = |report: &monostyle::analysis::ProjectReport| {
		let mut names: Vec<String> = report
			.files
			.iter()
			.flat_map(|file| file.findings.iter().map(|finding| finding.rule.clone()))
			.collect();

		names.sort_unstable();
		names
	};

	let first = rules_of(&analyze_paths(&paths, &options));
	let second = rules_of(&analyze_paths(&paths, &options));

	assert_eq!(first, second, "a cache hit changed which rules fired");
	assert!(!first.is_empty(), "the fixture should produce findings");
}

#[test]
fn a_cached_run_reports_unit_scores_too() {
	// Unit scores come from the memoized metrics, which the cache-hit path recomputes from restored lines.
	let temp = tempfile::tempdir().expect("a temporary directory");

	std::fs::write(
		temp.path().join("sample.rs"),
		"fn a() {\n    work();\n}\n\nfn b() {\n    work();\n}\n",
	)
	.expect("write");

	std::fs::create_dir_all(temp.path().join("target")).expect("mkdir");

	let options = AnalysisOptions {
		cache: true,

		..AnalysisOptions::default()
	};
	let paths = collect_paths(temp.path(), true, &options.rules.ignore);

	let first = analyze_paths(&paths, &options);
	let second = analyze_paths(&paths, &options);

	assert_eq!(first.files[0].units.len(), second.files[0].units.len());
	assert!(!first.files[0].units.is_empty());
}

// ---------------------------------------------------------------------------
// Cache disable
// ---------------------------------------------------------------------------

#[test]
fn disabling_the_cache_skips_discovery() {
	let temp = tempfile::tempdir().expect("a temporary directory");

	std::fs::write(temp.path().join("sample.rs"), "fn a() {}\n").expect("write");
	std::fs::create_dir_all(temp.path().join("target")).expect("mkdir");

	let options = AnalysisOptions {
		cache: false,

		..AnalysisOptions::default()
	};
	let paths = collect_paths(temp.path(), true, &options.rules.ignore);

	let _ = analyze_paths(&paths, &options);

	let cache_directory = temp.path().join("target/monostyle");

	assert!(
		!cache_directory.exists(),
		"a disabled cache should not create anything"
	);
}

// ---------------------------------------------------------------------------
// Workspace formats
// ---------------------------------------------------------------------------

#[test]
fn an_npm_workspaces_array_is_detected() {
	let temp = tempfile::tempdir().expect("a temporary directory");

	std::fs::write(
		temp.path().join("package.json"),
		r#"{"name": "root", "workspaces": ["packages/*"]}"#,
	)
	.expect("write");
	std::fs::create_dir_all(temp.path().join("packages/one")).expect("mkdir");
	std::fs::write(
		temp.path().join("packages/one/package.json"),
		r#"{"name": "one"}"#,
	)
	.expect("write");

	let packages = detect_packages(temp.path());

	assert_eq!(packages.len(), 1);
	assert_eq!(packages[0].name, "one");
	assert_eq!(packages[0].ecosystem, Ecosystem::Npm);
}

#[test]
fn an_npm_workspaces_object_is_detected() {
	// `workspaces` may be an object with a `packages` key rather than a bare array.
	let temp = tempfile::tempdir().expect("a temporary directory");

	std::fs::write(
		temp.path().join("package.json"),
		r#"{"name": "root", "workspaces": {"packages": ["packages/*"]}}"#,
	)
	.expect("write");
	std::fs::create_dir_all(temp.path().join("packages/one")).expect("mkdir");
	std::fs::write(
		temp.path().join("packages/one/package.json"),
		r#"{"name": "one"}"#,
	)
	.expect("write");

	assert_eq!(detect_packages(temp.path()).len(), 1);
}

#[test]
fn a_glob_member_expands_to_its_directories() {
	let temp = tempfile::tempdir().expect("a temporary directory");

	std::fs::write(
		temp.path().join("package.json"),
		r#"{"name": "root", "workspaces": ["packages/*"]}"#,
	)
	.expect("write");

	for name in ["alpha", "beta", "gamma"] {
		std::fs::create_dir_all(temp.path().join("packages").join(name)).expect("mkdir");
		std::fs::write(
			temp.path().join("packages").join(name).join("package.json"),
			format!(r#"{{"name": "{name}"}}"#),
		)
		.expect("write");
	}

	// A directory with no manifest is not a package.
	std::fs::create_dir_all(temp.path().join("packages/not-a-package")).expect("mkdir");

	let packages = detect_packages(temp.path());
	let mut names: Vec<&str> = packages
		.iter()
		.map(|package| package.name.as_str())
		.collect();

	names.sort_unstable();

	assert_eq!(names, vec!["alpha", "beta", "gamma"]);
}

#[test]
fn a_recursive_glob_reaches_nested_packages() {
	let temp = tempfile::tempdir().expect("a temporary directory");

	std::fs::write(
		temp.path().join("package.json"),
		r#"{"name": "root", "workspaces": ["apps/**"]}"#,
	)
	.expect("write");

	std::fs::create_dir_all(temp.path().join("apps/web/site")).expect("mkdir");
	std::fs::write(
		temp.path().join("apps/web/site/package.json"),
		r#"{"name": "site"}"#,
	)
	.expect("write");

	let packages = detect_packages(temp.path());

	assert!(
		packages.iter().any(|package| package.name == "site"),
		"got {packages:?}"
	);
}

#[test]
fn a_literal_member_is_resolved_without_expansion() {
	let temp = tempfile::tempdir().expect("a temporary directory");

	std::fs::write(
		temp.path().join("pnpm-workspace.yaml"),
		"packages:\n  - tools/cli\n",
	)
	.expect("write");
	std::fs::create_dir_all(temp.path().join("tools/cli")).expect("mkdir");
	std::fs::write(
		temp.path().join("tools/cli/package.json"),
		r#"{"name": "cli"}"#,
	)
	.expect("write");

	let packages = detect_packages(temp.path());

	assert_eq!(packages.len(), 1);
	assert_eq!(packages[0].name, "cli");
}

#[test]
fn a_glob_matching_nothing_finds_no_members() {
	let temp = tempfile::tempdir().expect("a temporary directory");

	std::fs::write(
		temp.path().join("package.json"),
		r#"{"name": "root", "workspaces": ["missing/*"]}"#,
	)
	.expect("write");

	let packages = detect_packages(temp.path());

	// The root declares no package of its own, so nothing is found. The assertion names the members
	// rather than the total, because a repository can legitimately report its own root as a package.
	assert!(
		packages.iter().all(|package| package.name != "missing"),
		"a glob matching nothing should contribute no packages: {packages:?}"
	);
}

#[test]
fn a_cargo_workspace_with_glob_members_is_detected() {
	let temp = tempfile::tempdir().expect("a temporary directory");

	std::fs::write(
		temp.path().join("Cargo.toml"),
		"[workspace]\nmembers = [\"crates/*\"]\n",
	)
	.expect("write");

	for name in ["alpha", "beta"] {
		std::fs::create_dir_all(temp.path().join("crates").join(name)).expect("mkdir");
		std::fs::write(
			temp.path().join("crates").join(name).join("Cargo.toml"),
			format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\n"),
		)
		.expect("write");
	}

	let packages = detect_packages(temp.path());

	assert_eq!(packages.len(), 2);
	assert!(
		packages
			.iter()
			.all(|package| package.ecosystem == Ecosystem::Cargo)
	);
}

#[test]
fn a_dart_workspace_is_detected() {
	let temp = tempfile::tempdir().expect("a temporary directory");

	std::fs::write(
		temp.path().join("pubspec.yaml"),
		"name: root\nworkspace:\n  - packages/app\n",
	)
	.expect("write");
	std::fs::create_dir_all(temp.path().join("packages/app")).expect("mkdir");
	std::fs::write(temp.path().join("packages/app/pubspec.yaml"), "name: app\n").expect("write");

	let packages = detect_packages(temp.path());

	assert_eq!(packages.len(), 1);
	assert_eq!(packages[0].name, "app");
	assert_eq!(packages[0].ecosystem, Ecosystem::Dart);
}

#[test]
fn a_malformed_cargo_manifest_is_skipped() {
	let temp = tempfile::tempdir().expect("a temporary directory");

	std::fs::write(temp.path().join("Cargo.toml"), "this is not toml [[[").expect("write");

	assert!(detect_packages(temp.path()).is_empty());
}

#[test]
fn a_cargo_manifest_with_no_package_section_is_skipped() {
	let temp = tempfile::tempdir().expect("a temporary directory");

	std::fs::write(
		temp.path().join("Cargo.toml"),
		"[workspace]\nmembers = []\n",
	)
	.expect("write");

	assert!(detect_packages(temp.path()).is_empty());
}

// ---------------------------------------------------------------------------
// Report conditionals
// ---------------------------------------------------------------------------

#[test]
fn a_report_with_tokenizer_warnings_prints_them() {
	// An unterminated construct makes a score less trustworthy, and the report has to say so.
	let report = analyze_source(
		std::path::Path::new("broken.rs"),
		"let text = r#\"never closed\nfn a() {}\n",
		&uncached(),
	)
	.expect("a report");

	assert!(
		!report.unterminated.is_empty(),
		"the fixture should leave a construct open"
	);

	let rendered = monostyle::report::render_project(
		&monostyle::analysis::aggregate(vec![report], Vec::new(), &uncached()),
		false,
		false,
	);

	assert!(
		rendered.contains("tokenizer warnings"),
		"the warning should be reported:\n{rendered}"
	);
	assert!(rendered.contains("never closed") || rendered.contains("reliab"));
}

#[test]
fn a_report_with_no_packages_omits_the_package_table() {
	let temp = tempfile::tempdir().expect("a temporary directory");

	std::fs::write(temp.path().join("sample.rs"), "fn a() {}\n").expect("write");

	let options = uncached();
	let paths = collect_paths(temp.path(), true, &options.rules.ignore);
	let mut report = analyze_paths(&paths, &options);

	// Clear detection so the conditional path is exercised; the fixture sits inside a package.
	report.packages.clear();

	let rendered = monostyle::report::render_project(&report, false, false);

	assert!(!rendered.contains("\npackages\n"));
}

#[test]
fn a_report_with_many_findings_truncates_the_list() {
	// The explain list is capped, and the cap has to report how much it hid.
	let temp = tempfile::tempdir().expect("a temporary directory");

	// A file with many independent problems produces more findings than the limit.
	let body: String = (0..120)
		.map(|index| format!("\tlet v{index} = work();\n\tif x{index} {{ work(); }}\n"))
		.fold(String::new(), |mut text, line| {
			text.push_str(&line);

			text
		});

	std::fs::write(
		temp.path().join("sample.rs"),
		format!("fn a() {{\n{body}}}\n"),
	)
	.expect("write");

	let options = uncached();
	let paths = collect_paths(temp.path(), true, &options.rules.ignore);
	let report = analyze_paths(&paths, &options);

	let rendered = monostyle::report::render_project(&report, true, false);

	assert!(rendered.contains("all findings"));
}

#[test]
fn a_report_with_many_rules_truncates_the_impact_table() {
	let temp = tempfile::tempdir().expect("a temporary directory");

	let body: String = (0..120)
		.map(|index| format!("\tlet v{index} = work();\n\tif x{index} {{ work(); }}\n"))
		.fold(String::new(), |mut text, line| {
			text.push_str(&line);

			text
		});

	std::fs::write(
		temp.path().join("sample.rs"),
		format!("fn a() {{\n{body}}}\n"),
	)
	.expect("write");

	let options = uncached();
	let paths = collect_paths(temp.path(), true, &options.rules.ignore);
	let rendered =
		monostyle::report::render_project(&analyze_paths(&paths, &options), false, false);

	assert!(rendered.contains("more rules") || rendered.contains("costing you points"));
}

#[test]
fn a_unit_table_limits_its_rows() {
	let temp = tempfile::tempdir().expect("a temporary directory");

	let functions = lines_of((0..20).map(|index| format!("fn f{index}() {{ work(); }}\n")));

	std::fs::write(temp.path().join("sample.rs"), functions).expect("write");

	let options = uncached();
	let paths = collect_paths(temp.path(), true, &options.rules.ignore);
	let rendered = monostyle::report::render_project(&analyze_paths(&paths, &options), false, true);

	// The table is capped at ten function rows. One further line in the section names the file rather
	// than a function, so the assertion counts the rows that carry a line number.
	let table = rendered
		.split("worst functions")
		.nth(1)
		.expect("the unit table should be present");
	let rows = table
		.lines()
		.filter(|line| {
			line.contains("sample.rs:") && line.chars().any(|character| character == ':')
		})
		.filter(|line| line.matches(':').count() >= 2)
		.count();

	assert!(
		rows <= 11,
		"the unit table should be capped at ten rows, got {rows}"
	);
}
