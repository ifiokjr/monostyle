//! Halstead metrics and the maintainability index.
//!
//! Halstead's measures count operators and operands rather than branches, so they capture a
//! different kind of pressure than cyclomatic or cognitive complexity: a function can have one path
//! and still be dense with arithmetic. The maintainability index combines them into a single figure,
//! which is what most people recognize from Visual Studio and `SonarQube`.
//!
//! # Why the index is normalized to 0–100
//!
//! The classic maintainability index is computed on a 0–171 scale and then rescaled. This
//! implementation produces the widely used 0–100 form directly, so it can be reported beside the
//! other scores without a second unit in the output.
//!
//! | Input | Meaning |
//! | --- | --- |
//! | Volume | Halstead volume: information content of the operator and operand streams |
//! | Cyclomatic | Independent paths |
//! | Lines | Lines of code |
//!
//! The formula is the standard one:
//!
//! ```text
//! MI = max(0, (171 - 5.2 ln V - 0.23 G - 16.2 ln L) / 171 * 100)
//! ```

use std::collections::HashSet;

use monostyle_core::Language;
use monostyle_lexer::LexedFile;
use monostyle_lexer::LexedLine;
use serde::Serialize;

/// Halstead's measures for a body of code.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Halstead {
	/// Distinct operators.
	pub distinct_operators: usize,
	/// Distinct operands.
	pub distinct_operands: usize,
	/// Total operator occurrences.
	pub total_operators: usize,
	/// Total operand occurrences.
	pub total_operands: usize,
	/// Vocabulary: distinct operators plus distinct operands.
	pub vocabulary: usize,
	/// Length: total operators plus total operands.
	pub length: usize,
	/// Volume: `length * log2(vocabulary)`.
	pub volume: f64,
	/// Difficulty: `distinct_operators / 2 * total_operands / distinct_operands`.
	pub difficulty: f64,
	/// Effort: `difficulty * volume`.
	pub effort: f64,
}

impl Halstead {
	/// Measures for code with no operators or operands.
	pub const EMPTY: Self = Self {
		distinct_operators: 0,
		distinct_operands: 0,
		total_operators: 0,
		total_operands: 0,
		vocabulary: 0,
		length: 0,
		volume: 0.0,
		difficulty: 0.0,
		effort: 0.0,
	};

	/// Whether there is enough code for the measures to mean anything.
	#[must_use]
	pub fn is_meaningful(&self) -> bool {
		self.vocabulary > 1 && self.length > 0
	}
}

/// A maintainability index measurement.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct MaintainabilityIndex {
	/// The index on a 0–100 scale, where higher is more maintainable.
	pub value: f64,
}

impl MaintainabilityIndex {
	/// A qualitative band for the index.
	///
	/// The thresholds follow the common convention: above 85 is easy to maintain, 65–85 is
	/// moderate, and below 65 is difficult.
	#[must_use]
	pub fn grade(&self) -> &'static str {
		match self.value {
			value if value >= 85.0 => "high",

			value if value >= 65.0 => "moderate",
			_ => "low",
		}
	}
}

/// Computes Halstead's measures over a body of code.
#[must_use]
pub fn halstead(file: &LexedFile) -> Halstead {
	halstead_of_lines(&file.lines, file.language)
}

/// Computes Halstead's measures over a slice of lines.
#[must_use]
pub fn halstead_of_lines(lines: &[LexedLine], _language: Language) -> Halstead {
	let operators = Operators::SHARED;
	// Sets rather than vectors: membership is tested once per token, so a linear scan makes the
	// measure quadratic in the token count, which is what dominated the runtime on large files.
	let mut distinct_operators: HashSet<String> = HashSet::new();
	let mut distinct_operands: HashSet<String> = HashSet::new();
	let mut total_operators = 0;
	let mut total_operands = 0;

	for line in lines {
		if !line.is_code() {
			continue;
		}

		// The masked view is used so that operators and operands inside strings and comments are
		// not counted, which is the same reason complexity counts them from the masked text.
		for token in tokenize(&line.masked_code, &operators) {
			match token {
				Token::Operator(text) => {
					total_operators += 1;
					distinct_operators.insert(text.to_string());
				}
				Token::Operand(text) => {
					total_operands += 1;
					distinct_operands.insert(text.to_string());
				}
			}
		}
	}

	let distinct_operators = distinct_operators.len();
	let distinct_operands = distinct_operands.len();
	let vocabulary = distinct_operators + distinct_operands;
	let length = total_operators + total_operands;
	let volume = if vocabulary > 1 {
		length as f64 * (vocabulary as f64).log2()
	} else {
		0.0
	};
	let difficulty = if distinct_operands > 0 {
		distinct_operators as f64 / 2.0 * total_operands as f64 / distinct_operands as f64
	} else {
		0.0
	};

	Halstead {
		distinct_operators,
		distinct_operands,
		total_operators,
		total_operands,
		vocabulary,
		length,
		volume,
		difficulty,
		effort: difficulty * volume,
	}
}

