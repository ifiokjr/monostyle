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

use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;

use monostyle_core::Category;
use monostyle_core::CategoryScores;
use monostyle_core::Finding;
use monostyle_core::Language;
use monostyle_core::Score;
use monostyle_core::ScoringConfig;
use monostyle_lexer::lex;
use monostyle_metrics::CodeUnit;
use monostyle_rules::RuleContext;
use monostyle_rules::RulesConfig;
use monostyle_rules::unit_measures::UnitMetrics;
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
		monostyle_core::score::grade(self.score())
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

		findings.sort_by(|left, right| right.penalty().total_cmp(&left.penalty()));

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
	/// Paths whose scores fell below the floor that applies to them.
	///
	/// Reported per path rather than as one verdict, so a failure names what to fix. A single "below the
	/// floor" message tells a reader nothing about where to look.
	pub floor_violations: Vec<FloorViolation>,
	/// Whether any floor was configured, so the report can say so when everything passes.
	#[serde(skip)]
	pub floor_configured: bool,
}

/// A path that scored below its floor.
#[derive(Debug, Clone, Serialize)]
pub struct FloorViolation {
	/// The path that fell short.
	pub path: PathBuf,
	/// The readability score achieved.
	pub readability: f64,
	/// The complexity score achieved.
	pub complexity: f64,
	/// The readability floor that applied, when one did.
	pub readability_floor: Option<f64>,
	/// The complexity floor that applied, when one did.
	pub complexity_floor: Option<f64>,
	/// Why this path has its own floor, when a section explained itself.
	pub reason: Option<String>,
	/// The rules that cost the most points at this path, worst first.
	///
	/// This is what turns a failure into a worklist: the reader sees the floor, how far short it fell, and
	/// which rules to address.
	pub top_offenders: Vec<(&'static str, f64)>,
}

impl FloorViolation {
	/// Whether either category fell short.
	#[must_use]
	pub fn is_below(&self) -> bool {
		self.readability_floor
			.is_some_and(|floor| self.readability < floor)
			|| self
				.complexity_floor
				.is_some_and(|floor| self.complexity < floor)
	}

	/// A one-line description of what fell short and by how much.
	#[must_use]
	pub fn summary(&self) -> String {
		let mut parts = Vec::new();

		if let Some(floor) = self.readability_floor
			&& self.readability < floor
		{
			parts.push(format!(
				"readability {:.1} is {:.1} below the {floor:.1} floor",
				self.readability,
				floor - self.readability
			));
		}

		if let Some(floor) = self.complexity_floor
			&& self.complexity < floor
		{
			parts.push(format!(
				"complexity {:.1} is {:.1} below the {floor:.1} floor",
				self.complexity,
				floor - self.complexity
			));
		}

		parts.join("; ")
	}
}

impl ProjectReport {
	/// Rules ranked by how much readability penalty each contributes.
	///
	/// The first entry is the single change that would recover the most points.
	#[must_use]
	pub fn readability_impact(&self) -> Vec<crate::aggregate::RuleImpact> {
		crate::aggregate::rank_rule_impact(&self.all_findings(), Category::Readability)
	}

	/// Rules ranked by how much complexity penalty each contributes.
	#[must_use]
	pub fn complexity_impact(&self) -> Vec<crate::aggregate::RuleImpact> {
		crate::aggregate::rank_rule_impact(&self.all_findings(), Category::Complexity)
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

		ranked.sort_by(|left, right| right.1.total_cmp(&left.1));
		ranked
	}

