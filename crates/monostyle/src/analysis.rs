//! Analysis pipeline.
//!
//! This is where metrics, rules, and scoring meet. A file is lexed once, measured, run
//! through the rule set, and turned into a [`FileReport`]; a set of files is aggregated into
//! a [`ProjectReport`].
//!
//! # Why scoring happens here and not in the rules
//!
//! Rules describe what they found; this module decides what it costs. Keeping the
//! conversion in one place means every report — per function, per file, per project — divides
//! by the same denominators and applies the same curve, so a function's score and its file's
//! score are directly comparable rather than merely similar.

use std::path::Path;
use std::path::PathBuf;

use monostyle_core::Category;
use monostyle_core::Finding;
use monostyle_core::Language;
use monostyle_core::Score;
use monostyle_core::ScoringConfig;
use monostyle_lexer::LexedFile;
use monostyle_lexer::lex;
use monostyle_metrics::CodeUnit;
use monostyle_rules::RulesConfig;
use serde::Serialize;

/// A scored function-like unit.
#[derive(Debug, Clone, Serialize)]
pub struct UnitReport {
	/// The unit's name.
	pub name: String,
	/// 1-based line the unit starts on.
	pub start_line: usize,
	/// 1-based line the unit ends on.
	pub end_line: usize,
	/// Lines of code in the unit.
	pub lines: usize,
	/// Cyclomatic complexity.
	pub cyclomatic: usize,
	/// Cyclomatic risk band.
	pub cyclomatic_risk: String,
	/// Cognitive complexity.
	pub cognitive: usize,
	/// Cognitive complexity band.
	pub cognitive_grade: String,
	/// How many points came from nesting.
	pub nesting_penalty: usize,
	/// Deepest nesting level reached.
	pub max_nesting: usize,
	/// Readability score for this unit.
	pub readability: Score,
	/// Complexity score for this unit.
	pub complexity: Score,
	/// Findings attributed to this unit.
	pub findings: Vec<Finding>,
}

impl UnitReport {
	/// The unit's combined score, out of 100.
	///
	/// A unit with no findings is clean rather than unmeasured, so it scores perfectly instead of
	/// dividing by an empty penalty set.
	#[must_use]
	pub fn score(&self) -> f64 {
		if self.findings.is_empty() {
			return Score::PERFECT;
		}

		f64::midpoint(self.readability.value, self.complexity.value)
	}

	/// The unit's overall grade.
	#[must_use]
	pub fn grade(&self) -> &'static str {
		match self.score() {
			value if value >= 90.0 => "excellent",
			value if value >= 75.0 => "good",
			value if value >= 60.0 => "fair",
			value if value >= 40.0 => "poor",
			_ => "bad",
		}
	}
}

/// A scored file.
#[derive(Debug, Clone, Serialize)]
pub struct FileReport {
	/// Path as reported, relative to the analysis root where possible.
	pub path: PathBuf,
	/// Detected language.
	pub language: Language,
	/// Total physical lines.
	pub total_lines: usize,
	/// Lines of code, excluding blanks, comments, and literal content.
	pub code_lines: usize,
	/// Comment lines.
	pub comment_lines: usize,
	/// Blank lines.
	pub blank_lines: usize,
	/// Whole-file cyclomatic complexity.
	pub cyclomatic: usize,
	/// Whole-file cognitive complexity.
	pub cognitive: usize,
	/// Readability score, out of 100.
	pub readability: Score,
	/// Complexity score, out of 100.
	pub complexity: Score,
	/// Every finding in the file.
	pub findings: Vec<Finding>,
	/// Scored units within the file.
	pub units: Vec<UnitReport>,
	/// Constructs the scanner could not close, if any.
	///
	/// Present so a caller can tell the difference between "clean code" and "code whose
	/// tokenization was uncertain".
	pub unterminated: Vec<String>,
}

