//! Cognitive complexity.
//!
//! Cyclomatic complexity treats a nested `if` inside three loops as equal to a flat
//! `if`. That is wrong for readability, and it is exactly the complaint that motivated
//! `SonarSource`'s cognitive complexity.
//!
//! This implementation follows the [cognitive complexity specification] in three parts:
//!
//! 1. **Increment for flow breaks.** `if`, `else`, loops, `catch`, and switches add 1.
//! 2. **Increment for nesting.** Each of those costs an additional 1 per level of
//!    nesting it sits inside. `else` and `else if` are exempt from the nesting penalty,
//!    because chaining alternatives stays flat in the reader's mind.
//! 3. **Increment for structure.** Sequences of binary logical operators add 1 per
//!    sequence, not per operator, since a chain of `&&`s reads as one condition.
//!
//! The nesting level itself rises only where the specification says it does: when the body
//! of an `if`, a loop, a `match`/`switch`, or a `catch` opens, and for nested functions and
//! closures. A brace by itself is structure, not a decision — match arms, scoping blocks,
//! and the unit's own body would otherwise read as one level deeper than the reader
//! experiences them, which is what made the numbers here incomparable with other tools.
//!
//! Crucially, this metric *rewards what monostyle recommends*: extracting a nested
//! block into its own function resets nesting to zero, so the score improves even when
//! the total number of branches is unchanged. That makes cognitive complexity the right
//! backbone for the complexity score.
//!
//! [cognitive complexity specification]: https://www.sonarsource.com/docs/CognitiveComplexity.pdf

use monostyle_lexer::LexedFile;
use monostyle_lexer::LexedLine;
use serde::Serialize;

/// A cognitive complexity measurement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct CognitiveComplexity {
	/// The total score.
	pub total: usize,
	/// Points added purely by nesting.
	pub nesting_penalty: usize,
	/// The deepest nesting level observed.
	pub max_nesting: usize,
}

impl CognitiveComplexity {
	/// A qualitative band for a cognitive complexity value.
	///
	/// `SonarSource`'s default rule flags a function over 15, which is the `Complex`
	/// boundary used here.
	#[must_use]
	pub fn grade(&self) -> &'static str {
		match self.total {
			0..=4 => "simple",
			5..=9 => "moderate",
			10..=15 => "complex",
			16..=25 => "hard",
			_ => "very-hard",
		}
	}
}

/// Computes cognitive complexity for every line in `file`.
#[must_use]
pub fn cognitive_complexity(file: &LexedFile) -> CognitiveComplexity {
	cognitive_complexity_of_lines(&file.lines)
}

/// Computes cognitive complexity over a slice of lines.
#[must_use]
pub fn cognitive_complexity_of_lines(lines: &[LexedLine]) -> CognitiveComplexity {
	let mut total = 0;
	let mut nesting_penalty = 0;
	let mut nesting = NestingTracker::default();

	for line in lines {
		// The nesting level a construct sits at is the depth *before* this line opens anything,
		// which is why the cost is computed first and the depth updated after.
		let cost = line_cost(line, nesting.depth());
		total += cost.total;
		nesting_penalty += cost.nesting_penalty;

		nesting.advance(line);
	}

	CognitiveComplexity {
		total,
		nesting_penalty,
		max_nesting: nesting.max(),
	}
}

/// Tracks the nesting level across a unit's lines.
///
/// The level rises only where the specification says it does — when the body of an `if`, a
/// loop, a `match`/`switch`, or a `catch` opens, and for nested functions and closures — and
/// falls again when that body closes. Every open block is remembered, raised or not, so a
/// closing brace is consumed by the construct that owns it rather than by whichever level
/// happens to be innermost.
#[derive(Default)]
struct NestingTracker {
	/// The current nesting level.
	depth: usize,
	/// The deepest level reached so far.
	max: usize,
	/// For every open block, whether its opener raised the level. Braces that open a plain
	/// block — a match arm, a scoping brace, the unit's own body — are tracked too, so the
	/// `}` that closes them is consumed by the right opener instead of by a construct the
	/// reader never descended into.
	raised: Vec<bool>,
	/// Whether the unit's own body has opened. The signature line is where the unit begins,
	/// so its brace is depth 0: the specification counts the nesting inside a unit, not the
	/// unit itself.
	unit_body_opened: bool,
	/// Whether the most recent code raised a level whose body brace has not been seen yet.
	/// A construct may put its body on the next line — C#'s Allman style puts every `{`
	/// there — and that brace is the construct's body, not a plain block.
	pending_raise: bool,
}

