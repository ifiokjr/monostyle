//! Markdown analysis.
//!
//! Documentation is where the code people copy from lives, so README examples deserve the
//! same scrutiny as source. This crate extracts the two things that can be measured:
//!
//! - **Code fences**, which are pulled out with their language and line offset so the normal
//!   rule set can score them exactly as it scores a source file. Doing this here rather than
//!   inside the rules crate matters: fences are scored by re-entering the analysis pipeline,
//!   so the extraction must not depend on that pipeline.
//! - **Structure**, meaning heading depth and prose runs, which are properties of the
//!   document rather than of any one code block.
//!
//! # Why fences are scored separately
//!
//! A fence is not a file. Its complexity belongs to the snippet, and its readability should
//! be judged against the expectation that documentation examples are short and direct. By
//! returning each fence as its own unit with the surrounding context preserved — language,
//! starting line, and the info string as written — the caller can attribute findings back to
//! the exact fence a reader would see.

use monostyle_core::Language;
use serde::Serialize;

/// A fenced code block extracted from a Markdown document.
#[derive(Debug, Clone, Serialize)]
pub struct CodeFence {
	/// The language the fence declares, when it is recognizable.
	pub language: Option<Language>,
	/// The info string as written, so unrecognized languages can be reported honestly.
	pub info_string: String,
	/// The fence's code, without the surrounding fence lines.
	pub code: String,
	/// The 1-based line of the opening fence.
	pub start_line: usize,
	/// The 1-based line of the closing fence.
	pub end_line: usize,
	/// The fence character: backtick or tilde.
	pub marker: char,
}

impl CodeFence {
	/// The number of lines of code inside the fence.
	#[must_use]
	pub fn line_count(&self) -> usize {
		self.code.lines().count()
	}

	/// Whether the language was recognized.
	#[must_use]
	pub fn is_recognized(&self) -> bool {
		self.language.is_some()
	}
}

/// A heading in a Markdown document.
#[derive(Debug, Clone, Serialize)]
pub struct Heading {
	/// The heading level, where 1 is `#`.
	pub level: usize,
	/// The heading text.
	pub text: String,
	/// The 1-based line the heading appears on.
	pub line: usize,
}

/// A run of prose without intervening structure.
#[derive(Debug, Clone, Serialize)]
pub struct ProseRun {
	/// The 1-based line the run starts on.
	pub start_line: usize,
	/// The number of consecutive prose lines.
	pub length: usize,
}

/// The structural analysis of a Markdown document.
#[derive(Debug, Clone, Serialize)]
pub struct Document {
	/// Every heading, in document order.
	pub headings: Vec<Heading>,
	/// Every code fence, in document order.
	pub fences: Vec<CodeFence>,
	/// Every consecutive run of prose lines, in document order.
	///
	/// All of them, not only the long ones: the threshold is a rule setting, and filtering here would
	/// make that setting inert.
	pub prose_runs: Vec<ProseRun>,
	/// Total lines in the document.
	pub total_lines: usize,
}

/// Heading levels that are skipped, such as `#` followed directly by `###`.
#[derive(Debug, Clone, Serialize)]
pub struct SkippedHeading {
	/// The level that was skipped over.
	pub from: usize,
	/// The level that appeared instead.
	pub to: usize,
	/// The line of the offending heading.
	pub line: usize,
}

/// Analyzes `source` as Markdown.
#[must_use]
pub fn analyze(source: &str) -> Document {
	let lines: Vec<&str> = source.lines().collect();
	let mut document = Document {
		headings: Vec::new(),
		fences: Vec::new(),
		prose_runs: Vec::new(),
		total_lines: lines.len(),
	};

	let mut index = 0;
	let mut prose_start: Option<usize> = None;
	let mut prose_length = 0;

	while index < lines.len() {
		let Some(line) = lines.get(index).copied() else {
			break;
		};

		if let Some(heading) = parse_heading(line) {
			flush_prose(&mut document, &mut prose_start, &mut prose_length);
			document.headings.push(Heading { ..heading });

			index += 1;
			continue;
		}

		if let Some((marker, info)) = parse_fence_open(line) {
			flush_prose(&mut document, &mut prose_start, &mut prose_length);

			let start_line = index + 1;
			let mut body = Vec::new();
			let mut cursor = index + 1;

			while let Some(candidate) = lines.get(cursor).copied() {
				if is_fence_close(candidate, marker) {
					break;
				}

				body.push(candidate);
				cursor += 1;
			}

			let info_string = info.trim().to_string();
			let language = Language::from_fence_tag(&info_string);

			document.fences.push(CodeFence {
				language,
				info_string,
				code: body.join("\n"),
				start_line,
				end_line: (cursor + 1).min(lines.len()),
				marker,
			});

			// The cursor stops on the closing fence; advancing past it resumes normal parsing.
			index = cursor + 1;
			continue;
		}

		if is_prose(line) {
			prose_start.get_or_insert(index);
			prose_length += 1;
		} else {
			flush_prose(&mut document, &mut prose_start, &mut prose_length);
		}

		index += 1;
	}

	flush_prose(&mut document, &mut prose_start, &mut prose_length);

	document
}