impl FileReport {
	/// The combined score, out of 100.
	///
	/// Readability and complexity are weighted equally: a file that is easy to follow but
	/// impossible to test is not better than the reverse, and both are first-class goals.
	#[must_use]
	pub fn overall(&self) -> f64 {
		f64::midpoint(self.readability.value, self.complexity.value)
	}

	/// Reduces this report to what aggregation needs.
	#[must_use]
	pub fn to_file_score(&self) -> crate::aggregate::FileScore {
		crate::aggregate::FileScore {
			path: self.path.clone(),
			language: self.language,
			code_lines: self.code_lines,
			readability: self.readability,
			complexity: self.complexity,
			total_penalty: self.readability.penalty + self.complexity.penalty,
		}
	}

	/// Findings ordered from most to least costly.
	#[must_use]
	pub fn ranked_findings(&self) -> Vec<&Finding> {
		let mut findings: Vec<&Finding> = self.findings.iter().collect();

		findings.sort_by(|left, right| {
			right
				.penalty()
				.partial_cmp(&left.penalty())
				.unwrap_or(std::cmp::Ordering::Equal)
		});

		findings
	}
}

/// A scored project.
#[derive(Debug, Clone, Serialize)]
pub struct ProjectReport {
	/// Every analyzed file.
	pub files: Vec<FileReport>,
	/// Aggregate readability score, weighted by lines of code.
	pub readability: Score,
	/// Aggregate complexity score, weighted by lines of code.
	pub complexity: Score,
	/// Total lines across all files.
	pub total_lines: usize,
	/// Total lines of code across all files.
	pub code_lines: usize,
	/// Files skipped because their language was unsupported.
	pub skipped: Vec<PathBuf>,
	/// Detected workspace packages, when the analyzed path is a monorepo.
	pub packages: Vec<crate::package::PackageReport>,
}

impl ProjectReport {
	/// Rules ranked by how much readability penalty each contributes.
	///
	/// The first entry is the single change that would recover the most points.
	#[must_use]
	pub fn readability_impact(&self) -> Vec<crate::aggregate::RuleImpact> {
		crate::aggregate::rank_rule_impact(&self.owned_findings(), Category::Readability)
	}

	/// Rules ranked by how much complexity penalty each contributes.
	#[must_use]
	pub fn complexity_impact(&self) -> Vec<crate::aggregate::RuleImpact> {
		crate::aggregate::rank_rule_impact(&self.owned_findings(), Category::Complexity)
	}

	/// Files ranked by the penalty they contribute, most costly first.
	#[must_use]
	pub fn file_impact(&self) -> Vec<(PathBuf, f64, f64, usize)> {
		let mut ranked: Vec<(PathBuf, f64, f64, usize)> = self
			.files
			.iter()
			.map(|file| {
				(
					file.path.clone(),
					file.readability.penalty + file.complexity.penalty,
					file.overall(),
					file.code_lines,
				)
			})
			.collect();

		ranked.sort_by(|left, right| {
			right
				.1
				.partial_cmp(&left.1)
				.unwrap_or(std::cmp::Ordering::Equal)
		});
		ranked
	}

	/// Findings paired with their paths, as owned values.
	#[must_use]
	pub fn owned_findings_public(&self) -> Vec<(PathBuf, Finding)> {
		self.owned_findings()
	}

	/// Findings paired with their paths, as owned values.
	fn owned_findings(&self) -> Vec<(PathBuf, Finding)> {
		self.files
			.iter()
			.flat_map(|file| {
				file.findings
					.iter()
					.map(|finding| (file.path.clone(), finding.clone()))
			})
			.collect()
	}
}

impl ProjectReport {
	/// The combined score, out of 100.
	#[must_use]
	pub fn overall(&self) -> f64 {
		f64::midpoint(self.readability.value, self.complexity.value)
	}

	/// Every finding across every file, paired with the file it came from.
	#[must_use]
	pub fn all_findings(&self) -> Vec<(&PathBuf, &Finding)> {
		self.files
			.iter()
			.flat_map(|file| {
				file.findings
					.iter()
					.map(move |finding| (&file.path, finding))
			})
			.collect()
	}