impl NestingTracker {
	/// The current nesting level.
	const fn depth(&self) -> usize {
		self.depth
	}

	/// The deepest level reached so far.
	const fn max(&self) -> usize {
		self.max
	}

	/// Advances the tracking by one line.
	fn advance(&mut self, line: &LexedLine) {
		self.advance_braces(line);
		self.advance_keywords(line);
	}

	/// Applies the line's braces in scan order.
	///
	/// Scanning in order is what makes a line that closes one body and opens another
	/// (`} else {`) settle back to its base level before the next body is raised. Raising
	/// before closing on the same line recorded a level the reader never climbs, and every
	/// `} else {` reported one deeper than it is.
	fn advance_braces(&mut self, line: &LexedLine) {
		let mut segment_start = 0;

		for (offset, character) in line.masked_code.char_indices() {
			match character {
				'{' => self.open_brace(line, segment_start, offset),
				'}' => {
					// The segment a later brace is judged by starts after this close, so a
					// `} else if b {` claims the `else if` and not the `if` it just closed.
					segment_start = offset + 1;
					self.close();
				}
				_ => {}
			}
		}
	}

	/// Applies one opening brace.
	fn open_brace(&mut self, line: &LexedLine, segment_start: usize, offset: usize) {
		// Both offsets come from `char_indices` over this same string, so the range is always
		// a valid character-boundary slice; the fallback keeps the scan total rather than
		// panicking on impossible input.
		let segment = line
			.masked_code
			.get(segment_start..offset)
			.unwrap_or_default();

		// A pending raise was recorded on the construct's own line and is only waiting for
		// its body brace, so claiming it must not raise again.
		let from_pending = self.unit_body_opened && std::mem::take(&mut self.pending_raise);
		let opens_here = self.unit_body_opened && opens_body(segment, line);

		self.pending_raise = false;
		self.unit_body_opened = true;

		if opens_here {
			self.raise(false);
		} else if !from_pending {
			// A plain block — a match arm, a scoping brace — is tracked so its `}` is
			// consumed by it rather than by an enclosing construct.
			self.raised.push(false);
		}
	}

	/// Applies a line's structure keywords.
	///
	/// A line without braces raises through its keywords alone, which is how languages that
	/// signal structure without them work: Ruby's `if x`, Shell's `if …; then`, Python's
	/// `if x:`. Lines that do carry braces are handled entirely by the brace scan, so a
	/// keyword and its body are never counted twice.
	fn advance_keywords(&mut self, line: &LexedLine) {
		let braced = line
			.masked_code
			.chars()
			.any(|character| matches!(character, '{' | '}'));

		if braced || !line.is_code() {
			return;
		}

		if line.nesting.iter().any(|keyword| raises_level(keyword)) {
			// The body brace may be on this line or the next; either way the level is the
			// construct's, so a later plain block must not claim it.
			self.raise(true);
		} else {
			// Any other code line ends the window in which a body brace could still follow
			// the raise, so a stray pending flag is cleared rather than attached to a later
			// block.
			self.pending_raise = false;
		}

		// End-keyword languages close their bodies the way they opened them: `fi`, `done`,
		// and `end` pair with the keyword that raised, the way a `}` pairs with a `{`.
		if closes_with_keyword(line) {
			self.close();
		}
	}

