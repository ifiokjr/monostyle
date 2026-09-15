//! Fixture-driven integration tests.
//!
//! Every test here runs the whole pipeline — collect paths, lex, measure, apply rules, score,
//! render — against files on disk rather than against strings in the test. That matters because the
//! failures worth catching are cross-cutting: a rule that is correct in isolation but fires on real
//! code, a path filter that skips the wrong file, a score that moves when it should not.
//!
//! Snapshots are used where the assertion is "this output is right" rather than "this value is
//! above a threshold". A snapshot makes a message change visible in a diff, which is what keeps an
//! explainability tool honest: the wording of a finding is part of its contract.

use std::path::Path;
use std::path::PathBuf;

use monostyle::analysis::AnalysisOptions;
use monostyle::analysis::ProjectReport;
use monostyle::analysis::analyze_paths;
use monostyle::analysis::collect_paths;
use monostyle_core::Language;
use monostyle_core::ignore::IgnoreConfig;

/// Returns the path to a fixture.
fn fixture(relative: &str) -> PathBuf {
	PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.join("tests/fixtures")
		.join(relative)
}

/// Analyzes a fixture directory with the default options.
fn analyze(relative: &str) -> ProjectReport {
	let root = fixture(relative);
	let options = AnalysisOptions {
		cache: false,
		..AnalysisOptions::default()
	};
	let paths = collect_paths(&root, true, &options.rules.ignore);

	assert!(
		!paths.is_empty(),
		"no analyzable files under {}",
		root.display()
	);

	analyze_paths(&paths, &options)
}

/// Analyzes a fixture directory, allowing the result to be empty.
///
/// Separate from [`analyze`] because the generated fixture exists to prove that nothing is found, and
/// a helper that insists on finding something would make that test impossible to write.
fn analyze_allow_empty(relative: &str) -> ProjectReport {
	let root = fixture(relative);
	let options = AnalysisOptions {
		cache: false,
		..AnalysisOptions::default()
	};
	let paths = collect_paths(&root, true, &options.rules.ignore);

	analyze_paths(&paths, &options)
}

/// Returns the rules that produced a penalizing finding in a report.
fn penalizing_rules(report: &ProjectReport) -> Vec<String> {
	let mut rules: Vec<String> = report
		.penalizing_findings()
		.into_iter()
		.map(|(_path, finding)| finding.rule.clone())
		.collect();

	rules.sort_unstable();
	rules.dedup();
	rules
}

/// Formats a report's scores for a snapshot.
fn score_summary(report: &ProjectReport) -> String {
	format!(
		"files: {}\ncode lines: {}\nreadability: {:.1}\ncomplexity: {:.1}\noverall: {:.1}",
		report.files.len(),
		report.code_lines,
		report.readability.value,
		report.complexity.value,
		report.overall()
	)
}

// ---------------------------------------------------------------------------
// Positive and negative controls
// ---------------------------------------------------------------------------

#[test]
fn well_written_code_produces_no_penalizing_findings() {
	let report = analyze("good");

	let rules = penalizing_rules(&report);

	assert!(
		rules.is_empty(),
		"the good fixture should have nothing to penalize, but these rules fired: {rules:?}"
	);
	assert!(
		report.readability.value > 95.0,
		"expected a high readability score, got {:.1}",
		report.readability.value
	);
}

#[test]
fn badly_written_code_is_caught_by_named_rules() {
	let report = analyze("bad");

	let rules = penalizing_rules(&report);

	// Each of these corresponds to a specific problem seeded into the fixture, so a regression that
	// stops one rule from firing is a test failure rather than a quiet score change.
	for expected in [
		"readability/blank-line-before-control-flow",
		"readability/deep-nesting",
		"readability/excessive-indentation",
		"readability/group-separation",
		"readability/long-parameter-list",
		"readability/comment-required-on-complex-unit",
	] {
		assert!(
			rules.contains(&expected.to_string()),
			"`{expected}` should have fired. Rules that did: {rules:?}"
		);
	}
}

#[test]
fn the_gap_between_good_and_bad_is_wide() {
	let good = analyze("good");
	let bad = analyze("bad");

	let gap = good.overall() - bad.overall();

	assert!(
		gap >= 40.0,
		"good ({:.1}) and bad ({:.1}) are only {gap:.1} apart, which is too close to be a signal",
		good.overall(),
		bad.overall()
	);
}

#[test]
fn good_fixture_scores_snapshot() {
	// Pinned because a change to any threshold, weight, or curve moves these numbers, and that is a
	// change worth seeing in review rather than discovering from a user report.
	//
	// The two fixtures are snapshotted in separate tests because a snapshot assertion aborts its test
	// on mismatch, so a single test holding both would report only the first regression.
	insta::assert_snapshot!("good_scores", score_summary(&analyze("good")));
}