	/// Findings that cost points, excluding credit entries.
	///
	/// Credit for a good comment is modelled as a finding with a negative weight, which keeps the
	/// scoring arithmetic uniform. Reporting has to reverse that: counting a credit line as a
	/// "rule violation" would tell the reader to fix something the tool just praised.
	#[must_use]
	pub fn penalizing_findings(&self) -> Vec<(&PathBuf, &Finding)> {
		self.all_findings()
			.into_iter()
			.filter(|(_path, finding)| finding.penalty() > 0.0)
			.collect()
	}

	/// Findings that earn credit.
	#[must_use]
	pub fn credited_findings(&self) -> Vec<(&PathBuf, &Finding)> {
		self.all_findings()
			.into_iter()
			.filter(|(_path, finding)| finding.penalty() < 0.0)
			.collect()
	}

	/// Counts penalizing findings by rule name, most frequent first.
	#[must_use]
	pub fn findings_by_rule(&self) -> Vec<(String, usize)> {
		let mut counts: Vec<(String, usize)> = Vec::new();

		for (_path, finding) in self.penalizing_findings() {
			match counts.iter_mut().find(|(rule, _)| *rule == finding.rule) {
				Some((_, count)) => *count += 1,
				None => counts.push((finding.rule.clone(), 1)),
			}
		}

		counts.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
		counts
	}

	/// The worst files by overall score, most problematic first.
	#[must_use]
	pub fn worst_files(&self, limit: usize) -> Vec<&FileReport> {
		let mut files: Vec<&FileReport> = self.files.iter().collect();

		files.sort_by(|left, right| {
			left.overall()
				.partial_cmp(&right.overall())
				.unwrap_or(std::cmp::Ordering::Equal)
		});

		files.into_iter().take(limit).collect()
	}
}

/// Analysis settings shared across a run.
#[derive(Debug, Clone, Default)]
pub struct AnalysisOptions {
	/// Rule thresholds.
	pub rules: RulesConfig,
	/// Scoring curve settings.
	pub scoring: ScoringConfig,
}

/// Analyzes one file's source text.
#[must_use]
pub fn analyze_source(path: &Path, source: &str, options: &AnalysisOptions) -> Option<FileReport> {
	let language = language_for_path(path)?;

	Some(analyze_with_language(path, source, language, options))
}

/// Analyzes source in a known language.
#[must_use]
pub fn analyze_with_language(
	path: &Path,
	source: &str,
	language: Language,
	options: &AnalysisOptions,
) -> FileReport {
	let lexed = lex(source, language);
	let findings = monostyle_rules::run_rules(&lexed, &options.rules);

	build_report(path, &lexed, findings, options)
}

/// Builds a report from an already-lexed file.
fn build_report(
	path: &Path,
	lexed: &LexedFile,
	findings: Vec<Finding>,
	options: &AnalysisOptions,
) -> FileReport {
	let code_lines = lexed.source_line_count();
	let cyclomatic = monostyle_metrics::cyclomatic_complexity(lexed);
	let cognitive = monostyle_metrics::cognitive_complexity(lexed);

	let readability = Score::from_findings(
		&findings,
		Category::Readability,
		code_lines,
		options.scoring,
	);
	let complexity =
		Score::from_findings(&findings, Category::Complexity, code_lines, options.scoring);

	let units = build_unit_reports(lexed, &findings, options);

	FileReport {
		path: path.to_path_buf(),
		language: lexed.language,
		total_lines: lexed.lines.len(),
		code_lines,
		comment_lines: lexed.lines.iter().filter(|line| line.is_comment()).count(),
		blank_lines: lexed.lines.iter().filter(|line| line.is_blank()).count(),
		cyclomatic: cyclomatic.total,
		cognitive: cognitive.total,
		readability,
		complexity,
		findings,
		units,
		unterminated: lexed
			.unterminated
			.iter()
			.map(|construct| {
				format!(
					"{} opened on line {} was never closed with `{}`",
					match construct.kind {
						monostyle_lexer::UnterminatedKind::BlockComment => "a block comment",
						monostyle_lexer::UnterminatedKind::Literal => "a string literal",
						monostyle_lexer::UnterminatedKind::Heredoc => "a heredoc",
						monostyle_lexer::UnterminatedKind::Regex => "a regex literal",
					},
					construct.line,
					construct.delimiter
				)
			})
			.collect(),
	}
}