	/// Raises the level one step and records the block that opened.
	fn raise(&mut self, awaits_body: bool) {
		self.depth += 1;
		self.max = self.max.max(self.depth);
		self.raised.push(true);
		self.pending_raise = awaits_body;
	}

	/// Closes the innermost open block.
	fn close(&mut self) {
		if self.raised.pop() == Some(true) {
			self.depth = self.depth.saturating_sub(1);
		}
	}
}

/// What one line costs, and how much of that cost came from nesting.
struct LineCost {
	total: usize,
	nesting_penalty: usize,
}

/// Computes the cognitive cost of a single line at a known nesting `depth`.
fn line_cost(line: &LexedLine, depth: usize) -> LineCost {
	let mut total = 0;
	let mut nesting_penalty = 0;

	for keyword in &line.nesting {
		let (costs, exempt_from_nesting) = nesting_charge(keyword);

		if !costs {
			continue;
		}

		total += 1;

		// A construct charged for its nesting is what makes cognitive complexity differ from
		// cyclomatic: the same branch reads harder the deeper it sits.
		if !exempt_from_nesting {
			total += depth;
			nesting_penalty += depth;
		}
	}

	// A run of `&&`/`||` counts once; alternating operators read as separate conditions and so count
	// again, which is why this is modeled as a sequence rather than a simple presence check.
	if line.logical_operators > 0 {
		total += count_logical_sequences(&line.text, line.logical_operators);
	}

	LineCost {
		total,
		nesting_penalty,
	}
}

/// Reports whether a keyword costs a point, and whether it is exempt from the nesting penalty.
///
/// Chained alternatives and construct markers are exempt because they do not feel nested to the
/// reader: `else` and its `elif` spellings continue a decision already accounted for rather than
/// opening a new one.
fn nesting_charge(keyword: &str) -> (bool, bool) {
	match keyword {
		"else" | "elif" | "elseif" | "else if" | "case" | "when" | "on" => (true, true),
		"if" | "elif" | "elseif" | "else if" | "unless" | "guard" | "for" | "foreach" | "while"
		| "until" | "repeat" | "loop" | "do" | "switch" | "match" | "select" | "cond" | "catch"
		| "except" | "rescue" => (true, false),
		_ => (false, true),
	}
}

/// Counts how many separate logical operator runs a line contains.
///
/// `a && b && c` is one sequence; `a && b || c` is two, because the reader must track that the
/// second operator changes the grouping. The count is therefore the number of times the
/// operator *kind* changes, plus one for the initial run.
///
/// Positions must be gathered across all operators before comparing kinds — a previous version
/// counted each operator's occurrences in turn, so `a && b || c && d` compared a `&&` against a
/// later `||` and then an earlier `&&` against a still-earlier `||`, reporting run boundaries
/// that did not exist.
fn count_logical_sequences(text: &str, total_operators: usize) -> usize {
	/// Every logical operator, in search order.
	const OPERATORS: [&str; 4] = ["&&", "||", " and ", " or "];

	if total_operators == 0 {
		return 0;
	}

	let mut positions: Vec<(usize, &str)> = OPERATORS
		.iter()
		.flat_map(|operator| positions_of(text, operator).map(|position| (position, *operator)))
		.collect();

	positions.sort_by_key(|(position, _)| *position);

	// A run continues while the operator kind is unchanged, so each change starts a new run.
	// Destructuring the window pair makes the two-element guarantee visible to the compiler
	// rather than assumed by the indexing.
	let sequences = positions
		.windows(2)
		.filter(|pair| matches!(pair, [left, right] if left.1 != right.1))
		.count()
		+ 1;

	sequences.min(total_operators)
}

/// Returns the byte positions of every occurrence of `needle` in `text`.
fn positions_of<'text>(
	text: &'text str,
	needle: &'text str,
) -> impl Iterator<Item = usize> + 'text {
	let mut search_from = 0;

	std::iter::from_fn(move || {
		let found = text.get(search_from..)?.find(needle)?;
		let absolute = search_from + found;
		search_from = absolute + needle.len();

		Some(absolute)
	})
}

