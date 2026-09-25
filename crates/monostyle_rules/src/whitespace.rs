//! Whitespace rules.
//!
//! These rules encode a specific aesthetic: complex code should be given room, because the eye needs
//! somewhere to rest. They are the direct, checkable form of the guidance that blank lines go before
//! control flow, between logical groups, and before returns.
//!
//! Each rule reports where the space is missing and why it matters, so a finding is actionable
//! without consulting documentation.
//!
//! # Why only these rules are auto-fixable
//!
//! Every fix here is a blank line added or removed — an edit Rustfmt, Prettier, Black, and `dart format`
//! all preserve, and none of them perform, so the fixes cannot fight the project's own tooling.
//!
//! Together they are what makes the rest of the module safe to follow automatically: the rules ask for
//! gaps before branches and returns, after blocks, and between groups, and the ceiling rule bounds how
//! large any of those gaps may grow.
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
use monostyle_languages::BlockStyle;
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
	let depths = PrefixDepth::new(&file.lines);

	for (index, line) in file.lines.iter().enumerate() {
		if !needs_blank_line(&file.lines, &depths, index, line, config) {
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
	depths: &PrefixDepth,
	index: usize,
	line: &LexedLine,
	config: &RulesConfig,
) -> bool {
	if !line.is_code() || line.decisions.is_empty() {
		return false;
	}

	// An attribute is not a statement, so a keyword inside one is a name rather than a decision.
	// `#[serde(default, rename_all = "kebab-case")]` was reported as a `default` branch missing its
	// blank line, which asked for a blank line inside an attribute list.
	if is_attribute(line) {
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
	if inside_expression(line, depths, index) {
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

	// Two single-line control-flow statements in a row are a tight guard sequence:
	// `if (a > b) return 1;` followed by `if (c < d) return 0;` reads better without
	// a blank between them, the same way a chain of early returns does. A multi-line
	// block (one that opens a brace) still asks for the blank, because its body is a
	// group rather than a line.
	if is_single_line_control_flow(previous) {
		return false;
	}

	// The first statement inside a block has nothing above it to separate from.
	!(previous.opens_block() || is_block_declaration(previous))
}

/// Returns true when a line is a control-flow statement whose whole body sits on the same line.
///
/// `if (a > b) return 1;` and `if ready { go(); }` are single-line: their braces balance and nothing
/// opens onto the lines below. The shape matters because padding between two of them chops a tight
/// guard sequence into pieces — the same reasoning that exempts a chain of early returns.
fn is_single_line_control_flow(line: &LexedLine) -> bool {
	if line.decisions.is_empty() {
		return false;
	}

	if line.opens_block() {
		return false;
	}

	let opens = line.masked_code.matches('{').count();
	let closes = line.masked_code.matches('}').count();

	opens == closes
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

/// Reports a control-flow block whose closing brace is followed straight by another statement.
///
/// Padding is symmetric: the space before a branch announces that a decision is starting, and the space
/// after its block announces that it finished. A `}` crowded against the next `let` reads as though the
/// branch were still open.
///
/// The rule stays quiet where another rule already owns the blank, so a single missing line is never
/// reported twice: the next line being control flow belongs to [`blank_line_before_control_flow`], and
/// the next line being a `return` belongs to [`blank_line_before_return`]. Closing punctuation — the
/// brace that ends the enclosing function, a wrapped expression's final paren — is not a statement, so
/// the end of a body asks for nothing.
///
/// [`blank_line_before_control_flow`]: fn@crate::whitespace::blank_line_before_control_flow
/// [`blank_line_before_return`]: fn@crate::whitespace::blank_line_before_return
pub fn blank_line_after_control_flow(file: &LexedFile, config: &RulesConfig) -> Vec<Finding> {
	if !config.require_blank_line_after_control_flow {
		return Vec::new();
	}

	let mut findings = Vec::new();

	// The depth each decision-opened body lives at, with the line that opened it, innermost last. Only
	// bodies opened by a decision are tracked: a function or type body ending asks for nothing, because
	// the space between two items is a different question from the space around a branch.
	let mut opened: Vec<(usize, usize)> = Vec::new();
	let mut depth: usize = 0;

	for (index, line) in file.lines.iter().enumerate() {
		if !line.is_code() {
			continue;
		}

		let code = line.masked_code.trim_start();
		let opens = line.masked_code.matches('{').count();
		let closes = line.masked_code.matches('}').count();
		let depth_before = depth;

		depth = depth_before.saturating_add(opens).saturating_sub(closes);

		// A body whose braces balance on the opener's own line — `if ready { work(); }` — never opened
		// a region, so it can never close one either.
		let spanned = depth > depth_before;

		if spanned && !line.decisions.is_empty() {
			opened.push((depth_before, index));
		}

		// A line that both closes and reopens — `} else {` — continues the same decision, so the chain
		// is reported once, at the brace that ends its last branch.
		if code.starts_with('}') && !code.ends_with('{') {
			let mut ended_decision = false;

			// A body lives one depth below its opener, so it has closed once the depth drops back to
			// the level the decision was written at.
			opened.retain(|(body_depth, opened_at)| {
				let keep = *body_depth < depth;

				if !keep && index > *opened_at {
					ended_decision = true;
				}

				keep
			});

			if ended_decision && let Some(finding) = missing_line_after(file, index) {
				findings.push(finding);
			}
		}
	}

	findings
}

/// Builds the finding for a statement crowded against the control-flow block above it.
///
/// A comment between the brace and the statement belongs to the statement — it describes what comes
/// next — so the blank goes above the comment rather than between the two, and the fix is anchored
/// there.
fn missing_line_after(file: &LexedFile, closer: usize) -> Option<Finding> {
	let next = file.lines.get(closer.checked_add(1)?)?;

	// A blank line is already the padding the rule asks for.
	if next.is_blank() {
		return None;
	}

	// A comment belongs to the code below it, so the blank goes above the comment rather than between
	// the comment and the statement it describes.
	if next.is_comment() {
		return Some(missing_line_finding(next));
	}

	// Closing punctuation: the brace that ends the enclosing body, or the final delimiter of a wrapped
	// expression. The end of a scope is not a statement and asks for nothing. A literal line is data,
	// not the start of a statement either.
	if !next.is_code() {
		return None;
	}

	if next.masked_code.trim_start().starts_with('}') || !next.starts_statement() {
		return None;
	}

	// Control flow and returns are owned by their own rules, which report the same missing blank with
	// their own explanation. The lexer's decision table is what the before-rule fires on, so consulting
	// the same table here is what keeps one missing line from being counted twice.
	if !next.decisions.is_empty() || next.is_return {
		return None;
	}

	Some(missing_line_finding(next))
}

/// Builds the finding for one line missing the blank above it.
fn missing_line_finding(next: &LexedLine) -> Finding {
	let anchor = Span::new(next.start_byte, next.start_byte, next.number, next.number);
	let fix = Fix::insert(anchor, "\n", "insert a blank line after the block above");

	FindingBuilder::new(
		"readability/blank-line-after-control-flow",
		Category::Readability,
		line_span(next),
	)
	.severity(Severity::Minor)
	.weight(1.0)
	.message("this statement follows a control-flow block with no blank line between them")
	.suggestion(
		"Add a blank line so the branch or loop reads as finished rather than running into the \
		 statement that follows it.",
	)
	.fix(fix)
	.build()
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
	let mut bodies = Bodies::default();

	for (index, line) in file.lines.iter().enumerate() {
		// The region is read before the line is recorded, so an arm line is judged
		// as the arm it is rather than as the first statement of its own body.
		let in_alternatives = bodies.region() == Region::Alternatives;

		if !line.is_code() || !line.is_return {
			bodies.visit(file, line);
			continue;
		}

		// An early-return guard *is* the pattern this style prefers, so it is never reported.
		// Reporting it would penalize exactly the structure the guide recommends.
		if is_guard_clause(line) {
			bodies.visit(file, line);
			continue;
		}

		if has_separation_above(&file.lines, index, 1) {
			bodies.visit(file, line);
			continue;
		}

		let Some(previous) = previous_code_line(&file.lines, index) else {
			bodies.visit(file, line);
			continue;
		};

		// A return as the first statement of its block is idiomatic and needs no preamble.
		if previous.opens_block() {
			bodies.visit(file, line);
			continue;
		}

		// A return directly after another return is a sequence of guards, which reads fine.
		if previous.is_return {
			bodies.visit(file, line);
			continue;
		}

		// A return inside a match or switch arm is an alternative, not a sequential exit:
		// the arms are cases of one decision, and padding between them is the formatter's
		// call. `Err(e) => return e,` is the case that matters — the arm is a single line,
		// and inserting a blank between it and the arm above it chops the match into
		// pieces the way no formatter would.
		if in_alternatives {
			bodies.visit(file, line);
			continue;
		}

		let anchor = Span::new(line.start_byte, line.start_byte, line.number, line.number);
		let fix = Fix::insert(anchor, "\n", "insert a blank line above the return");

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
					at a glance.",
			)
			.fix(fix)
			.build(),
		);

		bodies.visit(file, line);
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
///
/// A function's *body*, by contrast, is exactly the sequence this rule measures, so the
/// statements inside one are counted, while a struct literal's fields are data rather than
/// statements and stay out of the count.
pub fn group_separation(file: &LexedFile, config: &RulesConfig) -> Vec<Finding> {
	if !config.require_group_separation {
		return Vec::new();
	}

	let mut findings = Vec::new();
	let mut run = Run::default();
	let mut bodies = Bodies::default();
	let depths = PrefixDepth::new(&file.lines);

	for (index, line) in file.lines.iter().enumerate() {
		// The regions a line sits in are read before the line is recorded, so a declaration is
		// judged as the declaration it is rather than as the first member of its own body.
		let region = bodies.region();

		match classify(file, &depths, index, line, &run, region) {
			// A blank line, a doc comment, or the start of a new item closes whatever run was open.
			Disposition::Break { previous_was_doc } => {
				run.break_with(file, config, &mut findings, previous_was_doc);
			}
			// A comment, a label, or a line inside a literal is invisible to the rule: it neither
			// extends a run nor ends one, except for a doc comment, which `Break` handles above.
			Disposition::Skip => {}
			Disposition::Extend => run.extend(index),
		}

		bodies.visit(file, line);
	}

	run.close(file, config, &mut findings);
	findings
}

/// What the rule should do with one line.
enum Disposition {
	/// Close the open run.
	Break {
		/// True when this line was a doc comment, which makes the next code line a new item.
		previous_was_doc: bool,
	},
	/// Neither extend the run nor end it.
	Skip,
	/// Count this line as part of the open run.
	Extend,
}

/// Classifies one line for the group-separation rule.
///
/// The order of the guards is the whole logic. Each one answers a question that would make the later
/// questions meaningless, so the checks are deliberately in a fixed order: what kind of line it is,
/// where it sits, and only then whether it breaks the run or joins it.
fn classify(
	file: &LexedFile,
	depths: &PrefixDepth,
	index: usize,
	line: &LexedLine,
	run: &Run,
	region: Region,
) -> Disposition {
	// A blank line is the boundary the rule asks for, so it always ends the run.
	if line.is_blank() {
		return Disposition::Break {
			previous_was_doc: false,
		};
	}

	if !ignorable(file, depths, index, line, region) {
		if starts_new_item(file, line, run, region) {
			return Disposition::Break {
				previous_was_doc: false,
			};
		}

		return Disposition::Extend;
	}

	// The only ignorable line that still ends a run is a doc comment: it introduces a distinct item, so
	// the statements before it are one group and the ones after it another.
	if line.comment_intent == Some(CommentIntent::Documentation) {
		return Disposition::Break {
			previous_was_doc: true,
		};
	}

	Disposition::Skip
}

/// Returns true when a line is invisible to the rule — neither a statement nor a break.
///
/// Every case here is a line the rule has no opinion about: one that is not code, one that carries no
/// statement of its own, or one whose text belongs to an expression or a literal rather than to the
/// sequence being measured. Blank lines are not here: they are the boundary the rule asks for.
fn ignorable(
	file: &LexedFile,
	depths: &PrefixDepth,
	index: usize,
	line: &LexedLine,
	region: Region,
) -> bool {
	if line.is_comment() {
		return true;
	}

	if line.is_literal() || !line.is_code() {
		return true;
	}

	// The fields of a struct literal, the entries of a collection, and the arguments of a wrapped call
	// are data rather than statements, and the arms of a `match` or `switch` are cases rather than steps.
	if region == Region::Data || region == Region::Alternatives {
		return true;
	}

	// A lone `}` or `});`, or the `end` that closes a block in Ruby and Lua, ends a block rather than
	// stating anything. Counting it would make every function body read one statement longer than it
	// is, and the closing marker is the most common line in the language.
	if closes_block(file, line) {
		return true;
	}

	// A formatter may spread one statement over many physical lines. Only its first line extends the
	// run; arguments, collection entries, and closing delimiters remain part of that statement.
	starts_inside_expression(depths, index) || !line.starts_statement()
}

/// Returns true when a line introduces an item rather than joining the current run.
///
/// An item is its own group: the members of a type, a documented declaration, a function's signature,
/// and the first statement after one all begin something rather than continue.
fn starts_new_item(file: &LexedFile, line: &LexedLine, run: &Run, region: Region) -> bool {
	let style = file.profile.block_style;

	if is_type_declaration(line, style) || is_function_declaration(line, style) {
		return true;
	}

	declares_body(file, line)
		|| run.previous_was_doc
		|| introduces_alternative(line)
		|| region == Region::Items
}

/// Returns true when a line introduces one alternative of a choice.
///
/// `case 0:`, `default:`, and `_ =>` each begin a new arm. They are labels rather than statements, so a
/// run never accumulates across them: counting them reported a `switch` of three braced arms as nine
/// statements run together, because each label and each body statement was added to the same run.
fn introduces_alternative(line: &LexedLine) -> bool {
	/// Keywords that label one alternative.
	const ARM_KEYWORDS: &[&str] = &["case", "default", "when"];

	let trimmed = line.masked_code.trim_start();

	// A Rust or Dart arm: `Some(value) =>`, `_ =>`, `1 =>`.
	if trimmed.ends_with("=>") {
		return true;
	}

	// A C-family arm: `case 0:`, `default:`, optionally followed by a brace.
	let head = trimmed.split('{').next().unwrap_or(trimmed);

	has_keyword(line, ARM_KEYWORDS) && (trimmed.ends_with(':') || head.trim_end().ends_with(':'))
}

/// Where a line sits, which decides whether it counts as a statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Region {
	/// Top level, or inside a function body: these are the statements the rule measures.
	Statements,
	/// Inside a type body, whose members are items to read one at a time.
	Items,
	/// Inside a `match` or `switch` body, whose arms are cases rather than steps.
	Alternatives,
	/// Inside a literal: a struct literal, a collection, a wrapped argument list.
	Data,
}

/// The sequence of statements currently being measured.
#[derive(Default)]
struct Run {
	/// Index of the first line in the run, absent when no run is open.
	start: Option<usize>,
	length: usize,
	/// True when the previous line was a doc comment, which makes the next line a new item.
	previous_was_doc: bool,
}

impl Run {
	/// Reports the current run as a finding when it is long enough, and clears it.
	fn close(&mut self, file: &LexedFile, config: &RulesConfig, findings: &mut Vec<Finding>) {
		check_run(
			&file.lines,
			self.start,
			self.length,
			config.max_statements_per_group,
			findings,
		);

		self.start = None;
		self.length = 0;
		self.previous_was_doc = false;
	}

	/// Closes the run and applies the state a break decided.
	fn break_with(
		&mut self,
		file: &LexedFile,
		config: &RulesConfig,
		findings: &mut Vec<Finding>,
		previous_was_doc: bool,
	) {
		self.close(file, config, findings);
		self.previous_was_doc = previous_was_doc;
	}

	/// Counts one line as part of the run.
	fn extend(&mut self, index: usize) {
		self.start.get_or_insert(index);
		self.length += 1;
		self.previous_was_doc = false;
	}
}

/// The body of a declaration, described by what its members are.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BodyKind {
	/// Members are items rather than statements, as in a struct, enum, trait, or class.
	Items,
	/// The body holds a statement sequence, as in a function or a control-flow block.
	Statements,
	/// The body holds alternatives, as in a `match` or `switch` body.
	///
	/// Only one arm runs, so the arms are a set of cases rather than a sequence of steps. Counting them as
	/// statements reported a `switch` with nine patterns as nine statements run together, which is the
	/// shape a total function over an enum always has. Blank lines between arms are a matter of taste, and
	/// a formatter decides them.
	Alternatives,
	/// The body holds data rather than code, as in a struct literal or a collection.
	Data,
}

/// A body that is currently open.
#[derive(Debug, Clone, Copy)]
struct OpenBody {
	/// The depth the body was opened at: its members sit deeper than this.
	open_depth: usize,
	/// What the body holds.
	kind: BodyKind,
}

/// Tracks the bodies and literals that are open, so the rule knows what a line is part of.
///
/// The rule measures statement runs, and what counts as a statement depends on where the line sits.
/// A type's members are items rather than statements — a struct with twelve fields is one item, not
/// twelve lines run together — while a function's body is exactly the sequence this rule is looking
/// for. A literal's contents are neither: they are data.
///
/// Keeping this as a stack, rather than the single flag the rule used to carry, is what makes a
/// nested declaration correct: a struct declared inside a function body ends when its own brace
/// closes, and the statements after it belong to the function again. That flag was never cleared,
/// which made the rule blind inside every function body — the one place it matters most — and made
/// splitting a long run unable to clear the finding, because the split only closed the run that the
/// flag had already prevented from forming.
#[derive(Default)]
struct Bodies {
	/// The bodies that are still open, innermost last.
	open: Vec<OpenBody>,
	/// Brace depth in brace languages, `end` nesting in `end`-terminated ones.
	depth: usize,
	/// Unmatched delimiters opened by a line that is not a block, which is how a literal is tracked.
	data: usize,
}

impl Bodies {
	/// The region the next line sits in.
	fn region(&self) -> Region {
		if self.data > 0 {
			return Region::Data;
		}

		match self.open.last().map(|body| body.kind) {
			Some(BodyKind::Items) => Region::Items,
			Some(BodyKind::Alternatives) => Region::Alternatives,
			Some(BodyKind::Data) => Region::Data,
			// A function body and a control-flow block both hold statements, and so does the top
			// level of a file.
			Some(BodyKind::Statements) | None => Region::Statements,
		}
	}

	/// Records one line, opening and closing bodies as it does.
	fn visit(&mut self, file: &LexedFile, line: &LexedLine) {
		if !line.is_code() {
			return;
		}

		match file.profile.block_style {
			BlockStyle::Brace => self.visit_brace(file, line),
			BlockStyle::Indentation => self.visit_indentation(file, line),
			BlockStyle::EndKeyword => self.visit_end_keyword(file, line),
		}

		self.data = self.data_after(file, line);
	}

	/// Records a line in a brace-delimited language.
	fn visit_brace(&mut self, file: &LexedFile, line: &LexedLine) {
		let opens = line.masked_code.matches('{').count();
		let closes = line.masked_code.matches('}').count();
		let depth_before = self.depth;

		self.depth = depth_before.saturating_add(opens).saturating_sub(closes);

		// A body closes once the depth returns to the level it was declared at.
		self.open.retain(|body| body.open_depth < self.depth);

		// A declaration whose brace is still open after this line opens a body. One that opens and
		// closes on its own line — `struct Point { x: f64 }` — is a whole item in one line.
		if self.depth > depth_before
			&& let Some(kind) = body_kind(file, line)
		{
			self.open.push(OpenBody {
				open_depth: depth_before,
				kind: self.inherited(kind),
			});
		}
	}

	/// Records a line in an indentation-delimited language.
	///
	/// A body runs until a line returns to the indentation of the declaration that opened it, which is
	/// the same rule the scanner and the unit detector use.
	fn visit_indentation(&mut self, file: &LexedFile, line: &LexedLine) {
		self.open.retain(|body| line.indent > body.open_depth);

		if let Some(kind) = body_kind(file, line) {
			self.open.push(OpenBody {
				open_depth: line.indent,
				kind: self.inherited(kind),
			});
		}
	}

	/// Records a line in an `end`-terminated language.
	///
	/// The nesting is counted here rather than inferred from the text, because a closing `end` carries
	/// no keyword and a nested body has to know which opener it belongs to.
	fn visit_end_keyword(&mut self, file: &LexedFile, line: &LexedLine) {
		let first = line
			.masked_code
			.split_whitespace()
			.next()
			.unwrap_or_default();

		if file.profile.end_keywords.contains(&first) {
			self.depth = self.depth.saturating_sub(1);
		}

		self.open.retain(|body| body.open_depth < self.depth);

		if let Some(kind) = body_kind(file, line) {
			self.open.push(OpenBody {
				open_depth: self.depth,
				kind: self.inherited(kind),
			});

			self.depth += 1;
		}
	}

	/// The kind a body takes given where it sits.
	///
	/// A block opened inside a `match` or `switch` is part of one alternative, so it inherits that: the
	/// statements in one arm are that arm's body, not a sequence the reader has to scan in order. Without
	/// this a `switch` with braced arms accumulated every arm label and every arm body into one run.
	fn inherited(&self, kind: BodyKind) -> BodyKind {
		if self.region() == Region::Alternatives {
			return BodyKind::Alternatives;
		}

		kind
	}

	/// The data-delimiter depth after `line`.
	///
	/// Parens and brackets always belong to an expression, so a line inside one continues the
	/// statement above it. Braces are handled by the body stack in brace languages, where they may open
	/// a block; in the others a brace can only delimit a literal, so it counts here.
	fn data_after(&self, file: &LexedFile, line: &LexedLine) -> usize {
		let code = &line.masked_code;
		let mut balance = code.matches('(').count() as isize - code.matches(')').count() as isize;
		balance += code.matches('[').count() as isize - code.matches(']').count() as isize;

		if file.profile.block_style != BlockStyle::Brace {
			balance += code.matches('{').count() as isize;
			balance -= code.matches('}').count() as isize;
		}

		let total = self.data as isize + balance;

		// A stray closer — an unbalanced closing brace in a file the scanner already flagged as
		// unterminated — must not push the depth below zero, or every later line would look like it
		// sat inside a literal.
		total.max(0) as usize
	}
}

/// The body a line opens, if it opens one.
///
/// Four shapes open a body: a declaration, a control-flow statement, a trailing block or lambda, and a
/// literal. The first three hold code and the last holds data, and telling them apart is what keeps a
/// twelve-field struct literal out of the count while the statements of a function body stay in it.
fn body_kind(file: &LexedFile, line: &LexedLine) -> Option<BodyKind> {
	let style = file.profile.block_style;

	if !opens_body(line, style) {
		return None;
	}

	if is_type_declaration(line, style) {
		return Some(BodyKind::Items);
	}

	// A `match` or `switch` body holds alternatives rather than steps, so its arms are cases: only one
	// of them runs, and a total function over an enum necessarily lists one per variant.
	if opens_alternatives(line) {
		return Some(BodyKind::Alternatives);
	}

	if opens_statement_block(line) {
		return Some(BodyKind::Statements);
	}

	if is_function_declaration(line, style) || is_signature(line) {
		return Some(BodyKind::Statements);
	}

	// Only a brace language can open a literal body, because only there does a brace delimit a value.
	// In the others the brace belongs to the interpolation or is a shell expansion, and the body was
	// already recognized above or not at all.
	if style != BlockStyle::Brace {
		return None;
	}

	Some(literal_or_block(line))
}

/// Returns true when a line opens a block of statements rather than a list of items or alternatives.
///
/// A decision or nesting keyword is the plainest block there is — `if x {`, `while ready {` — followed
/// by the block markers that are not keywords: a `do` block, a `then` branch, an `=>` arm.
fn opens_statement_block(line: &LexedLine) -> bool {
	if !(line.decisions.is_empty() && line.nesting.is_empty()) {
		return true;
	}

	let trimmed = line.masked_code.trim_end();

	trimmed.ends_with("=>") || trimmed.ends_with("then") || trimmed.ends_with("do")
}

/// Classifies a brace that is neither a declaration nor a decision: it opens a statement block when the
/// brace begins a scope (`fn a() {`, a bare block) and a literal otherwise.
fn literal_or_block(line: &LexedLine) -> BodyKind {
	if brace_opens_a_block(line) {
		BodyKind::Statements
	} else {
		BodyKind::Data
	}
}

/// Returns true when a line opens a body whose members are alternatives rather than steps.
///
/// `match` and `switch` both introduce a set of cases, and a total function over an enum necessarily
/// lists one per variant. The word may land in either keyword list depending on the language profile —
/// Rust records `match` as a decision and C records `switch` as nesting — so both are consulted.
fn opens_alternatives(line: &LexedLine) -> bool {
	/// Keywords that introduce a choice between alternatives rather than a sequence.
	const ALTERNATIVE_KEYWORDS: &[&str] = &["match", "switch"];

	line.decisions
		.iter()
		.chain(line.nesting.iter())
		.any(|keyword| ALTERNATIVE_KEYWORDS.contains(&keyword.as_str()))
		|| has_keyword(line, ALTERNATIVE_KEYWORDS)
}

/// Returns true when a line's brace opens a block of code rather than a literal value.
///
/// The signal is what precedes the brace. A literal follows an assignment (`let p = Point {`) or sits
/// inside an expression (`take(Big {`), while a block follows a parameter list, a keyword, or a bare
/// type name.
fn brace_opens_a_block(line: &LexedLine) -> bool {
	let code = &line.masked_code;
	let Some(brace) = code.find('{') else {
		return false;
	};

	let prefix = code.get(..brace).unwrap_or_default().trim_end();

	// A brace with nothing before it is a bare scope, which holds statements.
	if prefix.is_empty() {
		return true;
	}

	if has_assignment(prefix) {
		return false;
	}

	// An unbalanced paren or bracket means the brace sits inside an expression, so it belongs to the
	// value being built rather than to a block.
	let parens = prefix.matches('(').count() as isize - prefix.matches(')').count() as isize;
	let brackets = prefix.matches('[').count() as isize - prefix.matches(']').count() as isize;

	if parens > 0 || brackets > 0 {
		return false;
	}

	// A prefix that ends in a value is a literal: `Some(Point {` closes a paren and a name, and the
	// brace is the value's own. A prefix that ends in a keyword or a type name introduces a block.
	let first = prefix
		.split(|character: char| !character.is_alphanumeric() && character != '_')
		.next()
		.unwrap_or_default();

	CONTROL_KEYWORDS.contains(&first) || prefix.ends_with(')') || is_bare_type_name(prefix)
}

/// Returns true when a prefix is a bare type name, which is how a literal is introduced.
///
/// `Point {` and `Self {` build a value, so the brace belongs to it. Anything with a keyword, an
/// operator, or a punctuation mark is not a type name and falls through to the block case.
fn is_bare_type_name(prefix: &str) -> bool {
	!prefix.is_empty()
		&& prefix
			.chars()
			.all(|character| character.is_alphanumeric() || character == '_')
}

/// Returns true when the prefix assigns a value, which makes a following brace a literal.
///
/// The comparison and arrow operators contain `=` without assigning, so each is excluded by name.
fn has_assignment(prefix: &str) -> bool {
	prefix.match_indices('=').any(|(index, _)| {
		let before = prefix.get(..index).unwrap_or_default();
		let after = prefix.get(index..).unwrap_or_default();

		!(after.starts_with("=>")
			|| after.starts_with("==")
			|| before.ends_with(['!', '<', '>', '=']))
	})
}

/// Returns true when a line is a signature — a parameter list with nothing after it.
///
/// This is how a function is declared without a declaration keyword: Dart's `void emit()`, a C
/// function definition, a Kotlin method. The keyword tables do not cover them, and the brace that
/// follows a closed parameter list is a body.
fn is_signature(line: &LexedLine) -> bool {
	let code = &line.masked_code;
	let end = code.find('{').unwrap_or(code.len());
	let head = code.get(..end).unwrap_or_default().trim_end();

	if !head.ends_with(')') {
		return false;
	}

	// `if x {`, `for entry in list {`, and `while running {` all end in a parameter list followed by
	// an opener, and all of them are statements rather than declarations. The decisions the lexer
	// already recorded are what separates them, and the fallback below covers a language whose
	// keyword table omits the word.
	if !line.decisions.is_empty() || !line.nesting.is_empty() {
		return false;
	}

	let first = head
		.split(|character: char| !character.is_alphanumeric() && character != '_')
		.next()
		.unwrap_or_default();

	if CONTROL_KEYWORDS.contains(&first) {
		return false;
	}

	// The parameter list must close the signature. This is what separates `void emit()` from
	// `register(handler);`, where the parens close a call rather than a declaration.
	head.matches(')').count() > head.matches('(').count().saturating_sub(1)
}

/// Words that introduce a statement rather than a named declaration.
///
/// Shared by the signature check and the block check, because both are asking the same question: does
/// this line name something, or does it do something?
const CONTROL_KEYWORDS: &[&str] = &[
	"if", "else", "elsif", "elif", "unless", "for", "while", "until", "loop", "switch", "match",
	"case", "when", "catch", "except", "rescue", "finally", "return", "yield", "await", "assert",
	"throw", "raise", "let", "const", "var", "do", "begin", "try", "with", "defer", "guard",
	"repeat",
];

/// Returns true when a line declares a body without a declaration keyword.
///
/// Dart's `void emit()`, a C function definition, and a Kotlin method all name a callable without any
/// word from the keyword tables, so the parameter list followed by an opener is the only signal. The
/// line is a declaration rather than a statement, which is why it ends a run instead of extending one:
/// the statements it introduces are measured as its body.
fn declares_body(file: &LexedFile, line: &LexedLine) -> bool {
	opens_body(line, file.profile.block_style) && is_signature(line)
}

/// Returns true when a line opens a body in this language's block style.
///
/// The marker differs by style: a brace language opens with `{`, and an indentation language with a
/// trailing `:`. An `end`-terminated language has no marker at all — `def work` is the whole opener —
/// so any line could be one, and the declaration and keyword checks that follow are what decide.
fn opens_body(line: &LexedLine, style: BlockStyle) -> bool {
	match style {
		BlockStyle::Brace => line.masked_code.contains('{'),
		BlockStyle::Indentation => line.masked_code.trim_end().ends_with(':'),
		BlockStyle::EndKeyword => true,
	}
}

/// Returns true when a line closes a block rather than stating anything.
///
/// The marker depends on how the language delimits blocks: a brace language closes with a brace, and an
/// `end`-terminated language with its closing keyword. Counting either as a statement would make every
/// function body read one statement longer than it is, and the closing marker is the most common line
/// in the language.
fn closes_block(file: &LexedFile, line: &LexedLine) -> bool {
	let trimmed = line.masked_code.trim();

	if file.profile.block_style == BlockStyle::EndKeyword {
		let first = trimmed.split_whitespace().next().unwrap_or_default();

		return file.profile.end_keywords.contains(&first);
	}

	trimmed
		.chars()
		.all(|character| matches!(character, '}' | ')' | ']' | ';' | ','))
}

/// Returns true when a line opens a type declaration whose members are one logical group.
fn is_type_declaration(line: &LexedLine, style: BlockStyle) -> bool {
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

	has_keyword(line, DECLARATION_KEYWORDS) && opens_body(line, style)
}

/// Returns true when a line declares a function or method.
fn is_function_declaration(line: &LexedLine, style: BlockStyle) -> bool {
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

	// A brace or indentation language writes a parameter list, which is what separates a declaration
	// from a call. Ruby and Lua do not require one — `def name` is the whole signature — so the
	// keyword alone is the signal there.
	let has_signature = line.masked_code.contains('(') || style == BlockStyle::EndKeyword;

	has_keyword(line, FUNCTION_KEYWORDS) && has_signature
}

/// Returns true when `line` contains one of `keywords` as a whole word.
fn has_keyword(line: &LexedLine, keywords: &[&str]) -> bool {
	line.masked_code
		.split(|character: char| !character.is_alphanumeric() && character != '_')
		.any(|word| keywords.contains(&word))
}

/// Reports a run of statements that is long enough to need internal grouping.
///
/// The limit is a statement count rather than a depth, because that is the thing the reader is being
/// asked to change: the message names both the run and the limit, so a run can be split until the
/// finding clears instead of being guessed at.
fn check_run(
	lines: &[LexedLine],
	run_start: Option<usize>,
	run_length: usize,
	limit: usize,
	findings: &mut Vec<Finding>,
) {
	if run_length <= limit {
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
			"{run_length} statements run together with no blank lines (limit {limit})"
		))
		.suggestion(format!(
			"Separate the logical groups within this run with blank lines so the phases of the \
			 function are visible. A blank line every {limit} statements or fewer clears this \
			 finding."
		))
		.build(),
	);
}

