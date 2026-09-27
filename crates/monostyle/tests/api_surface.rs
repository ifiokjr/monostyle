//! Direct tests of the reporting API surface.
//!
//! These cover the public methods the CLI composes: ranking, grading, floor violations, and the
//! GitHub annotation renderer. They construct real reports through `analyze_with_language` so the
//! shapes they assert are the ones production produces.

use std::path::Path;
use std::path::PathBuf;

use monostyle::aggregate::FileScore;
use monostyle::aggregate::rank_file_impact;
use monostyle::aggregate::rank_overall_impact;
use monostyle::aggregate::rank_rule_impact;
use monostyle::analysis::AnalysisOptions;
use monostyle::analysis::FileReport;
use monostyle::analysis::FloorViolation;
use monostyle::analysis::analyze_with_language;
use monostyle::report::render_file_line;
use monostyle::report::render_project_github;
use monostyle_core::Language;

/// The deliberately-bad fixture, which produces findings by design.
const BAD: &str = include_str!("fixtures/bad/rust.rs");

/// Analyzes a Rust snippet as a file named `sample.rs`.
fn analyze(source: &str) -> FileReport {
	analyze_with_language(
		Path::new("sample.rs"),
		source,
		Language::Rust,
		&AnalysisOptions::default(),
	)
}

#[test]
fn a_unit_report_grades_by_its_score() {
	let report = analyze("fn simple() -> u8 {\n    1 + 2\n}\n");

	assert!(
		!report.units.is_empty(),
		"the report should carry unit reports"
	);

	let unit = &report.units[0];

	assert_eq!(unit.grade(), "excellent");
}

#[test]
fn ranked_findings_order_by_penalty() {
	let report = analyze(BAD);
	let ranked = report.ranked_findings();

	assert!(!ranked.is_empty(), "the bad fixture produces findings");

	let penalties: Vec<f64> = ranked.iter().map(|finding| finding.penalty()).collect();
	let mut sorted = penalties.clone();
	sorted.sort_by(|left, right| right.partial_cmp(left).unwrap_or(std::cmp::Ordering::Equal));

	assert_eq!(
		penalties, sorted,
		"findings should run from most to least costly"
	);
}

#[test]
fn floor_violation_summarizes_each_category() {
	let both = FloorViolation {
		path: PathBuf::from("a.rs"),
		readability: 40.0,
		complexity: 50.0,
		readability_floor: Some(70.0),
		complexity_floor: Some(70.0),
		reason: None,
		top_offenders: Vec::new(),
	};

	assert!(both.is_below());
	assert_eq!(
		both.summary(),
		"readability 40.0 is 30.0 below the 70.0 floor; complexity 50.0 is 20.0 below the 70.0 floor"
	);

	let one = FloorViolation {
		path: PathBuf::from("b.rs"),
		readability: 40.0,
		complexity: 100.0,
		readability_floor: Some(70.0),
		complexity_floor: None,
		reason: None,
		top_offenders: Vec::new(),
	};

	assert!(one.is_below());
	assert!(one.summary().starts_with("readability 40.0 is"));

	let neither = FloorViolation {
		path: PathBuf::from("c.rs"),
		readability: 90.0,
		complexity: 90.0,
		readability_floor: None,
		complexity_floor: None,
		reason: None,
		top_offenders: Vec::new(),
	};

	assert!(!neither.is_below(), "no floors set means nothing is below");
	assert_eq!(neither.summary(), "");
}

#[test]
fn findings_are_counted_by_rule() {
	let counted = project_of(analyze(BAD)).findings_by_rule();

	assert!(!counted.is_empty(), "penalizing findings are counted");
	assert!(
		counted.iter().all(|(_, count)| *count > 0),
		"every entry carries at least one finding"
	);

	let counts: Vec<usize> = counted.iter().map(|(_, count)| *count).collect();
	let mut sorted = counts.clone();
	sorted.sort_by(|left, right| right.cmp(left));

	assert_eq!(counts, sorted, "rules run from most to least frequent");
}

#[test]
fn the_worst_files_come_first() {
	let good = analyze("fn fine() -> u8 {\n    1\n}\n");
	let bad = analyze(BAD);
	let project = project_of(bad);

	let mut project = project;
	project.files.push(good);

	let worst = project.worst_files(2);

	assert_eq!(worst.len(), 2);
	assert_eq!(worst[0].path, PathBuf::from("sample.rs"));
	assert!(
		worst[0].overall() <= worst[1].overall(),
		"the most problematic file comes first"
	);
}

#[test]
fn a_finding_with_credit_is_credited() {
	// A comment documenting reasoning earns credit rather than costing points.
	let report = analyze(
		"/// A doc comment explaining the reasoning behind this module.\nfn documented() -> u8 {\n    1\n}\n",
	);
	let project = project_of(report);
	let credited = project.credited_findings();

	// Whatever the rule set credits, the method must only ever return non-positive penalties.
	assert!(
		credited.iter().all(|(_, finding)| finding.penalty() < 0.0),
		"credited findings must carry negative penalties"
	);
}