/// Whether the code between the last close and this brace opens a nesting level.
///
/// A brace by itself is not enough: match arms, scoping blocks, and struct literals all
/// open braces whose contents the reader tracks at the level they already sit at. The
/// keywords the line carries are the evidence that a construct the specification lists
/// opens here, so the check is whether one of them falls in this brace's segment — the
/// text since the previous `}`, which keeps a `} else if b {` from claiming the `if` of
/// the body it just closed.
fn opens_body(segment: &str, line: &LexedLine) -> bool {
	line.nesting
		.iter()
		.any(|keyword| raises_level(keyword) && segment_contains_word(segment, keyword))
		|| declares_function(segment)
		|| opens_closure(segment)
}

/// Whether a nesting keyword's body sits one level deeper than the keyword itself.
///
/// `else` continues a decision the `if` already opened, and `case`/`when`/`on` label
/// branches of one, so none of them raise the level — their bodies sit beside the body
/// they follow, not beneath it. `try` costs nothing until a `catch` actually handles
/// something, so it does not descend either. An `else if` is exempt from the nesting
/// penalty as a chained alternative, but the body it opens is still an if body, so it
/// raises. `fn`/`function` declare nested functions, which are the specification's own
/// nesting clause.
fn raises_level(keyword: &str) -> bool {
	match keyword {
		"elif" | "elseif" | "else if" | "fn" | "function" => true,
		_ => matches!(nesting_charge(keyword), (true, false)),
	}
}

/// Whether a nested function is declared in `segment`.
///
/// A nested function costs no point of its own, but its body sits one level deeper: Rust
/// spells it `fn`, most other languages `function`. Both are matched as whole words, so
/// `format` is not `fn` and `function_name` is not `function`.
fn declares_function(segment: &str) -> bool {
	segment_contains_word(segment, "fn") || segment_contains_word(segment, "function")
}

/// Whether `segment` opens a closure body.
///
/// A closure's parameter list is a pair of pipes, `|x| { … }` or the empty `|| { … }`,
/// which is the only brace-opening shape in Rust that carries them. Or-patterns share the
/// character — `Some(1 | 2) => { … }` — but a match arm always precedes its brace with
/// `=>`, so a segment containing that arrow is an arm, not a closure. Languages whose
/// closures use `=>` instead of pipes are not detected here, and their bodies sit at the
/// level the reader was already at.
fn opens_closure(segment: &str) -> bool {
	!segment.contains("=>") && segment.matches('|').count() >= 2
}

/// Whether `text` contains `word` as a standalone identifier.
///
/// Keyword borders are what separate a construct from a name that merely contains it:
/// `notify` is not `if`, and `elsewise` is not `else`.
fn segment_contains_word(text: &str, word: &str) -> bool {
	let mut searched = 0;

	while let Some(offset) = text.get(searched..).and_then(|rest| rest.find(word)) {
		let start = searched + offset;
		let end = start + word.len();
		let bytes = text.as_bytes();
		let before = start
			.checked_sub(1)
			.and_then(|index| bytes.get(index))
			.is_none_or(|byte| !is_word_byte(*byte));
		let after = bytes.get(end).is_none_or(|byte| !is_word_byte(*byte));

		if before && after {
			return true;
		}

		searched = end;
	}

	false
}

/// Whether a byte can continue an identifier, so a keyword bordering one is part of a
/// longer name rather than the keyword itself.
fn is_word_byte(byte: u8) -> bool {
	byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'$'
}

/// Returns true when a line closes a nesting level with an end keyword.
///
/// Brace languages are handled by counting braces, because a closing brace carries no
/// keyword to look for; this is the counterpart for languages whose blocks close with a
/// word, where the word is the only signal there is.
fn closes_with_keyword(line: &LexedLine) -> bool {
	let trimmed = line.text.trim();

	trimmed == "end"
		|| trimmed == "fi"
		|| trimmed == "done"
		|| matches!(trimmed, "end)" | "end," | "end." | "esac")
}