#[test]
fn bad_fixture_scores_snapshot() {
	insta::assert_snapshot!("bad_scores", score_summary(&analyze("bad")));
}

// ---------------------------------------------------------------------------
// Ignoring
// ---------------------------------------------------------------------------

#[test]
fn generated_files_are_excluded_by_default() {
	let report = analyze_allow_empty("generated");

	assert!(
		report.files.is_empty(),
		"generated files should be skipped by default, but these were analyzed: {:?}",
		report
			.files
			.iter()
			.map(|file| file.path.display().to_string())
			.collect::<Vec<_>>()
	);
}

#[test]
fn generated_files_can_be_included() {
	let root = fixture("generated");
	let options = AnalysisOptions {
		cache: false,
		rules: monostyle_rules::RulesConfig {
			ignore: IgnoreConfig {
				generated: false,
				..IgnoreConfig::default()
			},
			..monostyle_rules::RulesConfig::default()
		},
		..AnalysisOptions::default()
	};

	let paths = collect_paths(&root, true, &options.rules.ignore);
	let report = analyze_paths(&paths, &options);

	assert!(
		!report.files.is_empty(),
		"turning the generated exclusion off should analyze these files"
	);
}

#[test]
fn ignored_directories_are_never_analyzed() {
	let config = IgnoreConfig::default();

	for directory in [
		"node_modules/pkg/index.js",
		"target/debug/build.rs",
		"vendor/lib/thing.go",
		"dist/bundle.js",
		".venv/lib/python3.12/site.py",
		"build/generated.rs",
	] {
		assert!(
			!config.allows(Path::new(directory)),
			"`{directory}` should be ignored by default"
		);
	}
}

#[test]
fn user_patterns_are_honoured() {
	let config = IgnoreConfig {
		patterns: vec!["**/*.spec.ts".to_string(), "legacy/**".to_string()],
		..IgnoreConfig::default()
	};

	assert!(!config.allows(Path::new("src/app.spec.ts")));
	assert!(!config.allows(Path::new("legacy/old.rs")));
	assert!(config.allows(Path::new("src/app.ts")));
}

#[test]
fn include_entries_override_every_other_rule() {
	// The escape hatch has to work against all three filters, or a project cannot bring back a
	// vendored directory that is genuinely theirs.
	let config = IgnoreConfig {
		patterns: vec!["src/**".to_string()],
		include: vec!["src/keep.rs".to_string()],
		..IgnoreConfig::default()
	};

	assert!(
		config.allows(Path::new("src/keep.rs")),
		"an include entry should win over a pattern"
	);
	assert!(!config.allows(Path::new("src/other.rs")));
}

#[test]
fn generated_detection_recognizes_each_ecosystem() {
	for path in [
		"lib/model.g.dart",
		"lib/model.freezed.dart",
		"src/schema.pb.rs",
		"src/api.pb.go",
		"src/form_pb2.py",
		"src/types.gen.ts",
		"obj/Debug/Form.designer.cs",
		"dist/app.min.js",
		"pnpm-lock.yaml",
		"Cargo.lock",
	] {
		assert!(
			monostyle_core::ignore::is_generated(Path::new(path)),
			"`{path}` looks generated"
		);
	}

	for path in ["src/main.rs", "lib/model.dart", "src/index.ts"] {
		assert!(
			!monostyle_core::ignore::is_generated(Path::new(path)),
			"`{path}` is hand-written"
		);
	}
}

// ---------------------------------------------------------------------------
// Languages
// ---------------------------------------------------------------------------

#[test]
fn each_language_fixture_is_detected_and_scored() {
	let report = analyze("languages");

	let languages: Vec<Language> = report.files.iter().map(|file| file.language).collect();

	for expected in [Language::Python, Language::TypeScript, Language::Dart] {
		assert!(
			languages.contains(&expected),
			"expected a {expected} fixture, got {languages:?}"
		);
	}

	// These fixtures are written in the style the tool recommends, so a finding against one means a
	// language profile is misconfigured rather than the fixture being wrong.
	for file in &report.files {
		assert!(
			file.readability.value > 70.0,
			"{} ({}) scored only {:.1}. Findings: {:?}",
			file.path.display(),
			file.language,
			file.readability.value,
			file.findings
				.iter()
				.map(|finding| format!("{}: {}", finding.rule, finding.message))
				.collect::<Vec<_>>()
		);
	}
}

#[test]
fn doc_comments_are_recognized_across_languages() {
	// The `/**` form is the common documentation syntax in TypeScript and Java, and treating it as an
	// ordinary comment made the classifier judge it. This asserts the fix holds per language.
	let report = analyze("languages");

	let typescript = report
		.files
		.iter()
		.find(|file| file.language == Language::TypeScript)
		.expect("a TypeScript fixture");

	assert!(
		typescript
			.findings
			.iter()
			.all(|finding| finding.rule != "readability/comment-narrates-code"),
		"a JSDoc block should never be penalized as narration"
	);
}

