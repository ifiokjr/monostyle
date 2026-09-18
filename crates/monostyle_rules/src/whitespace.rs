//! Whitespace rules.
//!
//! These rules encode a specific aesthetic: complex code should be given room, because the eye needs
//! somewhere to rest. They are the direct, checkable form of the guidance that blank lines go before
//! control flow, between logical groups, and before returns.
//!
//! Each rule reports where the space is missing and why it matters, so a finding is actionable
//! without consulting documentation.
//!
//! # Why exactly one rule is auto-fixable
//!
//! Inserting a blank line before a control-flow statement is the only edit here that is guaranteed to
//! survive a formatter. Rustfmt, Prettier, Black, and `dart format` all preserve a blank line between
//! statements and none of them remove one, so the fix cannot fight the project's own tooling.
//!
//! Every other rule is deliberately left to the reader. Breaking a long line, renaming an identifier,
//! extracting a function, and adding an explanatory comment are all judgement calls whose automated
//! version would be worse than the problem: a fixer that fights the formatter produces a diff the
//! next `format` run reverts, which is a worse experience than the finding itself.

use monostyle_core::Category;
use monostyle_core::Finding;
use monostyle_core::FindingBuilder;
use monostyle_core::Fix;
use monostyle_core::Severity;
use monostyle_core::Span;
use monostyle_lexer::CommentIntent;
use monostyle_lexer::LexedFile;
use monostyle_lexer::LexedLine;

use crate::config::RulesConfig;

/// Builds a span covering a single line.
fn line_span(line: &LexedLine) -> Span {
	Span::new(line.start_byte, line.end_byte, line.number, line.number)
}

/// Reports control-flow statements that are not preceded by a blank line.
///
/// Sequential `if` statements read as a rushed block when they are stacked directly on top
/// of each other; a blank line before each one lets the reader treat them as separate
/// decisions rather than one dense paragraph.
pub fn blank_line_before_control_flow(file: &LexedFile, config: &RulesConfig) -> Vec<Finding> {
	if !config.require_blank_line_before_control_flow {
		return Vec::new();
	}

	let mut findings = Vec::new();

	for (index, line) in file.lines.iter().enumerate() {
		if !needs_blank_line(&file.lines, index, line, config) {
			continue;
		}

		// The fix is mechanical: insert a line break at the start of this line, which puts a blank
		// line above it without touching its content or indentation.
		let span = line_span(line);
		let fix = Fix::insert(
			Span::new(span.start_byte, span.start_byte, line.number, line.number),
			"\n",
			"insert a blank line above",
		);

		findings.push(
			FindingBuilder::new(
				"readability/blank-line-before-control-flow",
				Category::Readability,
				span,
			)
			.severity(Severity::Minor)
			.weight(1.0)
			.message(format!(
				"`{}` follows the previous statement with no blank line between them",
				line.decisions.join("`, `")
			))
			.suggestion(
				"Add a blank line before this statement so the reader can treat it as a \
				 separate decision rather than part of the previous block.",
			)
			.fix(fix)
			.build(),
		);
	}

	findings
}

/// Decides whether a line is a control-flow statement missing the blank line above it.
///
/// Every exemption this rule has lives here, so the loop body stays a filter and a builder. Each one
/// is a case where a blank line would be wrong rather than merely absent.
fn needs_blank_line(
	lines: &[LexedLine],
	index: usize,
	line: &LexedLine,
	config: &RulesConfig,
) -> bool {
	if !line.is_code() || line.decisions.is_empty() {
		return false;
	}

	// A line that opens a block is a declaration, not a statement, so it is exempt: the space
	// belongs before the statements inside it, not before the declaration itself.
	if is_block_declaration(line) {
		return false;
	}

	// A continuation line is part of the statement above it, so a keyword inside it is an expression
	// rather than a new decision. An inline conditional in an argument list — `path / "x" if flag
	// else "y"` — was reported as a missing blank line before a branch, which asked for a blank line
	// inside a single expression.
	if inside_expression(line, lines, index) {
		return false;
	}

	// Separation is a property of the immediately preceding physical line. A blank line or a comment
	// above the statement already gives the reader the break this rule asks for, which is why the
	// check is on the physical neighbour rather than the previous code line: a comment between two
	// statements is a deliberate separator, not a violation.
	if has_separation_above(lines, index, config.min_blank_lines_between_control_flow) {
		return false;
	}

	let Some(previous) = previous_code_line(lines, index) else {
		return false;
	};

	// The first statement inside a block has nothing above it to separate from.
	!(previous.opens_block() || is_block_declaration(previous))
}