/// Reports a comment separated from the code it documents.
///
/// A comment above a line is read as describing that line, so a blank line between them does not make
/// the comment breathe — it orphans it. The comment belongs downward, always: padding a branch asks for
/// the blank *above* the comment, never between the comment and its code.
///
/// The same attachment governs chains: several comment blocks in a row are read as one description, so
/// a blank anywhere between the first block and the code it documents is reported, at the first blank.
///
/// # Why some separations are exempt
///
/// A block of `//!` at the top of a file is a header, not an attachment, and the blank after it is how
/// every file in this repository separates that header from its imports. The same is true of the first
/// comment block in a file, which is where license and copyright notices live. Neither is describing
/// the line below, so neither is held to it.
pub fn detached_comment(file: &LexedFile, config: &RulesConfig) -> Vec<Finding> {
	if !config.require_attached_comments {
		return Vec::new();
	}

	let mut findings = Vec::new();
	let mut index = 0;

	while let Some(block) = CommentBlock::starting_at(file, index) {
		if let Some(finding) = block.detachment(file) {
			findings.push(finding);
		}

		index = block.resumes_after();
	}

	findings
}

/// A comment block and the line it documents.
///
/// Walking forward from a block, comment lines are read as one description and blank lines are read as
/// the gap; the first code line after it all is what the description belongs to.
struct CommentBlock {
	/// Index of the block's first line.
	start: usize,
	/// Index of the first blank line past the block, when the block is separated from what follows.
	gap: Option<usize>,
	/// Index of the documented line.
	attached: usize,
}

