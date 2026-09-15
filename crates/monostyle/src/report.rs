//! Report rendering.
//!
//! Two audiences, two renderers: a human reading a terminal, and a tool consuming JSON.
//!
//! # How the text output is organized
//!
//! A report answers three questions in the order a reader asks them:
//!
//! 1. **How bad is it?** The header states both scores, weighted by lines of code, with a bar and a
//!    word for each.
//! 2. **What should I fix first?** The impact table ranks rules by the share of total penalty each
//!    accounts for, so the top row is the single change worth the most points.
//! 3. **Where exactly is it?** Each impact names its worst offender as `path:line`, which is what
//!    makes the report actionable rather than merely informative.
//!
//! The package and per-file tables answer the follow-up question — *which part of the repository is
//! this coming from* — and the findings list is opt-in, because a reader who wants every finding
//! wants it in full and a reader who does not should not have to scroll past it.

use std::fmt::Write as _;
use std::path::Path;
use std::path::PathBuf;

use monostyle_core::Finding;

use crate::aggregate::RuleImpact;
use crate::analysis::FileReport;
use crate::analysis::ProjectReport;
use crate::package::PackageReport;
use crate::style;

/// Renders a project report as text.
#[must_use]
pub fn render_project(report: &ProjectReport, explain: bool, show_units: bool) -> String {
	let mut output = String::new();

	render_header(&mut output, report);
	render_impact(&mut output, report);

	if !report.packages.is_empty() {
		render_packages(&mut output, &report.packages);
	}

	if show_units {
		render_worst_units(&mut output, report, 10);
	}

	render_files(&mut output, report);

	if explain {
		render_findings(&mut output, report);
	}

	render_unterminated(&mut output, report);
	render_next_step(&mut output, report);
	output
}

/// Writes the headline scores.
fn render_header(output: &mut String, report: &ProjectReport) {
	let width = style::width();

	let _ = writeln!(output);
	let _ = writeln!(output, "{}", style::bold("monostyle"));
	let _ = writeln!(output, "{}", style::rule(width));

	if report.packages.is_empty() {
		let _ = writeln!(
			output,
			"  {} files   {} lines of code",
			style::bold(&report.files.len().to_string()),
			style::bold(&format_number(report.code_lines)),
		);
	} else {
		let _ = writeln!(
			output,
			"  {} files   {} packages   {} lines of code",
			style::bold(&report.files.len().to_string()),
			style::bold(&report.packages.len().to_string()),
			style::bold(&format_number(report.code_lines)),
		);
	}

	let _ = writeln!(output);
	render_score_row(
		output,
		"readability",
		report.readability.value,
		report.readability.grade(),
	);
	render_score_row(
		output,
		"complexity",
		report.complexity.value,
		report.complexity.grade(),
	);
	let _ = writeln!(output);
	render_score_row(output, "overall", report.overall(), grade(report.overall()));
	let _ = writeln!(output);
}

/// Writes one score line with its bar.
fn render_score_row(output: &mut String, label: &str, value: f64, grade: &str) {
	let _ = writeln!(
		output,
		"  {:<12} {:>6}  {}  {}",
		style::bold(label),
		style::score(value),
		style::bar(value, 20),
		style::dim(grade)
	);
}

/// Writes the impact table: which rules cost the most, and where.
///
/// This is the actionable core of the report. Rules are ranked by the share of total penalty they
/// account for, so the first row is the highest-value fix.
fn render_impact(output: &mut String, report: &ProjectReport) {
	for (impacts, label) in [
		(report.readability_impact(), "readability"),
		(report.complexity_impact(), "complexity"),
	] {
		if impacts.is_empty() {
			continue;
		}

		let _ = writeln!(
			output,
			"{}",
			style::bold(&format!("{label}: what is costing you points"))
		);
		let _ = writeln!(output);

		for impact in impacts.iter().take(8) {
			render_impact_row(output, impact);
		}

		if impacts.len() > 8 {
			let _ = writeln!(
				output,
				"  {}",
				style::dim(&format!("… and {} more rules", impacts.len() - 8))
			);
			let _ = writeln!(output);
		}

		let _ = writeln!(output);
	}
}

