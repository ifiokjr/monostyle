//! Aggregation tests.
//!
//! The property these tests pin is the one that makes a project score meaningful: a large file
//! must weigh more than a small one. A plain per-file average fails that, and the failure is easy
//! to ship unnoticed because both formulas look reasonable on a repository where every file is
//! about the same size.

use std::path::PathBuf;

use monostyle::aggregate::FileScore;
use monostyle::aggregate::project_score;
use monostyle_core::Category;
use monostyle_core::Language;
use monostyle_core::Score;
use monostyle_core::ScoringConfig;

/// Builds a file score with a given penalty spread over a given line count.
fn file(name: &str, code_lines: usize, penalty: f64) -> FileScore {
	let category_config = ScoringConfig::default();
	let readability = Score::from_findings(&[], Category::Readability, code_lines, category_config);

	FileScore {
		path: PathBuf::from(name),
		language: Language::Rust,
		code_lines,
		// The penalty is carried directly so the test controls it exactly.
		readability: Score {
			penalty,
			..readability
		},
		complexity: Score::perfect(),
		total_penalty: penalty,
	}
}

#[test]
fn a_large_file_outweighs_a_small_one() {
	// The case from the requirement: a 1-line file with the worst possible penalty, and a
	// 100-line file scoring 70. A plain average would land near 35; a line-weighted score must
	// land far closer to the large file's value.
	let small = file("small.rs", 1, 100.0);
	let large = file("large.rs", 100, 1.0);

	let weighted = project_score(
		&[small, large],
		Category::Readability,
		ScoringConfig::default(),
	);

	// The unweighted average of the two densities would be enormous; the weighted score must be
	// dominated by the large file's much lower density.
	assert!(
		weighted.value > 60.0,
		"a 1-line file should barely move a 100-line file's score, got {:.1}",
		weighted.value
	);
}

#[test]
fn a_projects_score_matches_a_single_file_with_the_same_density() {
	// Splitting one file into two must not change the project's score, because the total penalty
	// and the total volume are unchanged. This is the invariant that makes the formula a weighted
	// mean rather than something else that happens to look like one.
	let whole = vec![file("whole.rs", 200, 8.0)];
	let split = vec![file("a.rs", 100, 4.0), file("b.rs", 100, 4.0)];

	let whole_score = project_score(&whole, Category::Readability, ScoringConfig::default());

	let split_score = project_score(&split, Category::Readability, ScoringConfig::default());

	assert!(
		(whole_score.value - split_score.value).abs() < 0.01,
		"splitting a file changed the project score: {:.1} vs {:.1}",
		whole_score.value,
		split_score.value
	);
}

#[test]
fn a_tiny_file_does_not_dominate_the_total() {
	let many_small: Vec<FileScore> = (0..50)
		.map(|index| file(&format!("s{index}.rs"), 2, 20.0))
		.collect();
	let one_large = file("large.rs", 5000, 0.0);

	let mut files = many_small;
	files.push(one_large);

	let score = project_score(&files, Category::Readability, ScoringConfig::default());

	// The large clean file is 98% of the volume, so even fifty badly-scoring small files cannot
	// drag the project far below the clean file's own score.
	assert!(
		score.value > 70.0,
		"fifty two-line files should not outweigh one 5000-line file, got {:.1}",
		score.value
	);
}

#[test]
fn an_empty_project_scores_perfectly() {
	let score = project_score(&[], Category::Readability, ScoringConfig::default());

	assert_eq!(score.value, 100.0, "nothing analyzed means nothing wrong");
}

#[test]
fn penalties_accumulate_across_files() {
	let one = vec![file("a.rs", 100, 5.0)];
	let two = vec![file("a.rs", 100, 5.0), file("b.rs", 100, 5.0)];

	let single = project_score(&one, Category::Readability, ScoringConfig::default());

	let double = project_score(&two, Category::Readability, ScoringConfig::default());

	// Doubling both the penalty and the volume leaves the density unchanged, so the score must be
	// identical. This is what makes density the right unit for comparison.
	assert!(
		(single.value - double.value).abs() < 0.01,
		"proportional growth changed the score: {:.1} vs {:.1}",
		single.value,
		double.value
	);
}

#[test]
fn overall_impact_uses_one_denominator_across_categories() {
	// The bug this pins: per-category shares are computed against that category's total, so
	// combining two ranked lists and quoting one of their percentages as a whole-project figure
	// overstates the benefit of the fix. A rule holding most of one category may hold a third of
	// the total.
	use monostyle_core::Category;
	use monostyle_core::Finding;
	use monostyle_core::Severity;
	use monostyle_core::Span;

	/// Builds a finding with an explicit category and penalty.
	fn finding(rule: &str, category: Category, weight: f64) -> Finding {
		Finding {
			rule: rule.to_string(),
			category,
			severity: Severity::Minor,
			span: Span::new(0, 0, 1, 1),
			message: String::new(),
			suggestion: String::new(),
			weight,
			fix: None,
		}
	}

	let findings = vec![
		(
			PathBuf::from("a.rs"),
			finding("complexity/expensive", Category::Complexity, 90.0),
		),
		(
			PathBuf::from("b.rs"),
			finding("readability/cheap", Category::Readability, 10.0),
		),
	];

	let impacts = monostyle::aggregate::rank_overall_impact(&findings);
	let top = impacts.first().expect("one rule should rank first");

	// 90 of 100 total penalty is 90%, not the 100% the complexity-only denominator would give.
	assert!(
		(top.share * 100.0 - 90.0).abs() < 0.01,
		"the top rule's share should be 90% of the total, got {:.1}%",
		top.share * 100.0
	);

	let total: f64 = impacts.iter().map(|impact| impact.share).sum();

	assert!(
		(total - 1.0).abs() < 0.01,
		"shares should sum to 1 across categories, got {total:.2}"
	);
}
