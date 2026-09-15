//! Report rendering.
//!
//! Two audiences are served: humans reading a terminal, and tools consuming JSON. The text
//! renderer leads with the scores and follows with the findings that caused them, because the
//! question a reader actually has is "how bad is it, and what do I fix first?".

use std::fmt::Write as _;
use std::path::Path;
use std::path::PathBuf;

use monostyle_core::Finding;

use crate::analysis::FileReport;
use crate::analysis::ProjectReport;
use crate::analysis::UnitReport;

/// Renders a project report as text.
#[must_use]
pub fn render_project(report: &ProjectReport, explain: bool, show_units: bool) -> String {
	let mut output = String::new();

	render_scores(&mut output, report);
	render_rule_summary(&mut output, report);

	if show_units {
		render_worst_units(&mut output, report, 15);
	}

	render_files(&mut output, report);

	if explain {
		render_findings(&mut output, report);
	}

	render_unterminated(&mut output, report);
	output
}

/// Writes the headline scores.
fn render_scores(output: &mut String, report: &ProjectReport) {
	let _ = writeln!(output, "monostyle");
	let _ = writeln!(output, "{}", "=".repeat(60));
	let _ = writeln!(output, "  files analyzed  {}", report.files.len());
	let _ = writeln!(output, "  lines of code   {}", report.code_lines);
	let _ = writeln!(output);
	let _ = writeln!(
		output,
		"  readability     {:>6.1}  {}   {}",
		report.readability.value,
		bar(report.readability.value),
		report.readability.grade()
	);
	let _ = writeln!(
		output,
		"  complexity      {:>6.1}  {}   {}",
		report.complexity.value,
		bar(report.complexity.value),
		report.complexity.grade()
	);
	let _ = writeln!(
		output,
		"  overall         {:>6.1}  {}   {}",
		report.overall(),
		bar(report.overall()),
		grade(report.overall())
	);
	let _ = writeln!(output);
}

/// Writes the most frequent rule violations, and credits separately.
///
/// Credits are listed apart from violations because a credit is praise: folding it into the
/// violation count would tell the reader to fix something the tool just rewarded.
fn render_rule_summary(output: &mut String, report: &ProjectReport) {
	let counts = report.findings_by_rule();
	let credits = report.credited_findings().len();

	if counts.is_empty() && credits == 0 {
		let _ = writeln!(output, "No rule violations found.");
		let _ = writeln!(output);

		return;
	}

	if !counts.is_empty() {
		let _ = writeln!(output, "Findings by rule");
		let _ = writeln!(output, "{}", "-".repeat(60));

		for (rule, count) in counts.iter().take(12) {
			let _ = writeln!(output, "  {count:>5}  {rule}");
		}

		if counts.len() > 12 {
			let _ = writeln!(output, "  {:>5}  ({} more rules)", "", counts.len() - 12);
		}
	}

	if credits > 0 {
		let _ = writeln!(output);
		let _ = writeln!(
			output,
			"  {credits} comments explain why (these earn credit rather than costing points)"
		);
	}

	let _ = writeln!(output);
}

/// Writes the lowest-scoring functions across the project.
fn render_worst_units(output: &mut String, report: &ProjectReport, limit: usize) {
	let mut units: Vec<(&Path, &UnitReport)> = report
		.files
		.iter()
		.flat_map(|file| {
			file.units
				.iter()
				.map(move |unit| (file.path.as_path(), unit))
		})
		.collect();

	if units.is_empty() {
		return;
	}

	units.sort_by(|left, right| {
		left.1
			.score()
			.partial_cmp(&right.1.score())
			.unwrap_or(std::cmp::Ordering::Equal)
	});

	let _ = writeln!(output, "Worst functions");
	let _ = writeln!(output, "{}", "-".repeat(60));
	let _ = writeln!(
		output,
		"  {:>6}  {:>4}  {:>4}  {:>4}  function",
		"score", "cyc", "cog", "nest"
	);

	for (path, unit) in units.iter().take(limit) {
		let _ = writeln!(
			output,
			"  {:>6.1}  {:>4}  {:>4}  {:>4}  {} {}:{}",
			unit.score(),
			unit.cyclomatic,
			unit.cognitive,
			unit.max_nesting,
			format_unit_name(&unit.name),
			display_path(path),
			unit.start_line
		);
	}

	let _ = writeln!(output);
}

