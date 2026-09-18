//! Line-length rules.
//!
//! Line length is a formatting concern, so this module treats it as one.
//!
//! # What is measured, and what is not
//!
//! The rule counts the **display width** of code on a line, with tabs expanded to the indentation
//! width. It does not count trailing comments, because a long comment is prose and prose wraps
//! differently than code.
//!
//! Two exclusions matter in practice:
//!
//! - **Markdown prose is never measured.** A paragraph is wrapped by whoever wrote it, and a
//!   Markdown table or a long URL legitimately exceeds any code limit. Only fenced code inside a
//!   Markdown file is measured, and it is measured against its own language's rules.
//! - **A line that cannot be broken is not reported.** A long string literal, a URL in a string, or
//!   a generated single-line structure has nowhere to wrap to, so a finding would be unactionable.
//!
//! # Why the limit is configurable
//!
//! The right limit is a project's formatting decision — a formatter's `print-width` is the authority
//! — so a hardcoded value would disagree with the project's own tooling. The default matches the
//! most common formatter settings, and a project that formats to a different width sets
//! `max-line-width` to match.

use monostyle_core::Category;
use monostyle_core::Finding;
use monostyle_core::FindingBuilder;
use monostyle_core::Language;
use monostyle_core::Severity;
use monostyle_core::Span;
use monostyle_lexer::LexedFile;
use monostyle_lexer::LexedLine;

use crate::config::RulesConfig;

/// Reports lines wider than the configured limit.
///
/// The width is compared against the project's own limit, so the rule agrees with the formatter
/// rather than fighting it.
#[must_use]
pub fn overlong_lines(file: &LexedFile, config: &RulesConfig) -> Vec<Finding> {
	// Markdown prose is not code, so it has no line limit. Only its fenced code is measured, and
	// that happens when the fence is analyzed as its own language.
	if file.language == Language::Markdown {
		return Vec::new();
	}

	let limit = config.max_line_width;

	if limit == 0 {
		return Vec::new();
	}

	let severe = config.severe_line_width();
	let mut findings = Vec::new();

	for line in &file.lines {
		let Some(severity) = overlong_severity(line, limit, severe) else {
			continue;
		};

		findings.push(overlong_finding(line, severity, limit));
	}

	findings
}

/// Returns the severity when a line is over the limit, or `None` when it is not reported.
///
/// Returning the severity rather than a boolean keeps the decision in one place: the loop no longer
/// needs to re-derive why a line was skipped.
fn overlong_severity(line: &LexedLine, limit: usize, severe: usize) -> Option<Severity> {
	if !line.is_code() {
		return None;
	}

	let width = display_width(line);

	if width <= limit {
		return None;
	}

	// A line with no breakable boundary has nowhere to wrap to. Reporting it would tell the reader to
	// do something the language does not allow.
	if !is_breakable(line) {
		return None;
	}

	if width > severe {
		return Some(Severity::Major);
	}

	Some(Severity::Minor)
}

/// Builds the finding for a line that is over the limit.
fn overlong_finding(line: &LexedLine, severity: Severity, limit: usize) -> Finding {
	let width = display_width(line);

	FindingBuilder::new(
		"readability/overlong-line",
		Category::Readability,
		Span::new(line.start_byte, line.end_byte, line.number, line.number),
	)
	.severity(severity)
	.weight(0.5)
	.message(format!(
		"this line is {width} columns wide, over the {limit} limit"
	))
	.suggestion(
		"Break this line at a logical boundary, or extract part of the expression into \
		 a named variable.",
	)
	.build()
}

/// Returns a line's display width, with tabs expanded.
///
/// Trailing comments are excluded because a comment is prose, and the column a code reader cares
/// about ends where the code does.
fn display_width(line: &LexedLine) -> usize {
	line.text
		.chars()
		.take(line.trailing_comment_column.unwrap_or(usize::MAX))
		.fold(0, |width, character| {
			if character == '\t' {
				width + 4
			} else {
				width + 1
			}
		})
}

/// Returns true when a line has a place to break.
///
/// A line is breakable when it contains a delimiter a reader could wrap at: an argument separator, a
/// boolean operator, or a chained call. A single long literal or URL has none.
fn is_breakable(line: &LexedLine) -> bool {
	/// Delimiters that indicate a natural wrap point.
	const BREAK_POINTS: &[&str] = &[",", "&&", "||", "+", " and ", " or ", "->", "=>", "?", ":"];

	let masked = &line.masked_code;

	// A line that is mostly one literal has no break point, even if punctuation appears inside the
	// literal itself; the masked view already hides literal contents, so a low ratio of code to
	// width means the length comes from something unbreakable.
	let literal_share = line.text.chars().count();
	let code_share = masked
		.chars()
		.filter(|character| !character.is_whitespace())
		.count();

	if code_share * 4 < literal_share {
		return false;
	}

	BREAK_POINTS.iter().any(|point| masked.contains(point))
}
