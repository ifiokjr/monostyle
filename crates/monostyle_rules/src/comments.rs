//! Comment rules.
//!
//! Comments are the one place where monostyle both takes points and gives them. A comment
//! that explains why complex code exists reduces the reader's burden and earns credit; a
//! comment that narrates what the code already says adds noise and loses points.
//!
//! # Why credit is a negative finding
//!
//! Credit is modelled as a [`Finding`] with a negative weight rather than as a separate
//! bonus channel. That keeps a single number — the penalty density — sufficient to explain
//! any final score, instead of requiring the reader to reconcile a penalty total with a
//! bonus total.
//!
//! # Why the heuristic stays quiet
//!
//! Distinguishing a why-comment from a how-comment is genuinely hard, and wrongly penalizing
//! a good explanation costs far more trust than missing a bad one. The classifier therefore
//! only fires on phrases that are essentially never used to narrate code, and treats a long
//! unmarked comment as an explanation.

use monostyle_core::Category;
use monostyle_core::Finding;
use monostyle_core::FindingBuilder;
use monostyle_core::Severity;
use monostyle_core::Span;
use monostyle_lexer::CommentIntent;
use monostyle_lexer::LexedFile;

use crate::config::RulesConfig;
use crate::metrics_bridge;

/// Reports complex units that carry no explanatory comment.
///
/// This is the rule that makes the "comment your complex code" advice checkable. It only
/// fires where complexity genuinely warrants explanation, so simple code is never nagged
/// into adding noise.
pub fn comment_required_on_complex_units(file: &LexedFile, config: &RulesConfig) -> Vec<Finding> {
	if !config.require_comment_on_complex_units {
		return Vec::new();
	}

	let units = metrics_bridge::units(file);
	let mut findings = Vec::new();

	for unit in units {
		let lines = metrics_bridge::unit_lines(file, &unit);
		let cognitive = monostyle_metrics::cognitive_complexity_of_lines(lines);

		if cognitive.total < config.comment_required_above_cognitive {
			continue;
		}

		// A unit counts as documented when a why-comment or a doc comment appears anywhere
		// inside it — including the declaration line, which is where the explanation belongs.
		let documented = file
			.lines
			.iter()
			.filter(|line| unit.contains(line.number))
			.filter_map(|line| line.comment_intent)
			.any(|intent| matches!(intent, CommentIntent::Why | CommentIntent::Documentation));

		if documented {
			continue;
		}

		findings.push(
			FindingBuilder::new(
				"readability/comment-required-on-complex-unit",
				Category::Readability,
				Span::new(0, 0, unit.start_line, unit.start_line),
			)
			.severity(Severity::Major)
			.weight(1.5)
			.message(format!(
				"`{}` has a cognitive complexity of {} but no comment explaining it",
				unit.name, cognitive.total
			))
			.suggestion(
				"Add a comment explaining why this function is as complex as it is — the \
				 constraint, tradeoff, or history that makes the branching necessary.",
			)
			.build(),
		);
	}

	findings
}

/// Rewards comments that explain why, and penalizes comments that narrate what.
///
/// Each why-comment earns a small credit and each narrating comment a small penalty. The
/// weights are deliberately small and roughly symmetric, so commenting is not a way to buy a
/// score — the surrounding code still has to be readable.
pub fn comment_quality(file: &LexedFile, config: &RulesConfig) -> Vec<Finding> {
	let mut findings = Vec::new();

	for line in &file.lines {
		let Some(intent) = line.comment_intent else {
			continue;
		};

		let span = Span::new(0, line.text.len(), line.number, line.number);

		match intent {
			CommentIntent::Why => {
				findings.push(
					FindingBuilder::new(
						"readability/comment-explains-why",
						Category::Readability,
						span,
					)
					.severity(Severity::Minor)
					// Negative weight: this is credit, not a penalty.
					.weight(-0.5)
					.message("this comment explains why the code is the way it is")
					.suggestion("No action needed; this is the kind of comment to keep.")
					.build(),
				);
			}
			CommentIntent::How if config.penalize_narrating_comments => {
				findings.push(
					FindingBuilder::new(
						"readability/comment-narrates-code",
						Category::Readability,
						span,
					)
					.severity(Severity::Minor)
					.weight(0.75)
					.message("this comment restates what the code already says")
					.suggestion(
						"Delete this comment, or replace it with an explanation of why the code \
						 does this rather than what it does.",
					)
					.build(),
				);
			}
			_ => {}
		}
	}

	findings
}

