//! Function-like unit detection.
//!
//! Scores are useful per file but actionable per function, so this module finds the
//! regions a complexity number should be attributed to.
//!
//! Detection is intentionally structural rather than semantic: a unit is a region that
//! opens a brace (or indented block, or `end`-terminated block) at a point following a
//! declaration. That is approximate — it cannot tell a `struct` from a `function` in
//! every language — but it finds the regions that carry decisions, which is what the
//! rules need, without a parser per language.

use monostyle_languages::BlockStyle;
use monostyle_lexer::LexedFile;
use monostyle_lexer::LexedLine;
use monostyle_lexer::LineKind;
use serde::Serialize;

/// A function-like region of code.
#[derive(Debug, Clone, Serialize)]
pub struct CodeUnit {
	/// The best available name for the unit.
	pub name: String,
	/// 1-based line where the unit starts.
	pub start_line: usize,
	/// 1-based line where the unit ends.
	pub end_line: usize,
	/// Nesting depth the unit was declared at.
	pub depth: usize,
}

impl CodeUnit {
	/// The number of lines the unit spans.
	#[must_use]
	pub const fn line_count(&self) -> usize {
		self.end_line.saturating_sub(self.start_line) + 1
	}

	/// Whether `line` falls inside this unit.
	#[must_use]
	pub const fn contains(&self, line: usize) -> bool {
		line >= self.start_line && line <= self.end_line
	}
}

/// Finds every function-like unit in `file`.
#[must_use]
pub fn find_units(file: &LexedFile) -> Vec<CodeUnit> {
	match file.profile.block_style {
		BlockStyle::Brace => find_brace_units(&file.lines),
		BlockStyle::Indentation => find_indentation_units(&file.lines),
		BlockStyle::EndKeyword => find_end_keyword_units(file),
	}
}

/// Finds units in brace-delimited languages by tracking brace depth.
///
/// A declaration is complete when the brace depth returns to the level it was declared at.
/// Tracking that level — rather than counting braces only — is what makes nested functions,
/// closures, and blocks inside a function all belong to the enclosing unit instead of each
/// ending it early.
fn find_brace_units(lines: &[LexedLine]) -> Vec<CodeUnit> {
	let mut units = Vec::new();
	let mut depth: usize = 0;
	let mut pending: Option<(String, usize, usize)> = None;
	let mut body_opened = false;

	for line in lines {
		if line.kind == LineKind::Blank || line.kind == LineKind::Comment {
			continue;
		}

		let code = code_only(&line.text);
		let opens = code.matches('{').count();
		let closes = code.matches('}').count();
		let before = depth;

		if pending.is_none()
			&& let Some(name) = unit_name(&code)
		{
			// A declaration is recorded along with the depth it was declared at, which is
			// the depth before its own braces open.
			pending = Some((name, line.number, depth));
			body_opened = false;
		}

		depth = depth.saturating_add(opens).saturating_sub(closes);

		let Some((name, start, declaration_depth)) = pending.clone() else {
			continue;
		};

		if depth > declaration_depth {
			// The body has begun; everything until the depth returns belongs to this unit.
			body_opened = true;

			continue;
		}

		// A one-line body opens and closes on the declaration line itself.
		let one_liner = !body_opened && opens > 0 && closes > 0 && before == declaration_depth;

		if body_opened || one_liner {
			units.push(CodeUnit {
				name,
				start_line: start,
				end_line: line.number,
				depth: declaration_depth,
			});

			pending = None;
			body_opened = false;
		}
	}

	// An unterminated unit still gets reported so its complexity is not silently dropped.
	if let Some((name, start, depth)) = pending {
		let last = lines.last().map_or(start, |line| line.number);

		units.push(CodeUnit {
			name,
			start_line: start,
			end_line: last,
			depth,
		});
	}

	units
}

/// Finds units in indentation-based languages such as Python and Haskell.
fn find_indentation_units(lines: &[LexedLine]) -> Vec<CodeUnit> {
	let mut units = Vec::new();
	let mut index = 0;

	while index < lines.len() {
		let Some(line) = lines.get(index) else {
			break;
		};

		if !line.is_code() {
			index += 1;
			continue;
		}

		let code = code_only(&line.text);

		if let Some(name) = unit_name(&code) {
			let body_indent = line.indent;
			let mut end = line.number;
			let mut cursor = index + 1;

			while let Some(candidate) = lines.get(cursor) {
				if candidate.is_blank() {
					cursor += 1;
					continue;
				}

				if candidate.indent <= body_indent {
					break;
				}

				end = candidate.number;
				cursor += 1;
			}

			units.push(CodeUnit {
				name,
				start_line: line.number,
				end_line: end,
				depth: body_indent / 4,
			});

			index = cursor;
			continue;
		}

		index += 1;
	}

	units
}

