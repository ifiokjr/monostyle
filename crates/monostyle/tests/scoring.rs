//! Scoring behaviour tests.
//!
//! These tests pin the property that matters most: good code scores well and bad code scores
//! badly, by a wide margin. They are written against the whole pipeline — lex, measure, run
//! rules, score — rather than against individual rules, because the failure this suite exists
//! to catch is a *combination* effect: rules whose thresholds interact to produce a score
//! nobody can justify.

use std::path::Path;
use std::path::PathBuf;

use monostyle::analysis::AnalysisOptions;
use monostyle::analysis::FileReport;
use monostyle::analysis::analyze_source;

/// Analyzes a fixture from the workspace's `examples/` directory.
///
/// The fixtures live at the workspace root rather than inside this crate because they are
/// documentation as much as test data: a reader should be able to open them without knowing
/// how the crates are laid out.
fn fixture(relative: &str) -> FileReport {
	let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.parent()
		.and_then(Path::parent)
		.expect("the crate lives two levels below the workspace root")
		.join(relative);

	let source = std::fs::read_to_string(&path)
		.unwrap_or_else(|error| panic!("could not read {}: {error}", path.display()));

	analyze_source(&path, &source, &AnalysisOptions::default())
		.unwrap_or_else(|| panic!("no language detected for {}", path.display()))
}

/// Returns the rules that fired in a report.
fn rules(report: &FileReport) -> Vec<String> {
	let mut names: Vec<String> = report
		.findings
		.iter()
		.map(|finding| finding.rule.clone())
		.collect();

	names.sort();
	names.dedup();
	names
}

#[test]
fn good_code_scores_near_perfect() {
	let report = fixture("examples/good/rust.rs");

	assert!(
		report.readability.value >= 95.0,
		"readable code should score highly, got {:.1}. Findings: {:?}",
		report.readability.value,
		rules(&report)
	);
	assert!(
		report.complexity.value >= 95.0,
		"simple code should score highly for complexity, got {:.1}",
		report.complexity.value
	);
}

#[test]
fn bad_code_scores_badly() {
	let report = fixture("examples/bad/rust.rs");

	assert!(
		report.readability.value <= 25.0,
		"unreadable code should score poorly, got {:.1}",
		report.readability.value
	);
	assert!(
		report.complexity.value <= 50.0,
		"complex code should score poorly for complexity, got {:.1}",
		report.complexity.value
	);
}

#[test]
fn good_and_bad_code_are_clearly_separated() {
	let good = fixture("examples/good/rust.rs");
	let bad = fixture("examples/bad/rust.rs");

	// The gap is the point: a scorer whose good and bad examples land within a few points of
	// each other is not measuring anything a reader would recognize.
	let gap = good.overall() - bad.overall();

	assert!(
		gap >= 40.0,
		"good code ({:.1}) and bad code ({:.1}) are only {gap:.1} apart, which is too close to \
		 be a useful signal",
		good.overall(),
		bad.overall()
	);
}

#[test]
fn whitespace_rules_catch_missing_blank_lines() {
	let report = fixture("examples/bad/rust.rs");
	let fired = rules(&report);

	assert!(
		fired.contains(&"readability/blank-line-before-control-flow".to_string()),
		"stacked control flow should be reported, got {fired:?}"
	);
	assert!(
		fired.contains(&"readability/excessive-indentation".to_string()),
		"deep indentation should be reported, got {fired:?}"
	);
}

#[test]
fn complexity_rules_catch_a_hard_function() {
	let report = fixture("examples/bad/rust.rs");
	let fired = rules(&report);

	assert!(
		fired.contains(&"complexity/cyclomatic-per-unit".to_string()),
		"a function with many paths should be reported, got {fired:?}"
	);
	assert!(
		fired.contains(&"complexity/cognitive-per-unit".to_string()),
		"a hard-to-follow function should be reported, got {fired:?}"
	);
}

