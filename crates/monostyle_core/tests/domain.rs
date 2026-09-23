//! Tests for the core domain types.
//!
//! These are small types with large consequences: a severity's multiplier and the score curve decide
//! what every number in a report means, so they are pinned rather than left to be discovered.

use monostyle_core::Category;
use monostyle_core::Finding;
use monostyle_core::FindingBuilder;
use monostyle_core::Fix;
use monostyle_core::Language;
use monostyle_core::Score;
use monostyle_core::ScoringConfig;
use monostyle_core::Severity;
use monostyle_core::Span;

/// Builds a finding with a given category, severity, and weight.
fn finding(category: Category, severity: Severity, weight: f64) -> Finding {
	FindingBuilder::new("test/rule", category, Span::new(0, 1, 1, 1))
		.severity(severity)
		.weight(weight)
		.message("a message")
		.suggestion("a suggestion")
		.build()
}

// ---------------------------------------------------------------------------
// Severity
// ---------------------------------------------------------------------------
#[test]
fn severity_multipliers_are_ordered() {
	assert!(Severity::Info.penalty_factor() < Severity::Minor.penalty_factor());
	assert!(Severity::Minor.penalty_factor() < Severity::Major.penalty_factor());
	assert!(Severity::Major.penalty_factor() < Severity::Critical.penalty_factor());
}

#[test]
fn info_never_costs_points() {
	// A finding that is reported but not scored is the point of this severity, so it must not move a
	// score even with a large weight.
	assert_eq!(Severity::Info.penalty_factor(), 0.0);
	assert_eq!(
		finding(Category::Readability, Severity::Info, 1000.0).penalty(),
		0.0
	);
}

#[test]
fn severity_labels_are_lowercase() {
	for severity in [
		Severity::Info,
		Severity::Minor,
		Severity::Major,
		Severity::Critical,
	] {
		assert_eq!(severity.label(), severity.label().to_lowercase());
	}
}

#[test]
fn severity_orders_from_least_to_most_serious() {
	assert!(Severity::Info < Severity::Minor);
	assert!(Severity::Minor < Severity::Major);
	assert!(Severity::Major < Severity::Critical);
}

// ---------------------------------------------------------------------------
// Findings
// ---------------------------------------------------------------------------
#[test]
fn a_finding_penalty_is_its_weight_scaled_by_severity() {
	let finding = finding(Category::Readability, Severity::Major, 2.0);

	assert_eq!(finding.penalty(), 2.0 * Severity::Major.penalty_factor());
}

#[test]
fn a_negative_weight_produces_credit() {
	// Credit for a good comment is modelled as a negative penalty so one number explains a score.
	let credit = finding(Category::Readability, Severity::Minor, -0.5);

	assert!(credit.penalty() < 0.0);
}

#[test]
fn a_builder_defaults_to_minor_and_reports_itself() {
	let finding =
		FindingBuilder::new("test/rule", Category::Readability, Span::new(0, 0, 1, 1)).build();

	assert_eq!(finding.severity, Severity::Minor);
	assert_eq!(finding.weight, 1.0);
	// An under-explained finding is a reporting weakness, not a reason to abandon a run, so the builder
	// substitutes readable defaults rather than panicking.
	assert_ne!(finding.message, "");
	assert_ne!(finding.suggestion, "");
}

#[test]
fn a_finding_can_carry_a_fix() {
	let fix = Fix::replace(Span::new(0, 5, 1, 1), "replacement", "test");

	let finding = FindingBuilder::new("test/rule", Category::Readability, Span::new(0, 5, 1, 1))
		.fix(fix.clone())
		.build();

	assert_eq!(finding.fix, Some(fix));
}

#[test]
fn a_finding_without_a_fix_reports_none() {
	let finding =
		FindingBuilder::new("test/rule", Category::Readability, Span::new(0, 5, 1, 1)).build();

	assert!(finding.fix.is_none());
}

#[test]
fn findings_serialize_their_explanation() {
	let finding = finding(Category::Complexity, Severity::Critical, 3.0);
	let encoded = serde_json::to_string(&finding).expect("encoding");

	assert!(encoded.contains("\"rule\":\"test/rule\""));
	assert!(encoded.contains("\"category\":\"complexity\""));
	assert!(encoded.contains("\"severity\":\"critical\""));
	assert!(encoded.contains("a message"));
	assert!(encoded.contains("a suggestion"));
}

#[test]
fn an_absent_fix_is_omitted_from_json() {
	let encoded = serde_json::to_string(&finding(Category::Readability, Severity::Minor, 1.0))
		.expect("encoding");

	assert!(
		!encoded.contains("fix"),
		"an absent fix should be omitted: {encoded}"
	);
}

// ---------------------------------------------------------------------------
// Categories
// ---------------------------------------------------------------------------
#[test]
fn categories_have_labels_matching_their_serialized_form() {
	for category in Category::ALL {
		let encoded = serde_json::to_string(&category).expect("encoding");

		assert_eq!(encoded, format!("\"{}\"", category.label()));
	}
}