impl CommentBlock {
	/// Finds the next comment block at or after `index`.
	///
	/// Returns `None` at end of file, including the case of a trailing comment block with no code after
	/// it — a comment that documents nothing below has nothing to detach from.
	fn starting_at(file: &LexedFile, index: usize) -> Option<Self> {
		let start = (index..file.lines.len())
			.find(|i| file.lines.get(*i).is_some_and(LexedLine::is_comment))?;

		let mut cursor = start;
		let mut gap = None;

		loop {
			let line = file.lines.get(cursor)?;

			if line.is_comment() {
				cursor += 1;
			} else if line.is_blank() {
				gap.get_or_insert(cursor);
				cursor += 1;
			} else {
				return Some(Self {
					start,
					gap,
					attached: cursor,
				});
			}
		}
	}

	/// The finding for a block separated from its code, if the separation counts.
	fn detachment(&self, file: &LexedFile) -> Option<Finding> {
		let gap = self.gap?;
		let blank_line = file.lines.get(gap)?;
		let attached = file.lines.get(self.attached)?;

		// A block at the very top of the file is a header when it is not an outer doc comment: `//!`
		// introduces the file, and a plain comment carries license or copyright text, so the blank after
		// either is formatting rather than a detachment. An outer doc block (`///`) at that position is
		// different — it is attached to the item below by definition, which is what makes a blank before
		// the item a mistake.
		let opener = file
			.lines
			.get(self.start)
			.map(|line| line.text.trim_start())
			.unwrap_or_default();
		let is_header =
			self.start == 0 && !(opener.starts_with("///") || opener.starts_with("/**"));

		if is_header {
			return None;
		}

		Some(
			FindingBuilder::new(
				"readability/detached-comment",
				Category::Readability,
				line_span(blank_line),
			)
			.severity(Severity::Minor)
			.weight(0.75)
			.message(format!(
				"this comment is separated from line {} by a blank line",
				attached.number
			))
			.suggestion(
				"A comment belongs to the code below it. Delete the blank line, or move the comment \
				 down if it describes something else.",
			)
			.fix(Fix::delete(
				Span::new(
					blank_line.start_byte,
					attached.start_byte,
					blank_line.number,
					attached.number,
				),
				"attach the comment to the line it documents",
			))
			.build(),
		)
	}

