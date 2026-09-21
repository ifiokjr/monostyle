//! Markdown rules.
//!
//! Readme examples are the code people copy, so they are held to the same layout standard as
//! source. The trick that makes this cheap is that a fence is just source with a language, so
//! each fence can be lexed and put through the same layout rules as a file.
//!
//! # Attributing findings back
//!
//! A fence's code starts at some line inside a larger document. Findings are reported against
//! the *Markdown* file's line numbers rather than the fence's, because that is the line a
//! reader can jump to. The offset is applied when the finding is built, which is why these
//! rules construct spans rather than reusing the ones the nested rules produced.

use monostyle_core::Category;
use monostyle_core::Finding;
use monostyle_core::FindingBuilder;
use monostyle_core::Severity;
use monostyle_core::Span;
use monostyle_lexer::LexedFile;
use monostyle_lexer::LexedLine;
use monostyle_lexer::lex;
use monostyle_markdown::CodeFence;

use crate::config::RulesConfig;
use crate::line_length;
use crate::structure;
use crate::whitespace;

/// Scores the readability of code inside Markdown fences.
///
/// The layout rules that apply to a source file apply here too, but the threshold for
/// complaint is lower: a documentation example that needs a second read has already failed at
/// its job.
pub fn fence_readability(file: &LexedFile, config: &RulesConfig) -> Vec<Finding> {
	if !config.score_markdown_fences {
		return Vec::new();
	}

	let mut findings = Vec::new();

	for fence in &scoreable_fences(file) {
		let Some(language) = fence.language else {
			continue;
		};

		if fence.code.trim().is_empty() {
			continue;
		}

		let lexed = lex(&fence.code, language);
		let nested = nested_layout_findings(&lexed, config);

		for finding in nested {
			findings.push(rebase(finding, fence, &language.to_string()));
		}
	}

	findings
}

/// Reports fences that declare a language the scanner does not know.
///
/// An example in a language monostyle cannot read is not measured at all, so saying so is more useful
/// than silently scoring the document as though the example were fine. It is usually a misspelling in
/// the info string.
pub fn fence_language_unknown(file: &LexedFile, config: &RulesConfig) -> Vec<Finding> {
	if !config.score_markdown_fences {
		return Vec::new();
	}

	unscoreable_fences(file, true)
}

/// Reports fences with no language tag.
///
/// An untagged fence cannot be scored, and it is also the fence a reader cannot copy usefully: there
/// is no way to tell what language the example is in.
pub fn fence_without_language(file: &LexedFile, config: &RulesConfig) -> Vec<Finding> {
	if !config.score_markdown_fences {
		return Vec::new();
	}

	unscoreable_fences(file, false)
}

/// Every fence in the document.
fn scoreable_fences(file: &LexedFile) -> Vec<CodeFence> {
	let source = document_source(file);

	monostyle_markdown::analyze(&source).fences
}

/// Reports the fences that cannot be scored, filtered to one reason.
///
/// The two reasons are separate rules because they have different remedies — tag the fence, or fix the
/// spelling — and because a project may reasonably decide that untagged fences are fine while
/// misspelled ones are not.
fn unscoreable_fences(file: &LexedFile, annotated: bool) -> Vec<Finding> {
	scoreable_fences(file)
		.iter()
		.filter(|fence| fence.language.is_none())
		.filter(|fence| fence.info_string.trim().is_empty() != annotated)
		.map(unscoreable_fence)
		.collect()
}

/// Returns the document's text, reconstructed from its lines.
fn document_source(file: &LexedFile) -> String {
	file.lines
		.iter()
		.map(|line| line.text.as_str())
		.collect::<Vec<_>>()
		.join("\n")
}

/// Builds the finding for a fence that cannot be scored.
fn unscoreable_fence(fence: &CodeFence) -> Finding {
	let annotated = !fence.info_string.trim().is_empty();

	FindingBuilder::new(
		if annotated {
			"markdown/fence-language-unknown"
		} else {
			"markdown/fence-without-language"
		},
		Category::Readability,
		Span::new(0, fence.start_line, fence.start_line, fence.start_line),
	)
	.severity(Severity::Minor)
	.weight(0.5)
	.message(if annotated {
		format!(
			"this fence declares an unsupported language `{}`",
			fence.info_string
		)
	} else {
		"this fence has no language tag".to_string()
	})
	.suggestion(if annotated {
		"Use a language monostyle supports, or check the spelling of the info string."
	} else {
		"Tag the fence with its language so readers know what they are copying and so the \
		 example can be analyzed."
	})
	.build()
}

/// Runs the layout rules over a fence's lexed code.
///
/// Only the layout and structure rules run here. Complexity rules are deliberately excluded:
/// a documentation example that walks through a messy state on purpose is doing its job, and
/// penalizing it would push authors toward hiding the very complexity they are explaining.
///
/// The set is listed explicitly rather than delegated to [`crate::run_rules`], which would run the
/// whole code rule set over the document. A Markdown file lexes as one language, so delegating would
/// score the surrounding prose as code — reporting a numbered list as a statement run — and would
/// attribute those findings to the wrong lines.
fn nested_layout_findings(lexed: &LexedFile, config: &RulesConfig) -> Vec<Finding> {
	let mut findings = Vec::new();

	findings.extend(whitespace::blank_line_before_control_flow(lexed, config));
	findings.extend(whitespace::blank_line_before_return(lexed, config));
	findings.extend(whitespace::group_separation(lexed, config));
	findings.extend(whitespace::excessive_indentation(lexed, config));
	findings.extend(whitespace::mixed_indentation(lexed));
	findings.extend(structure::deep_nesting(lexed, config));
	findings.extend(structure::long_parameter_list(lexed, config));
	findings.extend(line_length::overlong_lines(lexed, config));

	findings
}