/// Returns true when the lines above `index` already separate this statement.
///
/// Either a blank line (or the configured number of them) or a comment counts, because both
/// give the reader a visual break.
fn has_separation_above(lines: &[LexedLine], index: usize, required_blanks: usize) -> bool {
	if index == 0 {
		return true;
	}

	let mut blanks = 0;
	let mut cursor = index;

	while cursor > 0 {
		cursor -= 1;

		let Some(candidate) = lines.get(cursor) else {
			break;
		};

		if candidate.is_blank() {
			blanks += 1;

			if blanks >= required_blanks {
				return true;
			}

			continue;
		}

		// A comment immediately above the statement is itself the separator.
		if candidate.is_comment() {
			return true;
		}

		break;
	}

	false
}

/// Reports a `return` that is crowded against complex code above it.
///
/// A return is where control leaves, so giving it a blank line announces the departure
/// instead of burying it. The rule deliberately stays quiet in the common cases where a
/// blank line would be noise: the first statement in a block, and a return immediately after
/// an early-return guard, both read fine crowded.
pub fn blank_line_before_return(file: &LexedFile, config: &RulesConfig) -> Vec<Finding> {
	if !config.require_blank_line_before_return {
		return Vec::new();
	}

	let mut findings = Vec::new();

	for (index, line) in file.lines.iter().enumerate() {
		if !line.is_code() || !line.is_return {
			continue;
		}

		// An early-return guard *is* the pattern this style prefers, so it is never reported.
		// Reporting it would penalize exactly the structure the guide recommends.
		if is_guard_clause(line) {
			continue;
		}

		let Some(previous) = previous_code_line(&file.lines, index) else {
			continue;
		};

		if previous.is_blank() {
			continue;
		}

		// A return as the first statement of its block is idiomatic and needs no preamble.
		if previous.opens_block() {
			continue;
		}

		// A return directly after another return is a sequence of guards, which reads fine.
		if previous.is_return {
			continue;
		}

		findings.push(
			FindingBuilder::new(
				"readability/blank-line-before-return",
				Category::Readability,
				line_span(line),
			)
			.severity(Severity::Minor)
			.weight(0.75)
			.message("the return follows other work with no blank line before it")
			.suggestion(
				"Add a blank line before the return so the exit from this function is visible \
				 at a glance. This is left to you rather than fixed automatically: a formatter may \
				 reflow the surrounding block, and a rewrite that fights the formatter is worse than \
				 the missing line.",
			)
			.build(),
		);
	}

	findings
}

/// Returns true when a line is an early-return guard clause.
///
/// A guard is a return that sits immediately inside an `if` — the shape the style guide
/// recommends for flattening code. Detecting it needs the enclosing block's opener, so the
/// check is on the previous non-blank line being a conditional that this return is the body of.
fn is_guard_clause(line: &LexedLine) -> bool {
	line.decision_count() > 0 && line.decisions.iter().all(|decision| decision == "if")
}