	/// The index the scan resumes at: the documented line, which cannot start another block.
	fn resumes_after(&self) -> usize {
		self.attached
	}
}

/// Reports runs of blank lines longer than the configured maximum.
///
/// Every other rule in this module asks for a gap: a blank line before a branch, before a return,
/// between statement groups. A set of rules that only ever adds whitespace has no way to say when
/// there is too much, so following all of them at once can grow a gap without limit — five blank
/// lines between two `match` arms satisfy every rule that asked for one, and the result is harder to
/// read than the crowded version it replaced.
///
/// This is the ceiling that makes the rest safe to follow.
///
/// # Why the limit is per language
///
/// The number is not a universal constant. PEP 8 asks for two blank lines before a top-level Python
/// definition, and Go's `gofmt` and the Dart formatter each have their own convention, so a limit of
/// one would report the standard style of three of the languages this tool supports. The allowance is
/// therefore the larger of the configured maximum and the language's own convention, and only a run
/// longer than that is reported. Two consecutive blank lines in a Rust file are a gap; in a Python
/// file between two top-level definitions they are the documented layout.
///
/// Trailing blank lines at the end of a file are not reported: they separate nothing, and the
/// position of the last line is a matter for the formatter rather than for this rule.
///
/// # Why this rule is auto-fixable
///
/// Deleting a blank line is an edit every formatter leaves alone, and the number to keep is already
/// decided — it is the same allowance the finding measured against, so the fix cannot disagree with the
/// rule. Applying it is therefore idempotent: the second run finds a run of exactly the allowance and
/// reports nothing.
///
/// This is also what makes the rest of the module safe to follow automatically. The other whitespace
/// rules ask for gaps without ever bounding them, so a fixer that only inserts blank lines can grow a
/// gap until the file is mostly whitespace; this fix is the counterweight that brings it back.
pub fn excessive_blank_lines(file: &LexedFile, config: &RulesConfig) -> Vec<Finding> {
	let allowed = config
		.max_consecutive_blank_lines
		.max(blank_line_convention(file.language));

	if allowed == 0 {
		return Vec::new();
	}

	let mut findings = Vec::new();
	let mut run: Vec<&LexedLine> = Vec::new();

	for line in &file.lines {
		if line.is_blank() {
			run.push(line);

			continue;
		}

		if let Some(finding) = blank_run_finding(&run, allowed) {
			findings.push(finding);
		}

		run.clear();
	}

	// A trailing run is left alone: it separates nothing, and a file that ends with several blank
	// lines is a formatting question rather than a readability one.
	findings
}