/// Writes one row of the impact table.
fn render_impact_row(output: &mut String, impact: &RuleImpact) {
	let _ = writeln!(
		output,
		"  {} {:>5.1}%  {}  {}",
		style::share_bar(impact.share, 10),
		impact.share * 100.0,
		style::magenta(&impact.rule),
		style::dim(&format!(
			"({} finding{})",
			impact.count,
			plural(impact.count)
		)),
	);

	// The worst offender's location is the single most useful line in the report: it is where a
	// reader or an agent should look first.
	if let Some(offender) = &impact.worst_offender {
		let _ = writeln!(
			output,
			"             {} {}",
			style::cyan(&format!(
				"{}:{}",
				display_path(&offender.path),
				offender.line
			)),
			style::dim(&format!("[{}]", offender.severity))
		);
	}

	if !impact.message.is_empty() {
		let _ = writeln!(
			output,
			"             {}",
			wrap(&impact.message, 68, "             ")
		);
	}

	if !impact.suggestion.is_empty() {
		let _ = writeln!(
			output,
			"             {} {}",
			style::dim("->"),
			style::dim(&wrap(&impact.suggestion, 68, "             "))
		);
	}

	let _ = writeln!(output);
}

/// Writes per-package scores when the repository declares packages.
fn render_packages(output: &mut String, packages: &[PackageReport]) {
	let _ = writeln!(output, "{}", style::bold("packages"));
	let _ = writeln!(output);
	let _ = writeln!(
		output,
		"  {:>7} {:>7}  {}",
		style::dim("read"),
		style::dim("cplx"),
		style::dim("package")
	);

	// Ranked by penalty so the package causing the most damage is first.
	let mut ranked = packages.to_vec();
	ranked.sort_by(|left, right| {
		right
			.total_penalty
			.partial_cmp(&left.total_penalty)
			.unwrap_or(std::cmp::Ordering::Equal)
	});

	for package in &ranked {
		let _ = writeln!(
			output,
			"  {:>7} {:>7}  {} {}",
			style::score(package.readability.value),
			style::score(package.complexity.value),
			style::bold(&package.package.name),
			style::dim(&format!(
				"{} · {} · {} LOC",
				package.package.ecosystem.label(),
				display_path(&package.package.directory),
				format_number(package.code_lines)
			))
		);
	}

	let _ = writeln!(output);
}

/// Writes the lowest-scoring functions across the project.
fn render_worst_units(output: &mut String, report: &ProjectReport, limit: usize) {
	let mut units: Vec<(&Path, &crate::analysis::UnitReport)> = report
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

	let _ = writeln!(output, "{}", style::bold("worst functions"));
	let _ = writeln!(output);
	let _ = writeln!(
		output,
		"  {:>6} {:>4} {:>4} {:>4}  {}",
		style::dim("score"),
		style::dim("cyc"),
		style::dim("cog"),
		style::dim("nest"),
		style::dim("function")
	);

	for (path, unit) in units.iter().take(limit) {
		let _ = writeln!(
			output,
			"  {:>6} {:>4} {:>4} {:>4}  {} {}",
			style::score(unit.score()),
			unit.cyclomatic,
			unit.cognitive,
			unit.max_nesting,
			style::cyan(&format!("{}:{}", display_path(path), unit.start_line)),
			style::dim(&unit.name)
		);
	}

	let _ = writeln!(output);
}

/// Writes a per-file table, most costly first.
fn render_files(output: &mut String, report: &ProjectReport) {
	if report.files.is_empty() {
		return;
	}

	let _ = writeln!(output, "{}", style::bold("files"));
	let _ = writeln!(output);
	let _ = writeln!(
		output,
		"  {:>6} {:>6} {:>7} {}",
		style::dim("read"),
		style::dim("penalty"),
		style::dim("code"),
		style::dim("file")
	);

	// Sorted by penalty rather than by score, so the table answers "what is costing me" instead of
	// repeating the score ordering the header already shows.
	for (path, penalty, overall, code_lines) in report.file_impact() {
		let _ = writeln!(
			output,
			"  {:>6} {:>6.1} {:>7} {} {}",
			style::score(overall),
			penalty,
			format_number(code_lines),
			style::cyan(&display_path(&path)),
			style::dim(grade(overall))
		);
	}

	let _ = writeln!(output);
}