/// Scores each function-like unit and attributes findings to them.
fn build_unit_reports(
	lexed: &LexedFile,
	findings: &[Finding],
	options: &AnalysisOptions,
) -> Vec<UnitReport> {
	monostyle_metrics::find_units(lexed)
		.into_iter()
		.map(|unit| build_unit_report(lexed, &unit, findings, options))
		.collect()
}

/// Scores one unit and collects the findings that fall inside it.
fn build_unit_report(
	lexed: &LexedFile,
	unit: &CodeUnit,
	findings: &[Finding],
	options: &AnalysisOptions,
) -> UnitReport {
	let lines = unit_lines(lexed, unit);
	let cyclomatic = monostyle_metrics::complexity_of_lines(lines);
	let cognitive = monostyle_metrics::cognitive_complexity_of_lines(lines);
	let code_lines = lines
		.iter()
		.filter(|line| !line.is_blank() && !line.is_comment() && !line.is_literal())
		.count();

	// Only findings that fall within the unit contribute to its score, so a unit's number is
	// attributable to the unit rather than to its neighbours.
	let unit_findings: Vec<Finding> = findings
		.iter()
		.filter(|finding| unit.contains(finding.span.start_line))
		.cloned()
		.collect();

	let readability = Score::from_findings(
		&unit_findings,
		Category::Readability,
		code_lines,
		options.scoring,
	);
	let complexity = Score::from_findings(
		&unit_findings,
		Category::Complexity,
		code_lines,
		options.scoring,
	);

	UnitReport {
		name: unit.name.clone(),
		start_line: unit.start_line,
		end_line: unit.end_line,
		lines: code_lines,
		cyclomatic: cyclomatic.total,
		cyclomatic_risk: cyclomatic.risk().label().to_string(),
		cognitive: cognitive.total,
		cognitive_grade: cognitive.grade().to_string(),
		nesting_penalty: cognitive.nesting_penalty,
		max_nesting: cognitive.max_nesting,
		readability,
		complexity,
		findings: unit_findings,
	}
}

/// Returns the lines belonging to a unit.
fn unit_lines<'file>(
	file: &'file LexedFile,
	unit: &CodeUnit,
) -> &'file [monostyle_lexer::LexedLine] {
	let start = unit.start_line.saturating_sub(1).min(file.lines.len());
	let end = unit.end_line.min(file.lines.len());

	file.lines.get(start..end).unwrap_or_default()
}

/// Aggregates file reports into a project report.
#[must_use]
pub fn aggregate(
	files: Vec<FileReport>,
	skipped: Vec<PathBuf>,
	options: &AnalysisOptions,
) -> ProjectReport {
	let all_findings: Vec<Finding> = files
		.iter()
		.flat_map(|file| file.findings.clone())
		.collect();
	let code_lines: usize = files.iter().map(|file| file.code_lines).sum();
	let total_lines: usize = files.iter().map(|file| file.total_lines).sum();

	let readability = Score::from_findings(
		&all_findings,
		Category::Readability,
		code_lines,
		options.scoring,
	);
	let complexity = Score::from_findings(
		&all_findings,
		Category::Complexity,
		code_lines,
		options.scoring,
	);

	let packages = score_packages(&files, options);

	ProjectReport {
		files,
		readability,
		complexity,
		total_lines,
		code_lines,
		skipped,
		packages,
	}
}