	/// Whether any floor was configured for this run.
	///
	/// Used to decide whether "every path meets its floor" is worth printing: saying so when no floor was
	/// set would be misleading.
	#[must_use]
	pub fn has_any_floor(&self) -> bool {
		!self.floor_violations.is_empty() || self.floor_configured
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
	pub fn all_findings(&self) -> Vec<(&Path, &Finding)> {
		self.files
			.iter()
			.flat_map(|file| {
				file.findings
					.iter()
					.map(move |finding| (file.path.as_path(), finding))
			})
			.collect()
	}

	/// Findings that cost points, excluding credit entries.
	///
	/// Credit for a good comment is modelled as a finding with a negative weight, which keeps the
	/// scoring arithmetic uniform. Reporting has to reverse that: counting a credit line as a
	/// "rule violation" would tell the reader to fix something the tool just praised.
	#[must_use]
	pub fn penalizing_findings(&self) -> Vec<(&Path, &Finding)> {
		self.all_findings()
			.into_iter()
			.filter(|(_path, finding)| finding.penalty() > 0.0)
			.collect()
	}

	/// Findings that earn credit.
	#[must_use]
	pub fn credited_findings(&self) -> Vec<(&Path, &Finding)> {
		self.all_findings()
			.into_iter()
			.filter(|(_path, finding)| finding.penalty() < 0.0)
			.collect()
	}

	/// Counts penalizing findings by rule name, most frequent first.
	#[must_use]
	pub fn findings_by_rule(&self) -> Vec<(&'static str, usize)> {
		let mut counts: HashMap<&'static str, usize> = HashMap::new();

		for (_path, finding) in self.penalizing_findings() {
			*counts.entry(finding.rule).or_insert(0) += 1;
		}

		let mut counts: Vec<(&'static str, usize)> = counts.into_iter().collect();
		counts.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(right.0)));
		counts
	}

	/// The worst files by overall score, most problematic first.
	#[must_use]
	pub fn worst_files(&self, limit: usize) -> Vec<&FileReport> {
		let mut files: Vec<&FileReport> = self.files.iter().collect();

		files.sort_by(|left, right| left.overall().total_cmp(&right.overall()));

		files.into_iter().take(limit).collect()
	}
}

/// Analysis settings shared across a run.
///
/// Every field's default is its type's default, which is why `Default` is derived: the one field
/// that ever needed a hand-written default (the cache switch) is gone.
#[derive(Debug, Clone, Default)]
pub struct AnalysisOptions {
	/// Rule thresholds, including which paths to skip.
	pub rules: RulesConfig,
	/// Scoring curve settings.
	pub scoring: ScoringConfig,
	/// Score floors that apply across the repository.
	///
	/// Checked after scoring, so a run reports which paths fell short and by how much rather than only
	/// that something did.
	pub fail_under: crate::section::ScoreFloor,
	/// Per-path sections, each with its own thresholds and floors.
	///
	/// The most specific section that matches a path wins, so a project can hold its core to one standard
	/// and its tests to another without disabling a rule for everything.
	pub sections: Vec<crate::section::Section>,
}

impl AnalysisOptions {
	/// Returns the rules that apply to `path`.
	///
	/// A section's rules are layered onto the repository-wide values when the configuration is read, so a
	/// section only states what differs and this is a selection rather than a merge. The reference is
	/// borrowed because every file pays this call, and cloning the pattern lists it would copy was pure
	/// overhead.
	#[must_use]
	pub fn rules_for(&self, path: &Path) -> &RulesConfig {
		match crate::section::section_for(&self.sections, path) {
			Some(section) => &section.rules,
			None => &self.rules,
		}
	}

	/// Returns the floor that applies to `path`.
	///
	/// A more specific section replaces the repository-wide floor for the categories it sets and inherits
	/// the rest, so a section can raise the bar for readability without restating complexity.
	#[must_use]
	pub fn floor_for(&self, path: &Path) -> crate::section::ScoreFloor {
		self.floor_from(crate::section::section_for(&self.sections, path))
	}

	/// Merges the repository-wide floor with the section that applies, when one does.
	fn floor_from(&self, section: Option<&crate::section::Section>) -> crate::section::ScoreFloor {
		let Some(section) = section else {
			return self.fail_under;
		};

		crate::section::ScoreFloor {
			readability: section
				.fail_under
				.readability
				.or(self.fail_under.readability),
			complexity: section.fail_under.complexity.or(self.fail_under.complexity),
		}
	}