/// Rewrites a finding's span to point at the Markdown document instead of the fence.
fn rebase(finding: Finding, fence: &CodeFence, language: &str) -> Finding {
	// The fence's code begins on the line after its opening marker.
	let offset = fence.start_line;
	let start_line = offset + finding.span.start_line;
	let end_line = offset + finding.span.end_line;

	Finding {
		rule: finding.rule,
		category: finding.category,
		severity: finding.severity,
		span: Span::new(
			finding.span.start_byte,
			finding.span.end_byte,
			start_line,
			end_line,
		),
		message: format!("in the {language} example: {}", finding.message),
		suggestion: finding.suggestion,
		weight: finding.weight,
		// A fix inside a fence would address offsets in the extracted code, not the document, so
		// the edit cannot be applied without translating every position back.
		fix: None,
	}
}

/// Reports long unbroken runs of prose.
///
/// A wall of text is the prose equivalent of a function with no blank lines: the reader has
/// nowhere to rest and no visible structure to navigate by.
pub fn prose_runs(file: &LexedFile, config: &RulesConfig) -> Vec<Finding> {
	if file.language != monostyle_core::Language::Markdown {
		return Vec::new();
	}

	let source = file
		.lines
		.iter()
		.map(|line| line.text.as_str())
		.collect::<Vec<_>>()
		.join("\n");
	let document = monostyle_markdown::analyze(&source);

	document
		.prose_runs
		.iter()
		.filter(|run| run.length >= config.max_prose_run)
		.map(|run| {
			FindingBuilder::new(
				"markdown/prose-run",
				Category::Readability,
				Span::new(0, 0, run.start_line, run.start_line),
			)
			.severity(Severity::Minor)
			.weight(0.5)
			.message(format!(
				"{} consecutive lines of prose with no heading, list, or code to break it up",
				run.length
			))
			.suggestion(
				"Break this section up with a heading, a list, or a code example so the reader \
				 has visible structure to scan.",
			)
			.build()
		})
		.collect()
}

/// Reports a document that does not begin with a top-level heading.
///
/// A document whose first heading is level 2 or deeper has no title, which breaks outlines and
/// generated navigation. That is why it is its own rule: the remedy is to add one heading, not to
/// renumber the rest.
pub fn missing_title(file: &LexedFile) -> Vec<Finding> {
	if !is_markdown(file) {
		return Vec::new();
	}

	let headings = headings_of(file);

	let Some(first) = headings.first().filter(|heading| heading.level != 1) else {
		return Vec::new();
	};

	vec![
		FindingBuilder::new(
			"markdown/no-title",
			Category::Readability,
			Span::new(0, 0, first.line, first.line),
		)
		.severity(Severity::Minor)
		.weight(0.5)
		.message(format!(
			"the document starts at heading level {} rather than with a top-level title",
			first.level
		))
		.suggestion("Start the document with a single level-1 heading naming its subject.")
		.build(),
	]
}

/// Reports heading levels that skip a step.
///
/// Jumping from level 1 to level 3 breaks the outline, because the level-2 step exists only implicitly.
pub fn skipped_heading_levels(file: &LexedFile) -> Vec<Finding> {
	if !is_markdown(file) {
		return Vec::new();
	}

	monostyle_markdown::skipped_headings(&headings_of(file))
		.into_iter()
		.map(|skipped| {
			FindingBuilder::new(
				"markdown/skipped-heading-level",
				Category::Readability,
				Span::new(0, 0, skipped.line, skipped.line),
			)
			.severity(Severity::Minor)
			.weight(0.5)
			.message(format!(
				"heading level jumps from {} to {}",
				skipped.from, skipped.to
			))
			.suggestion(
				"Use the next heading level down rather than skipping one, so the document \
				 outline stays navigable.",
			)
			.build()
		})
		.collect()
}

/// Reports every heading-structure problem in one pass.
///
/// Kept for callers that want both checks together; the registry runs the two rules separately so each
/// can be disabled on its own.
pub fn heading_structure(file: &LexedFile) -> Vec<Finding> {
	let mut findings = missing_title(file);

	findings.extend(skipped_heading_levels(file));

	findings
}

/// Whether the file is a Markdown document.
fn is_markdown(file: &LexedFile) -> bool {
	file.language == monostyle_core::Language::Markdown
}

/// The document's headings.
fn headings_of(file: &LexedFile) -> Vec<monostyle_markdown::Heading> {
	monostyle_markdown::analyze(&document_source(file)).headings
}

/// Returns the lines of a lexed file, for callers that need them.
///
/// This exists so the module can be tested without reaching into `LexedFile` internals.
#[must_use]
pub fn lexed_lines(file: &LexedFile) -> &[LexedLine] {
	&file.lines
}