/// Computes the maintainability index for a body of code.
#[must_use]
pub fn maintainability_index(
	halstead: Halstead,
	cyclomatic: usize,
	lines_of_code: usize,
) -> MaintainabilityIndex {
	// Without a vocabulary there is no information content to measure, so the index would be a
	// function of line count alone. Reporting the maximum is honest: there is nothing wrong with a
	// body this small.
	if !halstead.is_meaningful() || lines_of_code == 0 {
		return MaintainabilityIndex { value: 100.0 };
	}

	let volume = halstead.volume.max(1.0);
	let lines = lines_of_code.max(1) as f64;
	let cyclomatic = cyclomatic.max(1) as f64;

	let raw = 171.0 - 5.2 * volume.ln() - 0.23 * cyclomatic - 16.2 * lines.ln();
	let value = (raw / 171.0 * 100.0).clamp(0.0, 100.0);

	MaintainabilityIndex { value }
}

/// A token classified for Halstead counting.
enum Token<'text> {
	/// An operator or delimiter.
	Operator(&'text str),
	/// An identifier or literal.
	Operand(&'text str),
}

/// The operator characters counted by the Halstead measures.
///
/// One shared set rather than one per language. The punctuation that carries meaning is nearly
/// identical across the supported languages, and a per-language table would be a place to encode a
/// distinction that does not change the measurement.
struct Operators {
	/// Single-character operators.
	single: &'static [char],
	/// Multi-character operators, matched first.
	multi: &'static [&'static str],
}

impl Operators {
	/// The shared operator set.
	const SHARED: Self = Self {
		single: COMMON_SINGLE,
		multi: COMMON_MULTI,
	};
}

/// Single-character operators shared by every supported language.
const COMMON_SINGLE: &[char] = &[
	'+', '-', '*', '/', '%', '=', '<', '>', '!', '&', '|', '^', '~', '?', ':', ';', ',', '.', '(',
	')', '[', ']', '{', '}',
];

/// Multi-character operators, matched longest first.
const COMMON_MULTI: &[&str] = &[
	"===", "!==", "<=>", "...", ">>>", "->", "=>", "==", "!=", "<=", ">=", "&&", "||", "++", "--",
	"+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=", "<<", ">>", "::", "?.", "??",
];

/// Splits a line into operator and operand tokens.
///
/// Deliberately lexical rather than grammatical: Halstead's measures are defined over token kinds,
/// and a tokenizer that does not need a parse tree is enough to count them.
fn tokenize<'text>(text: &'text str, operators: &Operators) -> Vec<Token<'text>> {
	let mut tokens = Vec::new();
	let mut index = 0;

	while index < text.len() {
		let rest = &text[index..];

		if rest.starts_with(char::is_whitespace) {
			index += rest.chars().next().map_or(1, char::len_utf8);
			continue;
		}

		if let Some(operator) = operators
			.multi
			.iter()
			.find(|operator| rest.starts_with(**operator))
		{
			tokens.push(Token::Operator(operator));
			index += operator.len();
			continue;
		}

		let character = rest.chars().next().unwrap_or_default();

		if operators.single.contains(&character) {
			// A dot is part of a member access rather than an operator of its own, and counting it
			// separately inflates the operator count on any code that calls methods.
			if character != '.' {
				tokens.push(Token::Operator(&rest[..character.len_utf8()]));
			}

			index += character.len_utf8();
			continue;
		}

		// A run of identifier or literal characters is one operand.
		let length = rest
			.char_indices()
			.find(|(_offset, character)| {
				character.is_whitespace() || operators.single.contains(character)
			})
			.map_or(rest.len(), |(offset, _)| offset);

		let length = if length == 0 {
			rest.chars().next().map_or(1, char::len_utf8)
		} else {
			length
		};

		if length > 0 {
			tokens.push(Token::Operand(&rest[..length]));
			index += length;
		} else {
			index += 1;
		}
	}

	tokens
}