/// Detects workspace packages and scores each one.
///
/// Detection runs against the common ancestor of the analyzed files, so scoring a directory inside
/// a repository still attributes files to the package that owns them rather than to the directory.
fn score_packages(
	files: &[FileReport],
	options: &AnalysisOptions,
) -> Vec<crate::package::PackageReport> {
	let Some(root) = common_root(files) else {
		return Vec::new();
	};

	let packages = crate::package::detect_packages(&root);

	if packages.is_empty() {
		return Vec::new();
	}

	let paths: Vec<PathBuf> = files.iter().map(|file| file.path.clone()).collect();
	let scores: Vec<crate::aggregate::FileScore> =
		files.iter().map(FileReport::to_file_score).collect();

	let by_path: std::collections::HashMap<&PathBuf, &crate::aggregate::FileScore> =
		paths.iter().zip(scores.iter()).collect();

	crate::package::attribute_files(&packages, &paths)
		.into_iter()
		.map(|(package, owned)| {
			let owned_scores: Vec<crate::aggregate::FileScore> = owned
				.iter()
				.filter_map(|path| by_path.get(path).map(|score| (*score).clone()))
				.collect();

			crate::package::score_package(&package, &owned_scores, options.scoring)
		})
		.collect()
}

/// Returns the deepest directory containing every analyzed file.
fn common_root(files: &[FileReport]) -> Option<PathBuf> {
	let mut root: Option<PathBuf> = None;

	for file in files {
		let directory = file.path.parent()?.to_path_buf();

		root = Some(match root {
			None => directory,
			Some(current) => {
				let mut shared = PathBuf::new();

				for (left, right) in current.components().zip(directory.components()) {
					if left != right {
						break;
					}

					shared.push(left);
				}

				shared
			}
		});
	}

	root
}

/// Resolves a language from a path's extension.
#[must_use]
pub fn language_for_path(path: &Path) -> Option<Language> {
	path.extension()
		.and_then(|extension| extension.to_str())
		.and_then(Language::from_extension)
}

/// Collects analyzable paths under `root`, honouring ignore files.
#[must_use]
pub fn collect_paths(root: &Path, respect_ignore: bool) -> Vec<PathBuf> {
	use ignore::WalkBuilder;

	let mut builder = WalkBuilder::new(root);

	builder.hidden(false);
	builder.git_ignore(respect_ignore);
	builder.git_global(respect_ignore);
	builder.git_exclude(respect_ignore);
	builder.ignore(respect_ignore);
	builder.parents(respect_ignore);

	builder
		.build()
		.filter_map(Result::ok)
		.filter(|entry| entry.file_type().is_some_and(|kind| kind.is_file()))
		.filter(|entry| language_for_path(entry.path()).is_some())
		.map(ignore::DirEntry::into_path)
		.collect()
}

/// Analyzes a set of paths in parallel.
#[must_use]
pub fn analyze_paths(paths: &[PathBuf], options: &AnalysisOptions) -> ProjectReport {
	use rayon::prelude::*;

	let results: Vec<(Option<FileReport>, Option<PathBuf>)> = paths
		.par_iter()
		.map(|path| {
			match std::fs::read_to_string(path) {
				Ok(source) => {
					let Some(language) = language_for_path(path) else {
						return (None, Some(path.clone()));
					};

					(
						Some(analyze_with_language(path, &source, language, options)),
						None,
					)
				}
				// A file that cannot be read as UTF-8 is reported as skipped rather than failing
				// the run, because one binary blob in a tree should not stop the analysis.
				Err(_) => (None, Some(path.clone())),
			}
		})
		.collect();

	let mut files = Vec::new();
	let mut skipped = Vec::new();

	for (report, skip) in results {
		if let Some(report) = report {
			files.push(report);
		}

		if let Some(path) = skip {
			skipped.push(path);
		}
	}

	files.sort_by(|left, right| left.path.cmp(&right.path));

	aggregate(files, skipped, options)
}
