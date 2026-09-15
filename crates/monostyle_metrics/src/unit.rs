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

		// An expression-bodied function has no braces at all: `const f = (x) => x + 1`. The unit is
		// the declaration line, and without this case the `pending` entry is never resolved and the
		// function is dropped from the report entirely.
		let expression_body = !body_opened
			&& opens == 0
			&& closes == 0
			&& line.masked_code.contains("=>")
			&& before == declaration_depth;

		if body_opened || one_liner || expression_body {
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

	// Shell declares a function as `name()` or `function name`, with no keyword that reads as one. A
	// leading identifier followed directly by an empty parameter list is the whole signature, so it is
	// recognized before the general path below rejects it for having no declaration keyword.
	if let Some(name) = shell_function_name(trimmed) {
		return Some(name);
	}

	// An arrow function has no declaration keyword either. `const name = (args) =>` and
	// `name = (args) =>` both define a callable, and without this the function is invisible.
	if let Some(name) = arrow_function_name(trimmed) {
		return Some(name);
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

/// Recognizes a shell function declaration.
///
/// Matches `name() {` and `function name {`, which are the two forms in POSIX sh, bash, and zsh. The
/// empty parameter list is what distinguishes a declaration from a subshell.
fn shell_function_name(code: &str) -> Option<String> {
	if let Some(rest) = code.strip_prefix("function ") {
		return first_identifier(rest.trim_start());
	}

	// `name()` with nothing between the parentheses.
	let open = code.find('(')?;
	let close = code.find(')')?;

	if close != open + 1 {
		return None;
	}

	let name = code[..open].trim();

	// A bare word followed by `()` is a declaration; anything with punctuation is not.
	if name.is_empty()
		|| !name
			.chars()
			.all(|character| character.is_alphanumeric() || character == '_')
	{
		return None;
	}

	Some(name.to_string())
}

/// Recognizes an arrow function bound to a name.
///
/// Handles `const name = (..) =>` and `name = (..) =>`, which are how TypeScript and JavaScript
/// declare a function without the `function` keyword.
fn arrow_function_name(code: &str) -> Option<String> {
	// Everything before the arrow and before the parameter list is the binding, which must contain an
	// assignment or this is not a declaration.
	let (head, _tail) = code.split_once("=>")?;
	let before_parameters = head.split('(').next()?;

	// The name is the identifier immediately before the `=`. Splitting on the assignment is what makes
	// this correct: trimming trailing `=` characters from the whole prefix does not work, because the
	// space in `const compute = ` sits between the name and the operator.
	let (target, _value) = before_parameters.split_once('=')?;

	// The target carries trailing whitespace before the operator, and `last_identifier` reads from the
	// end, so it must be trimmed or it finds nothing.
	last_identifier(target.trim_end())
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