/// Returns headings whose level jumps by more than one.
///
/// A `#` followed by `###` breaks the document outline: assistive technology and generated
/// tables of contents both rely on levels being sequential.
#[must_use]
pub fn skipped_headings(headings: &[Heading]) -> Vec<SkippedHeading> {
	let mut skipped = Vec::new();
	let mut previous: Option<usize> = None;

	for heading in headings {
		if let Some(level) = previous
			&& heading.level > level + 1
		{
			skipped.push(SkippedHeading {
				from: level,
				to: heading.level,
				line: heading.line,
			});
		}

		previous = Some(heading.level);
	}

	skipped
}

/// Records a finished prose run.
///
/// Every run is recorded, including short ones, and the caller decides which are long enough to report.
/// A hardcoded threshold here meant `max-prose-run` had no effect: the analysis filtered the runs before
/// the rule could apply the configured limit, so raising or lowering the setting changed nothing.
fn flush_prose(document: &mut Document, prose_start: &mut Option<usize>, prose_length: &mut usize) {
	if let Some(start) = prose_start.take()
		&& *prose_length > 0
	{
		document.prose_runs.push(ProseRun {
			start_line: start + 1,
			length: *prose_length,
		});
	}

	*prose_length = 0;
}

/// Parses an ATX heading, returning its level and text.
fn parse_heading(line: &str) -> Option<Heading> {
	let trimmed = line.trim_start();

	if !trimmed.starts_with('#') {
		return None;
	}

	let level = trimmed
		.chars()
		.take_while(|character| *character == '#')
		.count();

	// More than six hashes is not a heading in CommonMark.
	if level == 0 || level > 6 {
		return None;
	}

	let text = trimmed[level..]
		.trim()
		.trim_end_matches('#')
		.trim()
		.to_string();

	Some(Heading {
		level,
		text,
		line: 0,
	})
}

/// Parses an opening fence, returning its marker character and info string.
fn parse_fence_open(line: &str) -> Option<(char, String)> {
	let trimmed = line.trim_start();

	let marker = match trimmed.chars().next() {
		Some('`') => '`',
		Some('~') => '~',
		_ => return None,
	};

	let count = trimmed
		.chars()
		.take_while(|character| *character == marker)
		.count();

	// CommonMark requires at least three characters for a fence.
	if count < 3 {
		return None;
	}

	// A backtick fence's info string may not contain a backtick, which is what keeps a line
	// of inline code from being read as a fence.
	let info = &trimmed[count..];

	if marker == '`' && info.contains('`') {
		return None;
	}

	Some((marker, info.to_string()))
}

/// Returns true when `line` closes a fence opened with `marker`.
fn is_fence_close(line: &str, marker: char) -> bool {
	let trimmed = line.trim_start();
	let count = trimmed
		.chars()
		.take_while(|character| *character == marker)
		.count();

	count >= 3 && trimmed[count..].trim().is_empty()
}

/// Returns true when a line is prose rather than blank or structural.
fn is_prose(line: &str) -> bool {
	let trimmed = line.trim();

	if trimmed.is_empty() {
		return false;
	}

	// Lists, quotes, tables, and thematic breaks are structure, not prose runs.
	let structural = trimmed.starts_with(['-', '*', '+', '>', '|', '=', '_'])
		|| trimmed.starts_with("![")
		|| trimmed.starts_with('[')
		|| trimmed
			.chars()
			.next()
			.is_some_and(|character| character.is_ascii_digit())
			&& trimmed.contains(". ");

	!structural
}