#[test]
fn there_are_exactly_two_categories() {
	assert_eq!(Category::ALL.len(), 2);
	assert!(Category::ALL.contains(&Category::Readability));
	assert!(Category::ALL.contains(&Category::Complexity));
}

// ---------------------------------------------------------------------------
// Scoring
// ---------------------------------------------------------------------------
#[test]
fn a_clean_body_scores_perfectly() {
	let score = Score::from_findings(&[], Category::Readability, 100, ScoringConfig::default());

	assert_eq!(score.value, Score::PERFECT);
	assert_eq!(score.penalty, 0.0);
	assert_eq!(score.density, 0.0);
}

#[test]
fn the_half_life_is_the_density_that_scores_fifty() {
	// This is the property that makes the curve tunable with one readable number, so it is asserted
	// rather than assumed.
	let config = ScoringConfig {
		half_life: 12.0,
		min_normalization_lines: 100.0,
	};

	// A density of 12 per hundred lines means 12 penalty over 100 lines.
	let findings = vec![finding(Category::Readability, Severity::Minor, 12.0)];
	let score = Score::from_findings(&findings, Category::Readability, 100, config);

	assert!(
		(score.value - 50.0).abs() < 0.01,
		"a density equal to the half-life should score 50, got {:.2}",
		score.value
	);
}

#[test]
fn finding_the_density_is_reported_alongside_the_value() {
	let findings = vec![finding(Category::Readability, Severity::Minor, 12.0)];
	let score = Score::from_findings(
		&findings,
		Category::Readability,
		100,
		ScoringConfig::default(),
	);

	assert!(
		(score.density - 12.0).abs() < 0.01,
		"density should be the penalty per hundred lines"
	);
	assert!((score.penalty - 12.0).abs() < 0.01);
}

#[test]
fn a_negative_density_clamps_to_perfect() {
	// Credit can outweigh penalties, which is a valid state that should not produce a score above 100.
	let findings = vec![finding(Category::Readability, Severity::Minor, -50.0)];
	let score = Score::from_findings(
		&findings,
		Category::Readability,
		100,
		ScoringConfig::default(),
	);

	assert_eq!(score.value, Score::PERFECT);
}

#[test]
fn only_the_requested_category_is_counted() {
	let findings = vec![
		finding(Category::Readability, Severity::Minor, 10.0),
		finding(Category::Complexity, Severity::Minor, 1000.0),
	];

	let readability = Score::from_findings(
		&findings,
		Category::Readability,
		100,
		ScoringConfig::default(),
	);

	assert_eq!(
		readability.penalty, 10.0,
		"the complexity finding must not affect readability"
	);
}

#[test]
fn a_small_body_is_normalized_against_the_floor() {
	// Without a floor, a five-line function with one finding would produce an enormous density.
	let config = ScoringConfig {
		half_life: 12.0,
		min_normalization_lines: 100.0,
	};
	let findings = vec![finding(Category::Readability, Severity::Minor, 12.0)];

	let small = Score::from_findings(&findings, Category::Readability, 1, config);
	let exact = Score::from_findings(&findings, Category::Readability, 100, config);

	assert_eq!(
		small.value, exact.value,
		"a body below the floor uses the floor"
	);
}

#[test]
fn more_penalty_never_raises_a_score() {
	let config = ScoringConfig::default();
	let mut previous = Score::PERFECT;

	for penalty in [0.0, 1.0, 5.0, 20.0, 100.0, 1000.0] {
		let findings = vec![finding(Category::Readability, Severity::Minor, penalty)];
		let score = Score::from_findings(&findings, Category::Readability, 100, config);

		assert!(
			score.value <= previous + 0.01,
			"score rose as penalty increased"
		);
		previous = score.value;
	}
}

#[test]
fn a_zero_half_life_falls_back_to_the_default() {
	// A zero would divide by zero, so the curve substitutes the default rather than producing an
	// infinite density.
	let config = ScoringConfig {
		half_life: 0.0,
		min_normalization_lines: 20.0,
	};
	let findings = vec![finding(Category::Readability, Severity::Minor, 5.0)];

	let score = Score::from_findings(&findings, Category::Readability, 100, config);

	assert!(score.value.is_finite());
	assert!(score.value > 0.0);
}

#[test]
fn strict_and_lenient_bracket_the_default() {
	let default = ScoringConfig::default();

	assert!(ScoringConfig::strict().half_life < default.half_life);

	assert!(ScoringConfig::lenient().half_life > default.half_life);
}

#[test]
fn grades_span_the_range() {
	/// A score and the band it should fall in.
	const CASES: &[(f64, &str)] = &[
		(100.0, "excellent"),
		(90.0, "excellent"),
		(89.9, "good"),
		(75.0, "good"),
		(74.9, "fair"),
		(60.0, "fair"),
		(59.9, "poor"),
		(40.0, "poor"),
		(39.9, "bad"),
		(0.0, "bad"),
	];

	for (value, expected) in CASES {
		let score = Score {
			value: *value,
			penalty: 0.0,
			density: 0.0,
		};

		assert_eq!(
			score.grade(),
			*expected,
			"{value} should grade as {expected}"
		);
	}
}