#[test]
fn section_floors_override_the_run_wide_floor() {
	use monostyle::analysis::AnalysisOptions;
	use monostyle::section::Section;

	let mut options = AnalysisOptions::default();
	options.fail_under.readability = Some(90.0);
	options.sections.push(Section {
		path: "tests".into(),
		rules: monostyle_rules::RulesConfig::default(),
		fail_under: monostyle::section::ScoreFloor {
			readability: Some(60.0),
			complexity: Some(60.0),
		},
		ignore: false,
		reason: Some("tests are held to a different bar".into()),
	});

	let inside = Path::new("tests/sample.rs");
	let outside = Path::new("src/sample.rs");

	// The floor for a path under the section is the section's floor; elsewhere it is the run's.
	let floor_inside = options.floor_for(inside);
	let floor_outside = options.floor_for(outside);

	assert_eq!(floor_inside.readability, Some(60.0));
	assert_eq!(floor_outside.readability, Some(90.0));
}

#[test]
fn github_annotations_escape_and_carry_locations() {
	let rendered = render_project_github(&project_of(analyze(BAD)));

	assert!(
		rendered.contains("::warning file="),
		"minor findings render as warnings: {rendered}"
	);
	assert!(
		rendered.contains("line="),
		"every annotation carries a line"
	);
	assert!(
		!rendered.contains('\r'),
		"workflow command bodies must not contain raw carriage returns"
	);
}

/// Builds a project report around one file report, with its aggregate fields derived.
fn project_of(report: FileReport) -> monostyle::analysis::ProjectReport {
	let score = report.to_file_score();

	monostyle::analysis::ProjectReport {
		files: vec![report],
		readability: score.readability,
		complexity: score.complexity,
		total_lines: score.code_lines,
		code_lines: score.code_lines,
		skipped: Vec::new(),
		packages: Vec::new(),
		floor_violations: Vec::new(),
		floor_configured: false,
	}
}

#[test]
fn a_file_line_names_the_file_and_its_scores() {
	let report = analyze("fn fine() -> u8 {\n    1\n}\n");
	let line = render_file_line(&report);

	assert!(
		line.contains("sample.rs") && line.contains("readability"),
		"the summary line names the file and its scores: {line}"
	);
}

#[test]
fn file_scores_rank_by_total_penalty() {
	let scoring = monostyle_core::ScoringConfig::default();

	let scores = vec![
		FileScore {
			path: PathBuf::from("light.rs"),
			language: Language::Rust,
			code_lines: 10,
			readability: monostyle_core::Score::from_density(0.0, 0.0, scoring),
			complexity: monostyle_core::Score::from_density(0.0, 0.0, scoring),
			total_penalty: 1.0,
		},
		FileScore {
			path: PathBuf::from("heavy.rs"),
			language: Language::Rust,
			code_lines: 10,
			readability: monostyle_core::Score::from_density(30.0, 30.0, scoring),
			complexity: monostyle_core::Score::from_density(30.0, 30.0, scoring),
			total_penalty: 9.0,
		},
	];

	let ranked = rank_file_impact(&scores);

	assert_eq!(ranked[0].path, PathBuf::from("heavy.rs"));
}

#[test]
fn findings_rank_into_rule_impacts() {
	let first = analyze(BAD);
	let second = analyze(BAD);

	let owned: Vec<(PathBuf, monostyle_core::Finding)> = vec![
		(PathBuf::from("one.rs"), first),
		(PathBuf::from("two.rs"), second),
	]
	.into_iter()
	.flat_map(|(path, report)| {
		report
			.findings
			.into_iter()
			.map(move |finding| (path.clone(), finding))
	})
	.collect();

	let overall = rank_overall_impact(&owned);

	assert!(!overall.is_empty(), "findings rank into impacts");

	let readability = rank_rule_impact(&owned, monostyle_core::Category::Readability);

	assert!(!readability.is_empty(), "readability impacts are ranked");
}

#[test]
fn an_unknown_language_has_no_report() {
	let report = monostyle::analysis::analyze_source(
		Path::new("data.unknown-ext"),
		"whatever",
		&AnalysisOptions::default(),
	);

	assert!(report.is_none(), "unknown extensions are not analyzed");
}

#[test]
fn an_unterminated_scan_reports_its_construct() {
	let report = analyze("/* a comment that never closes\n");

	assert!(
		!report.unterminated.is_empty(),
		"the open block comment should be reported"
	);
}

/// Converts a report to its aggregate form so the conversion stays exercised.
#[test]
fn a_file_report_converts_to_a_score() {
	let report = analyze("fn fine() -> u8 {\n    1\n}\n");
	let score = report.to_file_score();

	assert_eq!(score.path, report.path);
	assert_eq!(score.code_lines, report.code_lines);
}