/// Writes every penalizing finding with its location and explanation.
fn render_findings(output: &mut String, report: &ProjectReport) {
	/// Findings listed before the list is truncated.
	const LIMIT: usize = 200;

	let mut findings: Vec<(PathBuf, Finding)> = report
		.owned_findings_public()
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

	let _ = writeln!(output, "{}", style::bold("all findings"));
	let _ = writeln!(output);

	for (path, finding) in findings.iter().take(LIMIT) {
		let _ = writeln!(
			output,
			"  {} {} {}",
			style::cyan(&format!(
				"{}:{}",
				display_path(path),
				finding.span.start_line
			)),
			style::magenta(&finding.rule),
			style::dim(&format!("[{}]", finding.severity.label()))
		);
		let _ = writeln!(output, "    {}", wrap(&finding.message, 74, "    "));
		let _ = writeln!(
			output,
			"    {} {}",
			style::dim("->"),
			style::dim(&wrap(&finding.suggestion, 74, "    "))
		);
		let _ = writeln!(output);
	}

	if findings.len() > LIMIT {
		let _ = writeln!(
			output,
			"  {}",
			style::dim(&format!("… and {} more findings", findings.len() - LIMIT))
		);
		let _ = writeln!(output);
	}
}

/// Writes a warning for files where tokenization was uncertain.
fn render_unterminated(output: &mut String, report: &ProjectReport) {
	let affected: Vec<&FileReport> = report
		.files
		.iter()
		.filter(|file| !file.unterminated.is_empty())
		.collect();

	if affected.is_empty() {
		return;
	}

	let _ = writeln!(output, "{}", style::yellow("tokenizer warnings"));
	let _ = writeln!(output);
	let _ = writeln!(
		output,
		"  {}",
		wrap(
			"Scores for these files are less reliable because a construct never closed:",
			72,
			"  "
		)
	);

	for file in affected {
		for warning in &file.unterminated {
			let _ = writeln!(
				output,
				"  {} {}",
				style::cyan(&display_path(&file.path)),
				warning
			);
		}
	}

	let _ = writeln!(output);
}

/// Writes the single highest-value next action.
///
/// A report that ends with one specific suggestion is far more likely to be acted on than one that
/// ends with a list.
fn render_next_step(output: &mut String, report: &ProjectReport) {
	// Every rule ranked against one denominator, so the percentage below is a share of the whole
	// score rather than of a single category. Combining the two per-category rankings would compare
	// percentages computed against different totals, which overstates the benefit of a fix.
	let impacts = crate::aggregate::rank_overall_impact(&report.owned_findings_public());

	let Some(top) = impacts.first() else {
		let _ = writeln!(output, "{}", style::green("No rule violations found."));

		return;
	};

	let location = top
		.worst_offender
		.as_ref()
		.map(|offender| format!("{}:{}", display_path(&offender.path), offender.line))
		.unwrap_or_default();

	let _ = writeln!(output, "{}", style::bold("start here"));
	let _ = writeln!(
		output,
		"  Fixing {} at {} would recover {:.1}% of the available points.",
		style::magenta(&top.rule),
		style::cyan(&location),
		top.share * 100.0
	);
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

/// Formats an integer with thousands separators.
fn format_number(value: usize) -> String {
	let digits = value.to_string();
	let mut formatted = String::with_capacity(digits.len() + digits.len() / 3);

	for (index, character) in digits.chars().enumerate() {
		if index > 0 && (digits.len() - index).is_multiple_of(3) {
			formatted.push(',');
		}

		formatted.push(character);
	}

	formatted
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

/// Returns an empty string or an `s`, for pluralization.
fn plural(count: usize) -> &'static str {
	if count == 1 { "" } else { "s" }
}

/// Wraps `text` to `width` columns with `indent` on continuation lines.
fn wrap(text: &str, width: usize, indent: &str) -> String {
	let mut lines: Vec<String> = Vec::new();
	let mut current = String::new();

	for word in text.split_whitespace() {
		if !current.is_empty() && current.len() + word.len() + 1 > width {
			lines.push(std::mem::take(&mut current));
		}

		if !current.is_empty() {
			current.push(' ');
		}

		current.push_str(word);
	}

	if !current.is_empty() {
		lines.push(current);
	}

	lines.join(&format!("\n{indent}"))
}

/// Renders a path relative to the current directory when possible.
fn display_path(path: &Path) -> String {
	if let Ok(current) = std::env::current_dir()
		&& let Ok(relative) = path.strip_prefix(&current)
	{
		return relative.display().to_string();
	}

	path.display().to_string()
}