/// Builds the finding for a completed run of blank lines, if it is longer than `allowed`.
fn blank_run_finding(run: &[&LexedLine], allowed: usize) -> Option<Finding> {
	let length = run.len();

	if length <= allowed {
		return None;
	}

	let first = *run.first()?;
	let span = line_span(first);

	// The fix deletes every blank line past the allowance, from the end of the last kept line to the
	// end of the run. Anchoring it at the end rather than rewriting the whole run keeps the edit as
	// small as possible, so a diff review sees removed lines rather than replaced ones.
	let kept = *run.get(allowed.checked_sub(1)?)?;
	let excess = run.get(allowed..)?;
	let last = *excess.last()?;

	let delete_span = Span::new(
		kept.end_byte,
		last.end_byte,
		kept.number.saturating_add(1),
		last.number,
	);

	let fix = Fix::delete(delete_span, format!("remove {} blank lines", excess.len()));

	Some(
		FindingBuilder::new(
			"readability/excessive-blank-lines",
			Category::Readability,
			span,
		)
		.severity(Severity::Minor)
		.weight(1.0)
		.message(format!(
			"{length} consecutive blank lines, over the limit of {allowed}"
		))
		.suggestion(format!(
			"Keep {allowed} blank line{} here. More than that reads as a gap in the code rather \
			 than a break between groups.",
			if allowed == 1 { "" } else { "s" }
		))
		.fix(fix)
		.build(),
	)
}