/// Finds units in `end`-terminated languages such as Ruby, Lua, and Shell.
fn find_end_keyword_units(file: &LexedFile) -> Vec<CodeUnit> {
	let end_keywords = file.profile.end_keywords;
	let mut units = Vec::new();
	let mut stack: Vec<(String, usize, usize)> = Vec::new();

	for line in &file.lines {
		if !line.is_code() {
			continue;
		}

		let code = code_only(&line.text);

		if let Some(name) = unit_name(&code) {
			stack.push((name, line.number, stack.len()));
		}

		// A closing keyword may appear alone (`end`) or as part of a larger expression
		// (`end)`), so the check is on the first word of the trimmed code.
		let first_word = code.split_whitespace().next().unwrap_or_default();

		if end_keywords.contains(&first_word)
			&& let Some((name, start, depth)) = stack.pop()
		{
			units.push(CodeUnit {
				name,
				start_line: start,
				end_line: line.number,
				depth,
			});
		}
	}

	// An unterminated unit still gets reported so its complexity is not silently dropped.
	for (name, start, depth) in stack {
		let last = file.lines.last().map_or(start, |line| line.number);

		units.push(CodeUnit {
			name,
			start_line: start,
			end_line: last,
			depth,
		});
	}

	units.sort_by_key(|unit| unit.start_line);
	units
}

/// Extracts an identifier for a declaration line, if the line looks like a declaration.
///
/// Two shapes count as a declaration:
///
/// 1. A token preceding a parameter list, where the text before that token contains a
///    declaration keyword (`fn`, `def`, `void`, `function`, ...).
/// 2. An identifier that *begins* the line and is followed by a parameter list, which is how
///    Dart, JavaScript, and Kotlin declare methods with no return type.
///
/// The `ends_with` fallback a previous version used was far too permissive: it matched any
/// call, so `if input.len() > MAX` registered `len` as a function and `Ok(())` registered
/// `Ok`. Requiring the identifier to start the line or sit after a declaration keyword is what
/// keeps calls from being mistaken for definitions.
fn unit_name(code: &str) -> Option<String> {
	let trimmed = code.trim();

	if trimmed.is_empty() {
		return None;
	}

	let parameters = trimmed.find('(')?;
	let before = trimmed[..parameters].trim();
	let name = if before.is_empty() {
		// The identifier begins the line, so it is the first token of the whole statement.
		first_identifier(trimmed)
	} else {
		last_identifier(before)
	}?;

	// A keyword-annotated declaration, such as `pub fn name(` or `static void name(`.
	if is_declaration_prefix(before) {
		return Some(name);
	}

	// An unannotated declaration, such as Dart's `name(` at the start of a line. Restricting
	// this to the start of the line is what excludes `if x.name(` and `result.then(`.
	if before.is_empty() {
		return Some(name);
	}

	None
}

/// Returns the leading identifier of `text`, if it starts with one.
fn first_identifier(text: &str) -> Option<String> {
	let identifier: String = text
		.chars()
		.take_while(|character| {
			character.is_alphanumeric() || *character == '_' || *character == '$'
		})
		.collect();

	if identifier.is_empty()
		|| identifier
			.chars()
			.next()
			.is_some_and(|c| c.is_ascii_digit())
	{
		return None;
	}

	// A control-flow keyword that happens to precede a parenthesis is a statement, not a
	// declaration.
	if is_control_keyword(&identifier) {
		return None;
	}

	Some(identifier)
}

/// Returns true when `word` is a control-flow or expression keyword rather than a name.
fn is_control_keyword(word: &str) -> bool {
	const CONTROL_KEYWORDS: &[&str] = &[
		"if", "else", "for", "while", "switch", "match", "return", "assert", "catch", "except",
		"until", "unless", "when", "case", "do", "yield", "await", "delete", "typeof", "new",
		"throw", "with", "let", "const", "var", "super", "this", "self",
	];

	CONTROL_KEYWORDS.contains(&word)
}

/// Returns the last identifier-like token in `text`.
fn last_identifier(text: &str) -> Option<String> {
	let identifier: String = text
		.chars()
		.rev()
		.take_while(|character| {
			character.is_alphanumeric() || *character == '_' || *character == '$'
		})
		.collect();

	let identifier: String = identifier.chars().rev().collect();

	if identifier.is_empty()
		|| identifier
			.chars()
			.next()
			.is_some_and(|c| c.is_ascii_digit())
	{
		return None;
	}

	Some(identifier)
}

/// Returns true when the text before a parameter list contains a declaration keyword.
fn is_declaration_prefix(before: &str) -> bool {
	const DECLARATION_KEYWORDS: &[&str] = &[
		"fn",
		"def",
		"func",
		"function",
		"fun",
		"sub",
		"proc",
		"method",
		"constructor",
		"void",
		"int",
		"pub",
		"async",
		"static",
		"override",
		"get",
		"set",
	];

	let words: Vec<&str> = before.split_whitespace().collect();

	words
		.iter()
		.any(|word| DECLARATION_KEYWORDS.contains(&word.split('<').next().unwrap_or(word)))
}

/// Returns the line with comments removed, for structural inspection.
fn code_only(text: &str) -> String {
	let mut result = String::with_capacity(text.len());
	let mut in_string: Option<char> = None;
	let mut characters = text.chars().peekable();

	while let Some(character) = characters.next() {
		if let Some(delimiter) = in_string {
			if character == '\\' {
				characters.next();
				continue;
			}

			if character == delimiter {
				in_string = None;
			}

			continue;
		}

		match character {
			'"' | '\'' | '`' => {
				in_string = Some(character);
			}
			'/' if characters.peek() == Some(&'/') => break,
			'#' => break,
			'-' if characters.peek() == Some(&'-') => break,
			_ => result.push(character),
		}
	}

	result
}
