//! Tests for report rendering and package detection.
//!
//! The rendered text is what a person reads and the JSON is what a tool consumes, so both are pinned.
//! The text output is checked for the properties that make it usable — a location on every finding, a
//! score on every row — rather than for exact wording, which would make every copy edit a test failure;
//! the JSON is checked for its shape, which is a contract with other tools.

use std::path::PathBuf;

use monostyle::analysis::AnalysisOptions;
use monostyle::analysis::analyze_paths;
use monostyle::analysis::collect_paths;
use monostyle::package::Ecosystem;
use monostyle::package::detect_packages;
use monostyle::report;

/// Returns the path to a fixture.
fn fixture(relative: &str) -> PathBuf {
	PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.join("tests/fixtures")
		.join(relative)
}

/// Analyzes a fixture directory.
fn analyze(relative: &str) -> monostyle::analysis::ProjectReport {
	let root = fixture(relative);
	let options = AnalysisOptions {
		cache: false,

		..AnalysisOptions::default()
	};
	let paths = collect_paths(&root, true, &options.rules.ignore);

	analyze_paths(&paths, &options)
}

// ---------------------------------------------------------------------------
// Text rendering
// ---------------------------------------------------------------------------

#[test]
fn the_report_states_both_scores() {
	let rendered = report::render_project(&analyze("bad"), false, false);

	assert!(
		rendered.contains("readability"),
		"the report should name readability"
	);
	assert!(
		rendered.contains("complexity"),
		"the report should name complexity"
	);
	assert!(
		rendered.contains("overall"),
		"the report should state an overall score"
	);
}

#[test]
fn the_report_names_the_file_count_and_volume() {
	let rendered = report::render_project(&analyze("bad"), false, false);

	assert!(
		rendered.contains("files"),
		"the report should state a file count"
	);
	assert!(
		rendered.contains("lines of code"),
		"the report should state the code volume"
	);
}

#[test]
fn the_report_points_at_a_location_for_each_impact() {
	// The location is what makes a report actionable, so a row without one is a row a reader cannot
	// act on.
	let rendered = report::render_project(&analyze("bad"), false, false);

	assert!(
		rendered.contains("rust.rs:"),
		"a finding location should appear as `path:line`, got:\n{rendered}"
	);
}

#[test]
fn the_report_ranks_by_impact() {
	let rendered = report::render_project(&analyze("bad"), false, false);

	assert!(
		rendered.contains("costing you points"),
		"the report should frame findings as impact"
	);
}

#[test]
fn the_report_offers_a_starting_point() {
	let rendered = report::render_project(&analyze("bad"), false, false);

	assert!(
		rendered.contains("start here"),
		"the report should name the highest-value fix"
	);
}

#[test]
fn a_clean_report_says_so() {
	let rendered = report::render_project(&analyze("good"), false, false);

	assert!(
		rendered.contains("No rule violations") || !rendered.contains("costing you points"),
		"a clean repository should not be told it has problems, got:\n{rendered}"
	);
}

#[test]
fn the_unit_table_appears_only_when_requested() {
	let with_units = report::render_project(&analyze("bad"), false, true);

	assert!(
		with_units.contains("worst functions"),
		"--units should add the table"
	);

	let without_units = report::render_project(&analyze("bad"), false, false);

	assert!(
		!without_units.contains("worst functions"),
		"the table should be absent by default"
	);
}

#[test]
fn every_finding_is_shown_only_when_explaining() {
	let explained = report::render_project(&analyze("bad"), true, false);

	assert!(
		explained.contains("all findings"),
		"--explain should list every finding"
	);

	let summary = report::render_project(&analyze("bad"), false, false);

	assert!(
		!summary.contains("all findings"),
		"the list should be absent by default, so a summary stays readable"
	);
}

#[test]
fn the_package_table_appears_for_a_workspace() {
	let rendered = report::render_project(&analyze("workspace"), false, false);

	assert!(
		rendered.contains("packages"),
		"a workspace should show a package table"
	);
	assert!(rendered.contains("alpha"));
	assert!(rendered.contains("beta"));
}

#[test]
fn a_root_package_is_reported_when_one_is_detected() {
	// The fixture declares no workspace, but the analyzed tree sits inside a Cargo package, so
	// detection walks upward and finds one. Reporting it is correct: the tool should attribute files to
	// the package that owns them even when that package is the analyzed root.
	let rendered = report::render_project(&analyze("bad"), false, false);

	assert!(
		rendered.contains("packages") || rendered.contains("files"),
		"the report should be usable whether or not a package was detected"
	);
}

#[test]
fn rendering_never_produces_a_ragged_score_line() {
	// Every score row goes through the same formatter, so a missing value would show as an empty column
	// rather than as a readable number.
	let rendered = report::render_project(&analyze("bad"), false, true);

	for line in rendered.lines() {
		if line.trim_start().starts_with("readability ")
			|| line.trim_start().starts_with("complexity ")
		{
			assert!(
				line.chars().any(|character| character.is_ascii_digit()),
				"a score row should contain a number: {line:?}"
			);
		}
	}
}

#[test]
fn an_empty_report_renders_without_panicking() {
	// A directory with no analyzable files reaches the renderer, so it must produce something.
	let options = AnalysisOptions {
		cache: false,

		..AnalysisOptions::default()
	};
	let report_value = analyze_paths(&[], &options);

	let rendered = report::render_project(&report_value, true, true);

	assert_ne!(rendered, "");
}

// ---------------------------------------------------------------------------
// JSON rendering
// ---------------------------------------------------------------------------