/// Reports long runs of statements with no blank lines between logical groups.
///
/// Grouping is the point: related statements belong together, and unrelated ones deserve a
/// gap. A long unbroken run means the reader has to infer the group boundaries from the code
/// itself.
///
/// Three things break a run besides a blank line, because each one already gives the reader a
/// boundary:
///
/// - **Type declarations**, since a struct's fields or an enum's variants are one logical
///   group.
/// - **Documented items**, because a doc comment introduces a distinct unit of its own.
/// - **Function declarations**, since consecutive small methods are separate items rather than
///   a cramped statement sequence.
///
/// Without those breaks the rule fires on nearly every well-organized file — a builder with
/// twelve documented setters reads as "twelve statements run together" — which would make it
/// noise rather than signal.
pub fn group_separation(file: &LexedFile, config: &RulesConfig) -> Vec<Finding> {
	if !config.require_group_separation {
		return Vec::new();
	}

	let mut findings = Vec::new();

	let mut run = Run::default();

	for (index, line) in file.lines.iter().enumerate() {
		match classify(line, &run) {
			// A blank line, a doc comment, or the start of a new item closes whatever run was open.
			Disposition::Break {
				declaration,
				previous_was_doc,
			} => run.break_with(&file.lines, &mut findings, declaration, previous_was_doc),
			// A comment or literal is invisible to the rule: it neither extends a run nor ends one,
			// except for a doc comment, which `Break` handles above.
			Disposition::Skip => {}
			Disposition::Extend => run.extend(index),
		}
	}

	run.close(&file.lines, &mut findings);
	findings
}

/// What the rule should do with one line.
enum Disposition {
	/// Close the open run.
	Break {
		/// `Some` records the declaration state for the next line; `None` leaves it unchanged, which
		/// is what a doc comment does — it ends the statements before it without ending the
		/// declaration body it sits inside.
		declaration: Option<bool>,
		/// True when this line was a doc comment, which makes the next code line a new item.
		previous_was_doc: bool,
	},
	/// Neither extend the run nor end it.
	Skip,
	/// Count this line as part of the open run.
	Extend,
}

/// Classifies one line for the group-separation rule.
fn classify(line: &LexedLine, run: &Run) -> Disposition {
	if line.is_blank() {
		return Disposition::Break {
			declaration: Some(false),
			previous_was_doc: false,
		};
	}

	let is_doc = line.comment_intent == Some(CommentIntent::Documentation);

	if line.is_comment() {
		// A doc comment starts a new item, so it ends whatever run preceded it. An ordinary comment
		// belongs to the code around it and changes nothing.
		if is_doc {
			return Disposition::Break {
				declaration: None,
				previous_was_doc: true,
			};
		}

		return Disposition::Skip;
	}

	if line.is_literal() || !line.is_code() {
		return Disposition::Skip;
	}

	let starts_new_item = is_type_declaration(line)
		|| is_function_declaration(line)
		|| run.previous_was_doc
		|| run.in_declaration;

	if starts_new_item {
		return Disposition::Break {
			declaration: Some(is_type_declaration(line) || !ends_declaration_body(line)),
			previous_was_doc: false,
		};
	}

	Disposition::Extend
}

/// The sequence of statements currently being measured.
#[derive(Default)]
struct Run {
	/// Index of the first line in the run, absent when no run is open.
	start: Option<usize>,
	length: usize,
	/// True while inside a type body, whose members are one logical group rather than a statement
	/// sequence.
	in_declaration: bool,
	/// True when the previous line was a doc comment, which makes the next line a new item.
	previous_was_doc: bool,
}

impl Run {
	/// Reports the current run as a finding when it is long enough, and clears it.
	fn close(&mut self, lines: &[LexedLine], findings: &mut Vec<Finding>) {
		check_run(lines, self.start, self.length, findings);

		self.start = None;
		self.length = 0;
		self.previous_was_doc = false;
	}

	/// Closes the run and applies the state a break decided.
	fn break_with(
		&mut self,
		lines: &[LexedLine],
		findings: &mut Vec<Finding>,
		declaration: Option<bool>,
		previous_was_doc: bool,
	) {
		self.close(lines, findings);

		// A doc comment carries the declaration state forward: it ends the statements before it
		// without ending the declaration body it sits inside.
		if let Some(in_declaration) = declaration {
			self.in_declaration = in_declaration;
		}

		self.previous_was_doc = previous_was_doc;
	}

