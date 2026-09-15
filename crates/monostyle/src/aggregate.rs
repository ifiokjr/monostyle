//! Line-weighted aggregation and impact attribution.
//!
//! # Why scores are weighted by lines, not averaged
//!
//! A per-file average treats every file as equally representative, which produces nonsense on a
//! real repository: a ten-line file scoring 0 would drag a project down as much as a
//! thousand-line file scoring 0, even though the former is a rounding error and the latter is
//! the repository.
//!
//! Aggregation therefore works the way coverage reporting does — weighted by the code volume each
//! score was computed over:
//!
//! ```text
//! project_penalty_density = Σ penalties / Σ lines of code
//! ```
//!
//! Penalties are summed rather than densities averaged, because summing penalties and dividing by
//! the total volume *is* the volume-weighted mean. That single expression handles both the file
//! and project case, so a file's score and the project's score are computed by the same rule
//! rather than by two definitions that have to be kept in agreement.
//!
//! # Why impact is reported, not just score
//!
//! A score tells you something is wrong; it does not tell you what to fix. Every finding therefore
//! records its penalty, and a [`ProjectReport`](crate::analysis::ProjectReport) can rank findings
//! by how much of the total penalty each one accounts for. That is what turns a report into a
//! worklist an agent or a person can act on in order.

use std::path::PathBuf;

use monostyle_core::Category;
use monostyle_core::Finding;
use monostyle_core::Language;
use monostyle_core::Score;
use monostyle_core::ScoringConfig;
use serde::Serialize;

/// A rule's contribution to a category's penalty.
#[derive(Debug, Clone, Serialize)]
pub struct RuleImpact {
	/// The rule name.
	pub rule: String,
	/// Total penalty this rule contributed.
	pub penalty: f64,
	/// How many findings the rule raised.
	pub count: usize,
	/// Share of the category's total penalty, from 0 to 1.
	pub share: f64,
	/// The penalty of the costliest single finding, tracked so the reported message is the one
	/// from the worst offender rather than from whichever finding happened to be last.
	#[serde(skip)]
	pub worst_penalty: f64,
	/// Lines of code at the worst offender, so a reader can gauge the scale of one fix.
	pub worst_offender: Option<OffenderLocation>,
	/// The explanation from the highest-penalty finding.
	pub message: String,
	/// The remedy from the highest-penalty finding.
	pub suggestion: String,
}

/// Where a finding lives.
#[derive(Debug, Clone, Serialize)]
pub struct OffenderLocation {
	/// The file the finding was in.
	pub path: PathBuf,
	/// The 1-based line.
	pub line: usize,
	/// The severity label.
	pub severity: String,
}

/// A scored file, reduced to what aggregation and reporting need.
#[derive(Debug, Clone, Serialize)]
pub struct FileScore {
	/// The file's path, relative to the analysis root where possible.
	pub path: PathBuf,
	/// The detected language.
	pub language: Language,
	/// Lines of code the score was computed over.
	pub code_lines: usize,
	/// Readability score for this file.
	pub readability: Score,
	/// Complexity score for this file.
	pub complexity: Score,
	/// Total penalty across both categories.
	pub total_penalty: f64,
}

impl FileScore {
	/// The combined score, out of 100.
	#[must_use]
	pub fn overall(&self) -> f64 {
		f64::midpoint(self.readability.value, self.complexity.value)
	}
}