#[test]
fn the_good_example_has_no_readability_findings() {
	let report = fixture("examples/good/rust.rs");

	let penalizing: Vec<&monostyle_core::Finding> = report
		.findings
		.iter()
		.filter(|finding| finding.penalty() > 0.0)
		.collect();

	assert!(
		penalizing.is_empty(),
		"readable code should have nothing to penalize, but found: {:?}",
		penalizing
			.iter()
			.map(|finding| format!("{}: {}", finding.rule, finding.message))
			.collect::<Vec<_>>()
	);
}

#[test]
fn every_finding_explains_itself() {
	// A finding without a message or a suggestion is a score nobody can act on, which defeats
	// the purpose of reporting rather than silently deducting.
	for fixture_path in ["examples/good/rust.rs", "examples/bad/rust.rs"] {
		let report = fixture(fixture_path);

		for finding in &report.findings {
			assert!(
				!finding.message.is_empty(),
				"{}: {} has no message",
				fixture_path,
				finding.rule
			);
			assert!(
				!finding.suggestion.is_empty(),
				"{}: {} has no suggestion",
				fixture_path,
				finding.rule
			);
			assert_ne!(
				finding.message, "rule violated",
				"{}: {} fell back to the default message",
				fixture_path, finding.rule
			);
		}
	}
}

#[test]
fn scores_are_bounded_and_monotonic_in_penalty() {
	let good = fixture("examples/good/rust.rs");
	let bad = fixture("examples/bad/rust.rs");

	for report in [&good, &bad] {
		assert!(
			(0.0..=100.0).contains(&report.readability.value),
			"readability out of range: {}",
			report.readability.value
		);
		assert!(
			(0.0..=100.0).contains(&report.complexity.value),
			"complexity out of range: {}",
			report.complexity.value
		);
	}

	// More penalty must never mean a higher score.
	assert!(
		good.readability.penalty <= bad.readability.penalty,
		"the good example carries more readability penalty than the bad one"
	);
}

#[test]
fn unit_detection_finds_the_real_functions_only() {
	let report = fixture("examples/bad/rust.rs");

	let names: Vec<&str> = report.units.iter().map(|unit| unit.name.as_str()).collect();

	assert!(
		names.contains(&"process"),
		"the declared function should be detected, got {names:?}"
	);

	// Calls must never be mistaken for declarations. `compute`, `transform`, and `emit` are all
	// invoked in this file and none of them is defined in it.
	for call in ["compute", "transform", "emit", "is_empty", "contains"] {
		assert!(
			!names.contains(&call),
			"`{call}` is a call site but was reported as a function: {names:?}"
		);
	}
}

#[test]
fn a_unit_spans_its_whole_body_including_nested_blocks() {
	let report = fixture("examples/bad/rust.rs");

	let unit = report
		.units
		.iter()
		.find(|unit| unit.name == "process")
		.expect("process should be detected");

	// The nested `for` loops inside the function must fall within its span; a detector that
	// ended the unit at the first nested close would cut this far short.
	assert!(
		unit.end_line > unit.start_line + 20,
		"the unit spans only {} lines, which means nested blocks ended it early",
		unit.end_line - unit.start_line
	);
}

#[test]
fn scoring_is_stable_for_an_empty_file() {
	let report = analyze_source(Path::new("empty.rs"), "", &AnalysisOptions::default())
		.expect("rust should be detected");

	assert_eq!(report.code_lines, 0);
	assert_eq!(
		report.readability.value, 100.0,
		"an empty file has nothing wrong with it"
	);
	assert_eq!(report.findings, [] as [monostyle_core::Finding; 0]);
}

#[test]
fn scoring_handles_a_file_with_only_comments() {
	let report = analyze_source(
		Path::new("comments.rs"),
		"// just a comment\n// and another\n",
		&AnalysisOptions::default(),
	)
	.expect("rust should be detected");

	assert_eq!(report.code_lines, 0);
	assert!(
		report.units.is_empty(),
		"a comment-only file declares no functions"
	);
	assert_eq!(report.readability.value, 100.0);
}