	/// Whether `path` is excluded by a section.
	#[must_use]
	pub fn is_ignored(&self, path: &Path) -> bool {
		crate::section::section_for(&self.sections, path).is_some_and(|section| section.ignore)
	}
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

	// The rules come from the section that matches the file's path, so a directory can hold a different
	// bar from the rest of the repository without the rule being disabled everywhere. The context shares
	// the unit measurements between the rules and this file's unit reports.
	let context = RuleContext::new(&lexed, options.rules_for(path));
	let findings = monostyle_rules::run_rules(&context);

	build_report(path, &context, findings, options)
}

/// Builds a report from a file that has been lexed and run through its rules.
fn build_report(
	path: &Path,
	context: &RuleContext<'_>,
	findings: Vec<Finding>,
	options: &AnalysisOptions,
) -> FileReport {
	let lexed = context.file();
	let code_lines = lexed.source_line_count();
	let cyclomatic = monostyle_metrics::cyclomatic_complexity(lexed);
	let cognitive = monostyle_metrics::cognitive_complexity(lexed);
	let CategoryScores {
		readability,
		complexity,
	} = Score::by_category(&findings, code_lines, options.scoring);

	let units = build_unit_reports(context, &findings, options);

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

/// Scores each function-like unit and attributes findings to it.
///
/// Findings are indexed by line once, so attributing them is a binary search per unit rather
/// than a scan of every finding for every unit.
fn build_unit_reports(
	context: &RuleContext<'_>,
	findings: &[Finding],
	options: &AnalysisOptions,
) -> Vec<UnitReport> {
	let by_line = FindingsByLine::new(findings);

	context
		.units()
		.iter()
		.map(|(unit, metrics)| {
			build_unit_report(unit, metrics, findings, &by_line.within(unit), options)
		})
		.collect()
}

/// Scores one unit and collects the findings that fall inside it.
fn build_unit_report(
	unit: &CodeUnit,
	metrics: &UnitMetrics,
	findings: &[Finding],
	positions: &[usize],
	options: &AnalysisOptions,
) -> UnitReport {
	// Only findings that fall within the unit contribute to its score, so a unit's number is
	// attributable to the unit rather than to its neighbours.
	let unit_findings: Vec<Finding> = positions
		.iter()
		.filter_map(|&position| findings.get(position).cloned())
		.collect();

	let CategoryScores {
		readability,
		complexity,
	} = Score::by_category(&unit_findings, metrics.code_lines, options.scoring);

	UnitReport {
		name: unit.name.clone(),
		start_line: unit.start_line,
		end_line: unit.end_line,
		lines: metrics.code_lines,
		cyclomatic: metrics.cyclomatic,
		cyclomatic_risk: metrics.cyclomatic_risk.to_string(),
		cognitive: metrics.cognitive,
		cognitive_grade: metrics.cognitive_grade.to_string(),
		nesting_penalty: metrics.nesting_penalty,
		max_nesting: metrics.max_nesting,
		readability,
		complexity,
		findings: unit_findings,
	}
}

/// A file's findings, ordered by the line each starts on.
struct FindingsByLine {
	/// The line a finding starts on and where it sits in the file's list, ordered by line.
	entries: Vec<(usize, usize)>,
}

impl FindingsByLine {
	/// Indexes `findings` by start line.
	fn new(findings: &[Finding]) -> Self {
		let mut entries: Vec<(usize, usize)> = findings
			.iter()
			.enumerate()
			.map(|(position, finding)| (finding.span.start_line, position))
			.collect();

		entries.sort_unstable();

		Self { entries }
	}

	/// Where the findings that start inside `unit` sit in the file's list, in list order.
	fn within(&self, unit: &CodeUnit) -> Vec<usize> {
		let first = self
			.entries
			.partition_point(|&(line, _)| line < unit.start_line);

		let mut positions: Vec<usize> = self
			.entries
			.get(first..)
			.unwrap_or_default()
			.iter()
			.take_while(|&&(line, _)| line <= unit.end_line)
			.map(|&(_, position)| position)
			.collect();

		positions.sort_unstable();

		positions
	}
}

/// Aggregates file reports into a project report.
#[must_use]
pub fn aggregate(
	files: Vec<FileReport>,
	skipped: Vec<PathBuf>,
	options: &AnalysisOptions,
) -> ProjectReport {
	let code_lines: usize = files.iter().map(|file| file.code_lines).sum();
	let total_lines: usize = files.iter().map(|file| file.total_lines).sum();

	let CategoryScores {
		readability,
		complexity,
	} = Score::by_category(
		files.iter().flat_map(|file| &file.findings),
		code_lines,
		options.scoring,
	);

	let packages = score_packages(&files, options);
	let floor_violations = find_floor_violations(&files, options);
	let floor_configured = options.fail_under.is_set()
		|| options
			.sections
			.iter()
			.any(|section| section.fail_under.is_set());

	ProjectReport {
		files,
		readability,
		complexity,
		total_lines,
		code_lines,
		skipped,
		packages,
		floor_violations,
		floor_configured,
	}
}

/// Finds the files that scored below the floor that applies to them.
///
/// A file is only checked against the floor its own section sets, so a path with a lower bar is not held to
/// the repository default. Each violation carries the rules that cost it the most points, which is what
/// makes the result actionable rather than merely negative.
fn find_floor_violations(files: &[FileReport], options: &AnalysisOptions) -> Vec<FloorViolation> {
	let mut violations = Vec::new();

	for file in files {
		let section = crate::section::section_for(&options.sections, &file.path);
		let floor = options.floor_from(section);

		if !floor.is_set() {
			continue;
		}

		let violation = FloorViolation {
			path: file.path.clone(),
			readability: file.readability.value,
			complexity: file.complexity.value,
			readability_floor: floor.readability,
			complexity_floor: floor.complexity,
			reason: section.and_then(|section| section.reason.clone()),
			top_offenders: top_offenders_for(file),
		};

		if violation.is_below() {
			violations.push(violation);
		}
	}

	// Worst shortfall first, measured against the floor that applied, so the report leads with the path
	// furthest from where it should be.
	violations.sort_by(|left, right| {
		let shortfall = |violation: &FloorViolation| {
			let read = violation
				.readability_floor
				.map_or(0.0, |floor| (floor - violation.readability).max(0.0));
			let cplx = violation
				.complexity_floor
				.map_or(0.0, |floor| (floor - violation.complexity).max(0.0));

			read + cplx
		};

		shortfall(right).total_cmp(&shortfall(left))
	});

	violations
}

/// Returns the rules that cost a file the most points, worst first.
fn top_offenders_for(file: &FileReport) -> Vec<(&'static str, f64)> {
	let mut totals: HashMap<&'static str, f64> = HashMap::new();

	for finding in &file.findings {
		let penalty = finding.penalty();

		if penalty <= 0.0 {
			continue;
		}

		*totals.entry(finding.rule).or_insert(0.0) += penalty;
	}

	let mut totals: Vec<(&'static str, f64)> = totals.into_iter().collect();
	totals.sort_by(|left, right| right.1.total_cmp(&left.1).then_with(|| left.0.cmp(right.0)));
	totals.truncate(3);
	totals
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

	let by_path: HashMap<&PathBuf, &crate::aggregate::FileScore> =
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
			Some(current) => shared_prefix(&current, &directory),
		});
	}