/// Reports files where comments outweigh the code they describe.
///
/// Past a point, more commentary makes code harder rather than easier to follow: the reader
/// must hold both the prose and the code in mind, and the prose ages badly as the code
/// changes.
///
/// Documentation comments are excluded from the count. A well-documented module or public API
/// legitimately has more comment lines than code, and penalizing that would push authors
/// toward under-documenting — the opposite of the intent. What this rule targets is a surplus
/// of *ordinary* commentary.
pub fn excessive_comments(file: &LexedFile, config: &RulesConfig) -> Vec<Finding> {
	/// Minimum lines of code before a comment ratio is meaningful.
	const MIN_CODE_LINES: usize = 30;

	let comment_lines = file
		.lines
		.iter()
		.filter(|line| line.is_comment())
		.filter(|line| line.comment_intent != Some(CommentIntent::Documentation))
		.count();

	let code_lines = file.source_line_count();

	// A file needs enough code for a ratio to be meaningful; below this, a short file with a
	// couple of explanatory comments trips the limit for no good reason.

	if code_lines < MIN_CODE_LINES {
		return Vec::new();
	}

	let ratio = comment_lines as f64 / code_lines as f64;

	if ratio <= config.max_comment_ratio {
		return Vec::new();
	}

	let span = file.lines.first().map_or(Span::new(0, 0, 1, 1), |line| {
		Span::new(0, line.text.len(), line.number, line.number)
	});

	vec![
		FindingBuilder::new(
			"readability/excessive-comments",
			Category::Readability,
			span,
		)
		.severity(Severity::Minor)
		.weight(1.0)
		.message(format!(
			"this file has {comment_lines} comment lines against {code_lines} lines of \
				 code, a ratio of {ratio:.2} over the {:.2} limit",
			config.max_comment_ratio
		))
		.suggestion(
			"Keep the comments that explain why, and delete the ones that restate the code \
				 or record its history.",
		)
		.build(),
	]
}

/// Reports documentation comments that say nothing about purpose or reasoning.
///
/// A doc comment block that only describes parameters, without stating what the item is for
/// or why it exists, is the documentation equivalent of narrating code.
pub fn thin_documentation(file: &LexedFile) -> Vec<Finding> {
	let mut findings = Vec::new();
	let mut run: Vec<usize> = Vec::new();

	for (index, line) in file.lines.iter().enumerate() {
		let is_doc = line.comment_intent == Some(CommentIntent::Documentation);

		if is_doc {
			run.push(index);
			continue;
		}

		check_documentation_run(file, &run, &mut findings);
		run.clear();
	}

	check_documentation_run(file, &run, &mut findings);
	findings
}

/// Reports a documentation block that lacks any explanatory prose.
///
/// A block is considered explanatory when it contains a sentence of reasonable length that
/// is not merely a parameter annotation. Short one-line markers such as `/// The port.` are
/// left alone, because penalizing them would discourage documenting at all.
fn check_documentation_run(file: &LexedFile, run: &[usize], findings: &mut Vec<Finding>) {
	/// Words of non-annotation prose a documentation block should contain.
	const MIN_PROSE_WORDS: usize = 8;

	if run.len() < 3 {
		return;
	}

	let mut prose_words = 0;

	for index in run {
		let Some(line) = file.lines.get(*index) else {
			continue;
		};

		let body = line.text.trim_start_matches(['/', '#', ' ', '\t', '*']);

		// Annotations are structural, not explanatory: `@param`, `# Arguments`, `:return:`.
		let is_annotation = body.starts_with('@')
			|| body.starts_with(':')
			|| body.starts_with("Arguments")
			|| body.starts_with("Parameters")
			|| body.starts_with("Returns")
			|| body.starts_with("Example")
			|| body.starts_with("Notes");

		if !is_annotation {
			prose_words += body.split_whitespace().count();
		}
	}

	if prose_words >= MIN_PROSE_WORDS {
		return;
	}

	let Some(first) = run.first().and_then(|index| file.lines.get(*index)) else {
		return;
	};

	findings.push(
		FindingBuilder::new(
			"readability/thin-documentation",
			Category::Readability,
			Span::new(0, first.text.len(), first.number, first.number),
		)
		.severity(Severity::Minor)
		.weight(0.5)
		.message(format!(
			"this {} line documentation block lists structure without explaining purpose",
			run.len()
		))
		.suggestion(
			"Add a sentence stating what this is for and why it exists, so the annotations have \
			 context.",
		)
		.build(),
	);
}