/// Writes a per-file table, worst first.
fn render_files(output: &mut String, report: &ProjectReport) {
	if report.files.is_empty() {
		return;
	}

	let _ = writeln!(output, "Files");
	let _ = writeln!(output, "{}", "-".repeat(60));
	let _ = writeln!(
		output,
		"  {:>6}  {:>6}  {:>5}  {:>4}  file",
		"read", "cplx", "code", "func"
	);

	let mut files: Vec<&FileReport> = report.files.iter().collect();

	files.sort_by(|left, right| {
		left.overall()
			.partial_cmp(&right.overall())
			.unwrap_or(std::cmp::Ordering::Equal)
	});

	for file in files {
		let _ = writeln!(
			output,
			"  {:>6.1}  {:>6.1}  {:>5}  {:>4}  {}",
			file.readability.value,
			file.complexity.value,
			file.code_lines,
			file.units.len(),
			display_path(&file.path)
		);
	}

	let _ = writeln!(output);
}

/// Writes every finding with its location and explanation.
fn render_findings(output: &mut String, report: &ProjectReport) {
	// Credit entries are summarized in the header rather than listed here, because there is
	// nothing for the reader to act on.
	let mut findings: Vec<(&PathBuf, &Finding)> = report
		.all_findings()
		.into_iter()
		.filter(|(_path, finding)| finding.penalty() > 0.0)
		.collect();

	if findings.is_empty() {
		return;
	}

	findings.sort_by(|left, right| {
		right
			.1
			.penalty()
			.partial_cmp(&left.1.penalty())
			.unwrap_or(std::cmp::Ordering::Equal)
	});

	let _ = writeln!(output, "Findings");
	let _ = writeln!(output, "{}", "-".repeat(60));

	for (path, finding) in findings {
		render_finding(output, path, finding);
	}
}

/// Writes one finding.
fn render_finding(output: &mut String, path: &Path, finding: &Finding) {
	let _ = writeln!(
		output,
		"  {}:{}  [{}] {}",
		display_path(path),
		finding.span.start_line,
		finding.severity.label(),
		finding.rule
	);
	let _ = writeln!(output, "      {}", finding.message);
	let _ = writeln!(output, "      → {}", finding.suggestion);
	let _ = writeln!(output);
}

/// Writes a warning for files where tokenization was uncertain.
///
/// This matters because an unterminated construct means the score itself is less trustworthy,
/// which a reader needs to know before trusting the number.
fn render_unterminated(output: &mut String, report: &ProjectReport) {
	let affected: Vec<&FileReport> = report
		.files
		.iter()
		.filter(|file| !file.unterminated.is_empty())
		.collect();

	if affected.is_empty() {
		return;
	}

	let _ = writeln!(output, "Tokenizer warnings");
	let _ = writeln!(output, "{}", "-".repeat(60));
	let _ = writeln!(
		output,
		"  Scores for these files are less reliable because a construct never closed:"
	);

	for file in affected {
		for warning in &file.unterminated {
			let _ = writeln!(output, "  {}: {warning}", display_path(&file.path));
		}
	}

	let _ = writeln!(output);
}

/// Renders a project report as JSON.
pub fn render_project_json(report: &ProjectReport) -> Result<String, serde_json::Error> {
	serde_json::to_string_pretty(report)
}

/// Renders a summary line suitable for a single-file report.
#[must_use]
pub fn render_file_line(file: &FileReport) -> String {
	format!(
		"{}: readability {:.1}, complexity {:.1} over {} lines of code",
		display_path(&file.path),
		file.readability.value,
		file.complexity.value,
		file.code_lines
	)
}

/// Draws a ten-cell score bar.
///
/// ASCII rather than block-drawing characters so the output renders identically in every
/// terminal, locale, and log viewer.
fn bar(value: f64) -> String {
	/// Number of cells in the bar.
	const CELLS: usize = 10;

	let filled = ((value / 100.0) * CELLS as f64)
		.round()
		.clamp(0.0, CELLS as f64) as usize;

	format!("[{}{}]", "#".repeat(filled), ".".repeat(CELLS - filled))
}

/// Maps a score to a qualitative grade.
fn grade(value: f64) -> &'static str {
	match value {
		value if value >= 90.0 => "excellent",
		value if value >= 75.0 => "good",
		value if value >= 60.0 => "fair",
		value if value >= 40.0 => "poor",
		_ => "bad",
	}
}

/// Renders a path relative to the current directory when possible.
fn display_path(path: &Path) -> String {
	let current = std::env::current_dir().ok();

	if let Some(current) = current
		&& let Ok(relative) = path.strip_prefix(&current)
	{
		return relative.display().to_string();
	}

	path.display().to_string()
}

/// Renders a unit's name for display, falling back when detection found nothing.
fn format_unit_name(name: &str) -> String {
	if name.is_empty() {
		"<anonymous>".to_string()
	} else {
		name.to_string()
	}
}

impl UnitReport {
	/// The unit's combined score, out of 100.
	#[must_use]
	pub fn score(&self) -> f64 {
		if self.findings.is_empty() {
			// A unit with no findings is clean rather than unmeasured, so it scores perfectly.
			return monostyle_core::Score::PERFECT;
		}

		f64::midpoint(self.readability.value, self.complexity.value)
	}
}