	// The deepest shared directory is usually `.../src`, which contains no manifest. A workspace is
	// declared at the repository or package root, so the search walks upward until it finds one.
	// Returning the deepest shared directory meant workspace detection silently found nothing
	// whenever the analyzed files lived below the manifest, which is the normal layout.
	root.and_then(|deepest| nearest_manifest_root(&deepest))
}

/// Returns the leading path components that `left` and `right` have in common.
fn shared_prefix(left: &Path, right: &Path) -> PathBuf {
	let mut shared = PathBuf::new();

	for (left, right) in left.components().zip(right.components()) {
		if left != right {
			break;
		}

		shared.push(left);
	}

	shared
}

/// Walks upward from `start` looking for a directory that declares a workspace.
fn nearest_manifest_root(start: &Path) -> Option<PathBuf> {
	/// Files that declare a workspace or a package.
	const MANIFESTS: &[&str] = &[
		"Cargo.toml",
		"package.json",
		"pnpm-workspace.yaml",
		"pubspec.yaml",
	];

	let mut directory = Some(start.to_path_buf());

	while let Some(current) = directory {
		if MANIFESTS
			.iter()
			.any(|manifest| current.join(manifest).is_file())
		{
			return Some(current);
		}

		directory = current.parent().map(Path::to_path_buf);
	}

	None
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
pub fn collect_paths(
	root: &Path,
	respect_ignore: bool,
	ignore_config: &monostyle_core::IgnoreConfig,
) -> Vec<PathBuf> {
	use ignore::WalkBuilder;

	let mut builder = WalkBuilder::new(root);
	builder.hidden(false);
	builder.git_ignore(respect_ignore);
	builder.git_global(respect_ignore);
	builder.git_exclude(respect_ignore);
	builder.ignore(respect_ignore);
	builder.parents(respect_ignore);

	// Ignored directories are pruned with a filter rather than an override pattern. The walk still
	// descends into them, but the filter rejects each entry before it is read, which is enough to
	// keep a repository with a large dependency cache fast without a second glob engine.
	let walker = builder.build();
	walker
		.filter_map(Result::ok)
		.filter(|entry| entry.file_type().is_some_and(|kind| kind.is_file()))
		.filter(|entry| !has_ignored_component(entry.path()))
		.filter(|entry| language_for_path(entry.path()).is_some())
		.map(ignore::DirEntry::into_path)
		.filter(|path| ignore_config.allows_under(path, root))
		.collect()
}

/// Whether any path component is a directory that is never analyzed.
///
/// Checked before the file-type filters so a dependency cache is rejected without reading its
/// directory entry.
fn has_ignored_component(path: &Path) -> bool {
	path.components().any(|component| {
		let name = component.as_os_str().to_string_lossy();
		monostyle_core::ignore::is_ignored_directory(&name)
	})
}

/// Analyzes a set of paths in parallel.
///
/// The closure body lives in `analyze_one` so this function holds only the two things that are
/// easier to follow when they are apart: the parallel dispatch, and the partitioning of results
/// into analyzed files and skipped paths.
#[must_use]
pub fn analyze_paths(paths: &[PathBuf], options: &AnalysisOptions) -> ProjectReport {
	use rayon::prelude::*;

	let results: Vec<(Option<FileReport>, Option<PathBuf>)> = paths
		.par_iter()
		.map(|path| analyze_one(path, options))
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

/// Analyzes one path, or reports why it was skipped.
fn analyze_one(path: &Path, options: &AnalysisOptions) -> (Option<FileReport>, Option<PathBuf>) {
	// A section marked `ignore` excludes its path entirely, which is what a vendored directory or
	// a set of deliberately bad fixtures warrants. Distinct from a floor of zero: an ignored path
	// produces no findings and contributes nothing to any score.
	if options.is_ignored(path) {
		return (None, None);
	}

	let Some(language) = language_for_path(path) else {
		return (None, Some(path.to_path_buf()));
	};

	let Ok(source) = std::fs::read_to_string(path) else {
		// A file that cannot be read as UTF-8 is reported as skipped rather than failing the
		// run, because one binary blob in a tree should not stop the analysis.
		return (None, Some(path.to_path_buf()));
	};

	(
		Some(analyze_with_language(path, &source, language, options)),
		None,
	)
}