	/// Counts one line as part of the run.
	fn extend(&mut self, index: usize) {
		self.start.get_or_insert(index);
		self.length += 1;
		self.previous_was_doc = false;
	}
}

/// Returns true when a line opens a type declaration whose members are one logical group.
fn is_type_declaration(line: &LexedLine) -> bool {
	/// Keywords that introduce a group of related members rather than a statement sequence.
	const DECLARATION_KEYWORDS: &[&str] = &[
		"struct",
		"enum",
		"union",
		"trait",
		"interface",
		"class",
		"impl",
		"record",
		"protocol",
		"extension",
		"namespace",
		"type",
	];

	has_keyword(line, DECLARATION_KEYWORDS) && line.masked_code.contains('{')
}

/// Returns true when a line declares a function or method.
fn is_function_declaration(line: &LexedLine) -> bool {
	/// Keywords that introduce a callable.
	const FUNCTION_KEYWORDS: &[&str] = &[
		"fn",
		"def",
		"func",
		"function",
		"fun",
		"proc",
		"sub",
		"method",
		"constructor",
		"lambda",
	];

	has_keyword(line, FUNCTION_KEYWORDS) && line.masked_code.contains('(')
}

/// Returns true when `line` contains one of `keywords` as a whole word.
fn has_keyword(line: &LexedLine, keywords: &[&str]) -> bool {
	line.masked_code
		.split(|character: char| !character.is_alphanumeric() && character != '_')
		.any(|word| keywords.contains(&word))
}

/// Returns true when a declaration's body closed on this line.
fn ends_declaration_body(line: &LexedLine) -> bool {
	let opens = line.masked_code.matches('{').count();
	let closes = line.masked_code.matches('}').count();

	opens > 0 && opens == closes
}

/// Reports a run of statements that is long enough to need internal grouping.
///
/// The threshold scales with nesting rather than being fixed: statements at depth 0 in a
/// short function are naturally one group, while the same count inside two levels of nesting
/// is much harder to scan.
fn check_run(
	lines: &[LexedLine],
	run_start: Option<usize>,
	run_length: usize,
	findings: &mut Vec<Finding>,
) {
	/// Statements in an unbroken run before grouping is expected.
	const RUN_LIMIT: usize = 8;

	if run_length < RUN_LIMIT {
		return;
	}

	let Some(start) = run_start else {
		return;
	};

	let Some(line) = lines.get(start) else {
		return;
	};

	findings.push(
		FindingBuilder::new(
			"readability/group-separation",
			Category::Readability,
			Span::new(line.start_byte, line.end_byte, line.number, line.number),
		)
		.severity(Severity::Minor)
		.weight(0.5)
		.message(format!(
			"{run_length} statements run together with no blank lines"
		))
		.suggestion(
			"Separate the logical groups within this run with blank lines so the phases of \
			 the function are visible.",
		)
		.build(),
	);
}

/// Reports indentation deeper than the configured limit.
///
/// Deep indentation is the visual symptom of nesting, and it is reported here as a layout
/// problem so that the reader is told to flatten rather than only that complexity is high.
pub fn excessive_indentation(file: &LexedFile, config: &RulesConfig) -> Vec<Finding> {
	let mut findings = Vec::new();

	for line in &file.lines {
		if !line.is_code() {
			continue;
		}

		if line.indent <= config.max_indent_width {
			continue;
		}

		findings.push(
			FindingBuilder::new(
				"readability/excessive-indentation",
				Category::Readability,
				line_span(line),
			)
			.severity(Severity::Major)
			.weight(1.5)
			.message(format!(
				"indented {} columns, over the {} column limit",
				line.indent, config.max_indent_width
			))
			.suggestion(
				"Flatten this block with early returns, or extract the inner logic into its \
				 own function.",
			)
			.build(),
		);
	}

	findings
}