/// Computes a score for a whole project from its files.
///
/// The penalty is summed across every file and divided by the total code volume, which is the
/// volume-weighted mean of the per-file densities. This is what stops a tiny file from
/// contributing as much as a large one.
#[must_use]
pub fn project_score(files: &[FileScore], category: Category, config: ScoringConfig) -> Score {
	if files.is_empty() {
		return Score::perfect();
	}

	// Each file contributes its own penalty density, weighted by the lines of code that density
	// was computed over:
	//
	//     project_density = Σ(density_i × lines_i) / Σ lines_i
	//
	// Summing raw penalties and dividing once would be simpler, and it is wrong: a file's density
	// is computed against a floored denominator, so a one-line file with a single finding carries
	// an enormous density that is not proportional to its size. Summing penalties lets that file
	// dominate the project — the opposite of weighting by lines, which is the whole point.
	let total_lines: f64 = files.iter().map(|file| file.code_lines as f64).sum();
	let weighted: f64 = files
		.iter()
		.map(|file| {
			let density = match category {
				Category::Readability => file.readability.density,
				Category::Complexity => file.complexity.density,
			};

			density * file.code_lines as f64
		})
		.sum();

	let penalty: f64 = files
		.iter()
		.map(|file| category_penalty(file, category))
		.sum();
	let density = if total_lines > 0.0 {
		weighted / total_lines
	} else {
		0.0
	};

	Score::from_density(density, penalty, config)
}

/// Returns the penalty a file contributed to a category.
fn category_penalty(file: &FileScore, category: Category) -> f64 {
	match category {
		Category::Readability => file.readability.penalty,
		Category::Complexity => file.complexity.penalty,
	}
}

/// Ranks rules by how much of a category's penalty each accounts for.
///
/// This is the actionable half of a report: the first entry is the single change that would
/// recover the most points.
#[must_use]
pub fn rank_rule_impact(findings: &[(PathBuf, Finding)], category: Category) -> Vec<RuleImpact> {
	let mut impacts: Vec<RuleImpact> = Vec::new();
	let mut total: f64 = 0.0;

	for (path, finding) in findings {
		if finding.category != category {
			continue;
		}

		let penalty = finding.penalty();

		// Credit entries are not impacts: they raise the score rather than detracting from it.
		if penalty <= 0.0 {
			continue;
		}

		total += penalty;

		match impacts
			.iter_mut()
			.find(|impact| impact.rule == finding.rule)
		{
			Some(impact) => {
				// The message kept is the one from the costliest finding, because that is what a
				// reader should act on first. It is captured before the total is updated, so the
				// comparison is against the previous worst rather than the new sum.
				if penalty >= impact.worst_penalty {
					impact.worst_penalty = penalty;
					impact.message.clone_from(&finding.message);
					impact.suggestion.clone_from(&finding.suggestion);
					impact.worst_offender = Some(OffenderLocation {
						path: path.clone(),
						line: finding.span.start_line,
						severity: finding.severity.label().to_string(),
					});
				}

				impact.penalty += penalty;
				impact.count += 1;
			}
			None => {
				impacts.push(RuleImpact {
					rule: finding.rule.clone(),
					penalty,
					count: 1,
					share: 0.0,
					worst_penalty: penalty,
					worst_offender: Some(OffenderLocation {
						path: path.clone(),
						line: finding.span.start_line,
						severity: finding.severity.label().to_string(),
					}),
					message: finding.message.clone(),
					suggestion: finding.suggestion.clone(),
				});
			}
		}
	}

	for impact in &mut impacts {
		impact.share = if total > 0.0 {
			impact.penalty / total
		} else {
			0.0
		};
	}

	impacts.sort_by(|left, right| {
		right
			.penalty
			.partial_cmp(&left.penalty)
			.unwrap_or(std::cmp::Ordering::Equal)
	});
	impacts
}

/// Ranks files by the penalty they contribute, most costly first.
///
/// Reported alongside the per-file score so a reader can tell the difference between a file that
/// scores badly and a file that is actually costing the project.
#[must_use]
pub fn rank_file_impact(files: &[FileScore]) -> Vec<FileScore> {
	let mut ranked = files.to_vec();

	ranked.sort_by(|left, right| {
		right
			.total_penalty
			.partial_cmp(&left.total_penalty)
			.unwrap_or(std::cmp::Ordering::Equal)
	});
	ranked
}