#[test]
fn a_perfect_score_displays_with_one_decimal() {
	let score = Score {
		value: 100.0,
		penalty: 0.0,
		density: 0.0,
	};

	assert_eq!(score.to_string(), "100.0");
}

#[test]
fn scores_serialize_with_their_components() {
	let score = Score {
		value: 82.5,
		penalty: 4.0,
		density: 3.3,
	};
	let encoded = serde_json::to_string(&score).expect("encoding");

	assert!(encoded.contains("\"value\":82.5"));
	assert!(encoded.contains("\"penalty\":4.0"));
	assert!(encoded.contains("\"density\":3.3"));
}

// ---------------------------------------------------------------------------
// Spans
// ---------------------------------------------------------------------------
#[test]
fn a_span_reports_its_line_count_inclusively() {
	assert_eq!(Span::new(0, 10, 1, 1).line_count(), 1);
	assert_eq!(Span::new(0, 10, 1, 5).line_count(), 5);
}

#[test]
fn a_single_line_span_has_a_line_count_of_one() {
	// Counting inclusively matters because every finding reports the range it covers.
	assert_eq!(Span::new(5, 5, 7, 7).line_count(), 1);
}

#[test]
fn a_span_serializes_its_offsets_and_lines() {
	let encoded = serde_json::to_string(&Span::new(0, 10, 1, 2)).expect("encoding");

	assert!(encoded.contains("\"start_byte\":0"));
	assert!(encoded.contains("\"end_byte\":10"));
	assert!(encoded.contains("\"start_line\":1"));
	assert!(encoded.contains("\"end_line\":2"));
}

// ---------------------------------------------------------------------------
// Fixes
// ---------------------------------------------------------------------------
#[test]
fn a_replace_fix_covers_its_range() {
	let fix = Fix::replace(Span::new(0, 5, 1, 1), "text", "a test edit");

	assert_eq!(fix.span.start_byte, 0);
	assert_eq!(fix.span.end_byte, 5);
	assert_eq!(fix.replacement, "text");
	assert!(!fix.is_empty());
}

#[test]
fn an_insert_fix_has_no_width() {
	let fix = Fix::insert(Span::new(5, 5, 1, 1), "\n", "a blank line");

	assert_eq!(fix.span.start_byte, fix.span.end_byte);
	assert!(
		!fix.is_empty(),
		"an insertion is not empty even though it has no width"
	);
}

#[test]
fn a_delete_fix_has_no_replacement() {
	let fix = Fix::delete(Span::new(0, 5, 1, 1), "remove");

	assert_eq!(fix.replacement, "");
	assert!(!fix.is_empty());
}

#[test]
fn a_zero_width_empty_fix_is_empty() {
	// The fixer filters these out, so recognizing one matters.
	let fix = Fix::replace(Span::new(3, 3, 1, 1), "", "no-op");

	assert!(fix.is_empty());
}

// ---------------------------------------------------------------------------
// Languages
// ---------------------------------------------------------------------------
#[test]
fn languages_display_as_their_names() {
	assert_eq!(Language::Rust.to_string(), "rust");
	assert_eq!(Language::Dart.to_string(), "dart");
}

#[test]
fn languages_serialize_as_their_documented_names() {
	// The serialized form is what a consumer matches on and what the CLI accepts, so it must be the same
	// spelling the readme lists. A default rename policy produced `type-script` and `c-sharp`, neither of
	// which the tool accepts on the command line.
	for language in Language::ALL {
		let encoded = serde_json::to_string(&language).expect("encoding");

		assert_eq!(
			encoded,
			format!("\"{}\"", language.name()),
			"{language}: serialized form should match its name"
		);
	}
}

#[test]
fn language_names_round_trip_through_serde() {
	// A name that serializes one way and deserializes another would corrupt a cache entry silently.
	for language in Language::ALL {
		let encoded = serde_json::to_string(&language).expect("encoding");
		let decoded: Language = serde_json::from_str(&encoded).expect("decoding");

		assert_eq!(decoded, language);
	}
}

#[test]
fn every_language_declares_itself_c_family_or_not() {
	// The predicate drives parameter-list detection, so it must be total and consistent with the block
	// style rather than guessed per language.
	assert!(Language::Rust.is_c_family());
	assert!(Language::Dart.is_c_family());
	assert!(!Language::Python.is_c_family());
	assert!(!Language::Markdown.is_c_family());
}

#[test]
fn the_language_list_is_large_enough_to_be_useful() {
	// The count is stated in the readme, so a language accidentally dropped from the list would make
	// the documentation wrong.
	assert!(Language::ALL.len() >= 23, "expected at least 23 languages");
}