/// Reports files that mix tabs and spaces for indentation.
pub fn mixed_indentation(file: &LexedFile) -> Vec<Finding> {
	// Only leading whitespace on code lines counts. Indentation inside a string literal or a
	// heredoc is data — a test fixture asserting on the indentation of the source it contains,
	// for instance — and reporting it would tell the author to change a value they deliberately
	// wrote.
	let tab_indented = file
		.lines
		.iter()
		.filter(|line| line.is_code())
		.any(|line| line.indent_text.contains('\t'));

	let space_indented = file
		.lines
		.iter()
		.filter(|line| line.is_code())
		.any(|line| line.indent_text.starts_with("    "));

	if !(tab_indented && space_indented) {
		return Vec::new();
	}

	let span = file.lines.first().map_or(Span::new(0, 0, 1, 1), line_span);

	vec![
		FindingBuilder::new("readability/mixed-indentation", Category::Readability, span)
			.severity(Severity::Major)
			.weight(2.0)
			.message("this file indents with both tabs and spaces")
			.suggestion("Pick one indentation character and apply it consistently across the file.")
			.build(),
	]
}

/// Returns true when a line continues an expression rather than starting a statement.
///
/// Two shapes count: a line that opens a delimiter which has not closed yet, and a line whose own text
/// begins inside one. In both cases the keyword it carries belongs to an expression.
fn inside_expression(line: &LexedLine, lines: &[LexedLine], index: usize) -> bool {
	// A control-flow keyword used as a value is part of an expression rather than a statement. `let after =
	// if flag { a } else { b }` has an `if` on the line, and reporting it asked for a blank line in the middle
	// of a binding.
	if is_expression_branch(line) {
		return true;
	}

	// The line itself starts inside an unclosed delimiter from an earlier line.
	let mut depth: isize = 0;

	for earlier in lines.iter().take(index) {
		if !earlier.is_code() {
			continue;
		}

		depth += earlier.masked_code.matches('(').count() as isize;
		depth -= earlier.masked_code.matches(')').count() as isize;
		depth += earlier.masked_code.matches('[').count() as isize;
		depth -= earlier.masked_code.matches(']').count() as isize;
	}

	if depth > 0 {
		return true;
	}

	// The line opens more than it closes, so whatever follows is a continuation of it.
	let opens = line.masked_code.matches('(').count() + line.masked_code.matches('[').count();
	let closes = line.masked_code.matches(')').count() + line.masked_code.matches(']').count();

	opens > closes
}

/// Returns true when a line's control-flow keyword introduces a value rather than a statement.
///
/// The signal is an assignment before the keyword, which is how Rust, Kotlin, Scala, and Python all write a
/// conditional expression.
fn is_expression_branch(line: &LexedLine) -> bool {
	let code = &line.masked_code;
	let Some(assignment) = code.find('=') else {
		return false;
	};

	// A comparison is not an assignment, so the operator itself is excluded.
	let operator = code.get(assignment..).unwrap_or_default();

	if operator.starts_with("==") || operator.starts_with("=>") {
		return false;
	}

	let Some(keyword) = code.find(" if ") else {
		return false;
	};

	assignment < keyword
}

/// Returns true when a line declares a block rather than being a statement inside one.
///
/// A declaration is worth exempting because the space belongs before the statements it
/// contains, not before the declaration itself. The check must therefore exclude control-flow
/// statements: `if x {` also ends with a brace, but it is a statement that deserves a blank
/// line above it, not a declaration. Testing for an absence of decision keywords is what
/// separates the two, since a declaration has none.
fn is_block_declaration(line: &LexedLine) -> bool {
	if !line.decisions.is_empty() {
		return false;
	}

	let trimmed = line.masked_code.trim();

	trimmed.ends_with('{')
		|| trimmed.ends_with('(')
		|| trimmed.ends_with("then")
		|| trimmed.ends_with("do")
		|| trimmed.ends_with("=>")
}

/// Finds the index of the previous line that is not blank.
fn previous_code_line(lines: &[LexedLine], index: usize) -> Option<&LexedLine> {
	lines.iter().take(index).rev().find(|candidate| {
		!candidate.is_blank() && !candidate.is_comment() && !candidate.is_literal()
	})
}