/// The number of blank lines a language's own style asks for between top-level items.
///
/// Only languages with a documented convention appear here. A language that does not is governed by
/// the configured maximum alone, which keeps the rule from inventing a standard for a language whose
/// community never agreed on one.
///
/// The value is a floor rather than an override: a project that allows more than its language asks
/// for keeps that setting, because the configured maximum is what a project chose deliberately.
fn blank_line_convention(language: monostyle_core::Language) -> usize {
	/// PEP 8: "surround top-level function and class definitions with two blank lines".
	const PYTHON: usize = 2;
	/// `dart format` puts a blank line between declarations, and two around a class body's members.
	const DART: usize = 2;

	match language {
		monostyle_core::Language::Python => PYTHON,
		monostyle_core::Language::Dart => DART,
		_ => 1,
	}
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
fn inside_expression(line: &LexedLine, depths: &PrefixDepth, index: usize) -> bool {
	// A control-flow keyword used as a value is part of an expression rather than a statement. `let after =
	// if flag { a } else { b }` has an `if` on the line, and reporting it asked for a blank line in the middle
	// of a binding.
	if is_expression_branch(line) {
		return true;
	}

	if starts_inside_expression(depths, index) {
		return true;
	}

	// The line opens more than it closes, so whatever follows is a continuation of it.
	let opens = line.masked_code.matches('(').count() + line.masked_code.matches('[').count();
	let closes = line.masked_code.matches(')').count() + line.masked_code.matches(']').count();

	opens > closes
}

/// Returns true when a line begins inside parentheses or brackets opened above it.
///
/// This is a lookup into the file's [`PrefixDepth`] rather than a rescan. The rescan was quadratic — it
/// walked every earlier line for every line — and a twenty-thousand-line file took sixteen seconds to
/// analyze, with the cost growing fourfold when the file doubled.
fn starts_inside_expression(depths: &PrefixDepth, index: usize) -> bool {
	depths.starts_inside(index)
}

/// The delimiter depth at every line boundary, computed once per file.
///
/// "Is this line a continuation of an expression above it?" is asked for every line, and answering it by
/// walking the lines above is what made the rule set quadratic in file size. Building the table once
/// turns each question into a single lookup, and the answer is identical: the depth after the first
/// `index` lines is exactly what the rescan computed.
struct PrefixDepth {
	/// One entry per boundary: `boundaries[i]` is the depth after the first `i` lines.
	boundaries: Vec<isize>,
}

impl PrefixDepth {
	/// Builds the depth table for `lines`.
	fn new(lines: &[LexedLine]) -> Self {
		let mut boundaries = Vec::with_capacity(lines.len() + 1);
		let mut depth: isize = 0;

		boundaries.push(depth);

		for line in lines {
			// A line that is not code contributes nothing, which is what the rescan's `continue` did.
			if line.is_code() {
				depth += line.masked_code.matches('(').count() as isize;
				depth -= line.masked_code.matches(')').count() as isize;
				depth += line.masked_code.matches('[').count() as isize;
				depth -= line.masked_code.matches(']').count() as isize;
			}

			boundaries.push(depth);
		}

		Self { boundaries }
	}

	/// Whether the line at `index` begins inside an expression opened above it.
	fn starts_inside(&self, index: usize) -> bool {
		self.boundaries.get(index).copied().unwrap_or(0) > 0
	}
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

/// Whether the line is an attribute, decorator, or annotation rather than a statement.
///
/// These attach metadata to the declaration below them. A keyword inside one is an option name — a
/// serde `default`, an angular `if`, a Java `for` in an annotation — so treating it as control flow
/// asks for a blank line that would separate the attribute from the item it describes.
fn is_attribute(line: &LexedLine) -> bool {
	let trimmed = line.masked_code.trim_start();

	// Rust and Rust-like: `#[...]` and the inner `#![...]`. A decorator's `@` and a Java or C#
	// annotation's leading `@` are the same shape: metadata before a declaration.
	trimmed.starts_with("#[")
		|| trimmed.starts_with("#![")
		|| trimmed.starts_with('@')
		|| trimmed.starts_with("[[")
}

/// Finds the index of the previous line that is not blank.
fn previous_code_line(lines: &[LexedLine], index: usize) -> Option<&LexedLine> {
	lines.iter().take(index).rev().find(|candidate| {
		!candidate.is_blank() && !candidate.is_comment() && !candidate.is_literal()
	})
}