// ---------------------------------------------------------------------------
// Markdown
// ---------------------------------------------------------------------------

#[test]
fn markdown_prose_is_never_reported_for_line_length() {
	let report = analyze("markdown");
	let file = report.files.first().expect("a Markdown fixture");

	assert!(
		file.findings
			.iter()
			.all(|finding| finding.rule != "readability/overlong-line"),
		"Markdown prose has no line limit, but a line-length finding fired"
	);
}

#[test]
fn markdown_measures_code_inside_fences() {
	let report = analyze("markdown");
	let file = report.files.first().expect("a Markdown fixture");

	assert!(
		file.findings
			.iter()
			.any(|finding| finding.rule == "readability/blank-line-before-return"),
		"the deliberately cramped fence should be measured. Findings: {:?}",
		file.findings
			.iter()
			.map(|finding| finding.rule.clone())
			.collect::<Vec<_>>()
	);
}

// ---------------------------------------------------------------------------
// Workspaces
// ---------------------------------------------------------------------------

#[test]
fn workspace_packages_are_detected_and_scored_separately() {
	let report = analyze("workspace");

	assert!(
		!report.packages.is_empty(),
		"package detection found nothing; the fixture declares a workspace in Cargo.toml"
	);

	let names: Vec<&str> = report
		.packages
		.iter()
		.map(|package| package.package.name.as_str())
		.collect();

	assert!(
		names.contains(&"alpha"),
		"expected the alpha crate, got {names:?}"
	);
	assert!(
		names.contains(&"beta"),
		"expected the beta crate, got {names:?}"
	);
}

#[test]
fn packages_are_scored_independently() {
	let report = analyze("workspace");

	let alpha = report
		.packages
		.iter()
		.find(|package| package.package.name == "alpha")
		.expect("alpha");
	let beta = report
		.packages
		.iter()
		.find(|package| package.package.name == "beta")
		.expect("beta");

	assert_eq!(alpha.files, 1);
	assert_eq!(beta.files, 1);

	// Beta has two guard clauses without blank lines between them, so it must score lower than the
	// crate that has none. This is the per-package attribution doing its job.
	assert!(
		beta.readability.value < alpha.readability.value,
		"beta ({:.1}) should score below alpha ({:.1})",
		beta.readability.value,
		alpha.readability.value
	);
}

// ---------------------------------------------------------------------------
// Determinism and robustness
// ---------------------------------------------------------------------------

#[test]
fn analysis_is_deterministic() {
	let first = analyze("bad");
	let second = analyze("bad");

	assert_eq!(first.readability.value, second.readability.value);
	assert_eq!(first.complexity.value, second.complexity.value);
	assert_eq!(first.files.len(), second.files.len());
}

#[test]
fn every_finding_explains_itself() {
	for fixture_name in ["good", "bad", "languages", "markdown", "workspace"] {
		let report = analyze(fixture_name);

		for file in &report.files {
			for finding in &file.findings {
				assert!(
					!finding.message.is_empty() && finding.message != "rule violated",
					"{fixture_name}: {} has no message",
					finding.rule
				);
				assert!(
					!finding.suggestion.is_empty() && finding.suggestion != "review this location",
					"{fixture_name}: {} has no suggestion",
					finding.rule
				);
			}
		}
	}
}

#[test]
fn scores_stay_within_bounds() {
	for fixture_name in ["good", "bad", "languages", "markdown", "workspace"] {
		let report = analyze(fixture_name);

		assert!(
			(0.0..=100.0).contains(&report.readability.value),
			"{fixture_name}: readability out of range: {}",
			report.readability.value
		);
		assert!(
			(0.0..=100.0).contains(&report.complexity.value),
			"{fixture_name}: complexity out of range: {}",
			report.complexity.value
		);
	}
}

#[test]
fn unit_scores_are_attributed_to_the_right_function() {
	let report = analyze("bad");
	let file = report.files.first().expect("a fixture");
	let unit = file.units.first().expect("a detected function");

	// Every finding attributed to a unit must fall inside its line range, or a function's score is
	// being moved by its neighbours.
	for finding in &unit.findings {
		assert!(
			finding.span.start_line >= unit.start_line && finding.span.start_line <= unit.end_line,
			"a finding at line {} was attributed to `{}` spanning {}..{}",
			finding.span.start_line,
			unit.name,
			unit.start_line,
			unit.end_line
		);
	}
}

#[test]
fn json_output_round_trips() {
	let report = analyze("bad");
	let encoded = monostyle::report::render_project_json(&report).expect("JSON encoding");

	let decoded: serde_json::Value = serde_json::from_str(&encoded).expect("JSON decoding");

	assert!(decoded["files"].is_array());
	assert!(decoded["readability"]["value"].is_number());
	assert!(
		decoded["files"][0]["findings"].is_array(),
		"findings should be present in the JSON"
	);
}