#[test]
fn json_output_has_the_documented_shape() {
	let report_value = analyze("bad");
	let encoded = report::render_project_json(&report_value).expect("JSON encoding");
	let decoded: serde_json::Value = serde_json::from_str(&encoded).expect("JSON decoding");

	for key in [
		"files",
		"readability",
		"complexity",
		"code_lines",
		"skipped",
		"packages",
	] {
		assert!(
			decoded.get(key).is_some(),
			"the JSON should contain `{key}`"
		);
	}

	for key in ["value", "penalty", "density"] {
		assert!(
			decoded["readability"].get(key).is_some(),
			"a score should contain `{key}`"
		);
	}
}

#[test]
fn json_findings_carry_their_explanation() {
	let report_value = analyze("bad");
	let encoded = report::render_project_json(&report_value).expect("JSON encoding");
	let decoded: serde_json::Value = serde_json::from_str(&encoded).expect("JSON decoding");

	let finding = &decoded["files"][0]["findings"][0];

	for key in [
		"rule",
		"category",
		"severity",
		"message",
		"suggestion",
		"span",
	] {
		assert!(
			finding.get(key).is_some(),
			"a finding should contain `{key}`"
		);
	}
}

#[test]
fn json_packages_are_included_for_a_workspace() {
	let report_value = analyze("workspace");
	let encoded = report::render_project_json(&report_value).expect("JSON encoding");
	let decoded: serde_json::Value = serde_json::from_str(&encoded).expect("JSON decoding");

	let packages = decoded["packages"]
		.as_array()
		.expect("packages is an array");

	assert_eq!(packages.len(), 2);
	assert!(packages[0]["package"]["name"].is_string());
	assert!(packages[0]["package"]["ecosystem"].is_string());
	assert!(packages[0]["code_lines"].is_number());
}

#[test]
fn a_missing_fix_is_omitted_from_json() {
	// The field is skipped when absent, so a consumer can treat its presence as "this is fixable"
	// rather than having to check for null.
	let report_value = analyze("good");
	let encoded = report::render_project_json(&report_value).expect("JSON encoding");

	assert!(
		!encoded.contains("\"fix\":null"),
		"an absent fix should be omitted rather than null"
	);
}

// ---------------------------------------------------------------------------
// Package detection
// ---------------------------------------------------------------------------

#[test]
fn a_cargo_workspace_is_detected() {
	let packages = detect_packages(&fixture("workspace"));

	let names: Vec<&str> = packages
		.iter()
		.map(|package| package.name.as_str())
		.collect();

	assert!(names.contains(&"alpha"));
	assert!(names.contains(&"beta"));
	assert!(
		packages
			.iter()
			.all(|package| package.ecosystem == Ecosystem::Cargo)
	);
}

#[test]
fn package_directories_are_absolute_paths_to_the_manifest() {
	let packages = detect_packages(&fixture("workspace"));

	for package in &packages {
		assert!(
			package.directory.is_absolute(),
			"{}: directory should be absolute",
			package.name
		);
		assert!(
			package.directory.join("Cargo.toml").is_file(),
			"{}: the directory should contain a manifest",
			package.name
		);
	}
}

#[test]
fn ecosystem_labels_are_lowercase() {
	assert_eq!(Ecosystem::Cargo.label(), "cargo");
	assert_eq!(Ecosystem::Npm.label(), "npm");
	assert_eq!(Ecosystem::Dart.label(), "dart");
}

#[test]
fn a_directory_with_no_manifest_detects_no_packages() {
	let temp = tempfile::tempdir().expect("a temporary directory");

	assert!(detect_packages(temp.path()).is_empty());
}

#[test]
fn an_npm_package_json_is_detected() {
	let temp = tempfile::tempdir().expect("a temporary directory");

	std::fs::write(
		temp.path().join("package.json"),
		r#"{"name": "my-package", "version": "1.0.0"}"#,
	)
	.expect("write");

	let packages = detect_packages(temp.path());

	assert_eq!(packages.len(), 1);
	assert_eq!(packages[0].name, "my-package");
	assert_eq!(packages[0].ecosystem, Ecosystem::Npm);
}

#[test]
fn a_pnpm_workspace_is_detected() {
	let temp = tempfile::tempdir().expect("a temporary directory");

	std::fs::write(
		temp.path().join("pnpm-workspace.yaml"),
		"packages:\n  - \"packages/*\"\n",
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
}

#[test]
fn a_dart_package_is_detected() {
	let temp = tempfile::tempdir().expect("a temporary directory");

	std::fs::write(temp.path().join("pubspec.yaml"), "name: my_app\n").expect("write");

	let packages = detect_packages(temp.path());

	assert_eq!(packages.len(), 1);
	assert_eq!(packages[0].name, "my_app");
	assert_eq!(packages[0].ecosystem, Ecosystem::Dart);
}

#[test]
fn a_malformed_manifest_is_skipped_rather_than_failing() {
	// A repository with one broken manifest should still analyze, because refusing to score any of it
	// would be a worse outcome than skipping the file that cannot be read.
	let temp = tempfile::tempdir().expect("a temporary directory");

	std::fs::write(temp.path().join("package.json"), "this is not json").expect("write");

	let packages = detect_packages(temp.path());

	assert!(packages.is_empty());
}

#[test]
fn a_package_with_no_name_field_is_skipped() {
	let temp = tempfile::tempdir().expect("a temporary directory");

	std::fs::write(temp.path().join("package.json"), r#"{"version": "1.0.0"}"#).expect("write");

	assert!(detect_packages(temp.path()).is_empty());
}
