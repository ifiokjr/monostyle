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
	let mut max_nesting = 0;
	let mut depth: usize = 0;

	for line in lines {
		// The nesting level a construct sits at is the depth *before* this line opens anything, which
		// is why the cost is computed first and the depth updated after.
		let cost = line_cost(line, depth);

		total += cost.total;
		nesting_penalty += cost.nesting_penalty;

		// Depth changes after costing so that a construct is charged for the nesting it sits inside,
		// not the nesting it creates.
		if opens_nesting(line) {
			depth += 1;
			max_nesting = max_nesting.max(depth);
		}

		if closes_nesting(line) {
			depth = depth.saturating_sub(1);
		}
	}

	CognitiveComplexity {
		total,
		nesting_penalty,
		max_nesting,
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
/// reader: `else` continues a decision already accounted for rather than opening a new one.
fn nesting_charge(keyword: &str) -> (bool, bool) {
	match keyword {
		"else" | "case" | "when" | "on" => (true, true),
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

/// Returns true when a line opens a nesting level.
fn opens_nesting(line: &LexedLine) -> bool {
	if line.nesting.iter().any(|keyword| {
		matches!(
			keyword.as_str(),
			"if" | "elif"
				| "elseif" | "else if"
				| "else" | "unless"
				| "guard" | "for"
				| "foreach" | "while"
				| "until" | "repeat"
				| "loop" | "do"
				| "switch" | "match"
				| "select" | "cond"
				| "catch" | "except"
				| "rescue" | "try"
				| "with" | "case"
				| "when" | "let"
				| "function"
		)
	}) {
		return true;
	}

	// A line that opens a block ends in a brace or a block keyword, which is how brace
	// languages signal nestable structure without a keyword to match.
	let trimmed = line.masked_code.trim_end();

	trimmed.ends_with('{') || trimmed.ends_with("then") || trimmed.ends_with("do")
}

/// Returns true when a line closes a nesting level.
///
/// Brace languages are handled by counting braces rather than by keyword, because a
/// closing brace carries no keyword to look for.
fn closes_nesting(line: &LexedLine) -> bool {
	let trimmed = line.text.trim();

	if trimmed.starts_with('}') || trimmed == "end" || trimmed == "fi" || trimmed == "done" {
		return true;
	}

	matches!(trimmed, "end)" | "end," | "end." | "esac")
}
