//! Code-quality rules beyond layout.
//!
//! Layout rules answer "does this look right". These answer "is this saying something a reader will
//! stumble on": an unnamed constant, a name too short to carry meaning, an error handler that
//! silently swallows, or a block of code that was commented out instead of deleted.
//!
//! Each rule here is deliberately conservative. These are judgement calls rather than formatting, so
//! a false positive costs more than a miss — an author who is told their `i` is a bad name learns to
//! ignore the tool.

use monostyle_core::Category;
use monostyle_core::Finding;
use monostyle_core::FindingBuilder;
use monostyle_core::Language;
use monostyle_core::Severity;
use monostyle_core::Span;
use monostyle_lexer::LexedFile;
use monostyle_lexer::LexedLine;
use monostyle_lexer::LineKind;

use crate::config::RulesConfig;

/// Reports numeric literals that carry meaning without naming it.
///
/// A `3` in an expression forces the reader to work out what it counts. The rule stays quiet on the
/// values where a bare literal is universal — `0`, `1`, `2`, and the common round numbers — because
/// flagging those is noise, and noise is what makes people stop reading findings.
#[must_use]
pub fn magic_numbers(file: &LexedFile, config: &RulesConfig) -> Vec<Finding> {
	if !config.report_magic_numbers {
		return Vec::new();
	}

	let mut findings = Vec::new();

	for line in &file.lines {
		if !line.is_code() {
			continue;
		}

		// A literal in a constant declaration is already named by the declaration it sits in.
		if is_constant_declaration(line) {
			continue;
		}

		for literal in unnamed_literals(line) {
			findings.push(magic_number_finding(line, &literal.text));
		}
	}

	findings
}

/// Returns the numeric literals on a line that carry meaning without a name.
fn unnamed_literals(line: &LexedLine) -> Vec<NumericLiteral> {
	numeric_literals(&line.masked_code)
		.into_iter()
		.filter(|literal| !is_conventional(literal.value))
		.collect()
}

/// Builds the finding for one unnamed literal.
fn magic_number_finding(line: &LexedLine, text: &str) -> Finding {
	FindingBuilder::new(
		"readability/magic-number",
		Category::Readability,
		Span::new(line.start_byte, line.end_byte, line.number, line.number),
	)
	.severity(Severity::Minor)
	.weight(0.5)
	.message(format!(
		"the literal `{text}` carries meaning without a name"
	))
	.suggestion(
		"Name this value as a constant so the reader knows what it represents and where else it is used.",
	)
	.build()
}

/// Reports identifiers too short to carry meaning.
///
/// Short names are conventional in a few positions — a loop index, a coordinate, a lambda parameter
/// — so those are recognized rather than reported. The rule targets names in binding positions where
/// a reader has to hold the meaning in mind for more than a line.
#[must_use]
pub fn short_identifiers(file: &LexedFile, config: &RulesConfig) -> Vec<Finding> {
	if !config.report_short_identifiers {
		return Vec::new();
	}

	let mut findings = Vec::new();

	for line in &file.lines {
		if !line.is_code() {
			continue;
		}

		for name in unclear_names(line, file.language, config.min_identifier_length) {
			findings.push(
				FindingBuilder::new(
					"readability/short-identifier",
					Category::Readability,
					Span::new(line.start_byte, line.end_byte, line.number, line.number),
				)
				.severity(Severity::Minor)
				.weight(0.35)
				.message(format!("`{name}` is too short to convey meaning"))
				.suggestion(
					"Rename this to describe what it holds, or keep it if its scope makes the \
					 meaning obvious.",
				)
				.build(),
			);
		}
	}

	findings
}

/// Returns the names on a line that are too short to carry meaning.
///
/// Two kinds are kept: names at or above the minimum length, and the conventional short names that are
/// idiomatic where they appear. A loop index named `i` or a coordinate named `x` is clearer than any longer
/// alternative, so reporting those would be wrong.
fn unclear_names(line: &LexedLine, language: Language, minimum: usize) -> Vec<String> {
	binding_names(&line.masked_code, language)
		.into_iter()
		.filter(|name| name.chars().count() < minimum)
		.filter(|name| !is_conventional_name(name))
		.collect()
}

/// Reports exception handlers that discard the error.
///
/// An empty handler is almost always a bug rather than a decision: it turns a failure into silence,
/// which is the hardest kind of problem to diagnose. The rule fires on a handler whose body is empty
/// or is a bare pass, and the suggestion asks for the reason rather than for a change, because
/// sometimes swallowing is correct.
#[must_use]
pub fn empty_handlers(file: &LexedFile, config: &RulesConfig) -> Vec<Finding> {
	if !config.report_empty_handlers {
		return Vec::new();
	}

	let mut findings = Vec::new();

	for (index, line) in file.lines.iter().enumerate() {
		if !line.is_code() || !introduces_handler(line) {
			continue;
		}

		if !handler_discards_its_error(file, index, line) {
			continue;
		}

		findings.push(
			FindingBuilder::new(
				"readability/empty-handler",
				Category::Readability,
				Span::new(line.start_byte, line.end_byte, line.number, line.number),
			)
			.severity(Severity::Major)
			.weight(1.5)
			.message("this error handler discards the error without acting on it")
			.suggestion(
				"Handle the error, propagate it, or add a comment explaining why discarding it is \
				 safe — and it usually is not.",
			)
			.build(),
		);
	}

	findings
}

/// Whether a line opens an error handler.
///
/// Two forms qualify: a language with a handler keyword (`catch`, `except`, `rescue`), and Rust, which has
/// none and instead uses a `match` arm whose pattern is `Err`.
fn introduces_handler(line: &LexedLine) -> bool {
	/// Keywords that introduce an error handler.
	const HANDLER_KEYWORDS: &[&str] = &["catch", "except", "rescue"];

	if opens_with_handler_keyword(&line.masked_code, HANDLER_KEYWORDS) {
		return true;
	}

	// The check is on the pattern followed by a fat arrow so an `Err` in an expression is not mistaken for a
	// handler.
	let trimmed = line.masked_code.trim_start();

	trimmed.starts_with("Err")
		&& line.masked_code.contains("=>")
		&& line.masked_code.trim_end().ends_with('{')
}

/// Whether a line opens a handler block with one of `keywords`.
///
/// The keyword has to be a word of its own, not a fragment of a longer identifier. Matching any
/// occurrence of the text reported every function whose *name* contained one:
/// `fn complexity_rules_catch_a_hard_function()` opens no handler, and was reported as discarding an
/// error it never caught.
///
/// The keyword may sit mid-line, because JavaScript writes `} catch (error) {` after the closing brace
/// of the `try` body. Requiring the line to begin with it would miss every such handler.
fn opens_with_handler_keyword(code: &str, keywords: &[&str]) -> bool {
	keywords
		.iter()
		.filter(|keyword| !keyword.is_empty())
		.any(|keyword| has_word(code, keyword))
}

/// Whether `code` contains `word` delimited by non-identifier characters on both sides.
fn has_word(code: &str, word: &str) -> bool {
	let mut from = 0;

	while let Some(offset) = code.get(from..).and_then(|rest| rest.find(word)) {
		let start = from + offset;
		let end = start + word.len();
		let before = code.get(..start).and_then(|head| head.chars().next_back());
		let after = code.get(end..).and_then(|tail| tail.chars().next());

		// Both sides must be word boundaries, which is what a bare `find` does not give: without
		// this, `rescue_all` matches `rescue` and a name like `catches` matches `catch`.
		if !is_identifier_char(before) && !is_identifier_char(after) {
			return true;
		}

		from = end;
	}

	false
}

/// Whether a character can appear inside an identifier.
fn is_identifier_char(character: Option<char>) -> bool {
	character.is_some_and(|value| value.is_alphanumeric() || value == '_')
}

/// Whether a handler's body does nothing with the error.
fn handler_discards_its_error(file: &LexedFile, index: usize, line: &LexedLine) -> bool {
	// A handler on the same line as its body, as in `catch (e) {}` or `Err(_) => {}`.
	if let Some(inline) = line
		.masked_code
		.split_once('{')
		.map(|(_head, tail)| tail.trim())
		&& body_is_empty(inline)
	{
		return true;
	}

	// Otherwise the body is the next code line, since a brace or an indent follows the handler.
	let Some(body) = file
		.lines
		.iter()
		.skip(index + 1)
		.find(|candidate| candidate.is_code())
	else {
		return false;
	};

	body_is_empty(body.masked_code.trim())
}

/// Whether a handler body contains nothing.
///
/// A body that closes immediately contains only a brace, which is the shape an empty block produces whether
/// it was written on one line or two.
fn body_is_empty(text: &str) -> bool {
	/// Bodies that do nothing while looking deliberate.
	///
	/// A bare `pass` or `continue` is how a deliberate no-op is written in each language.
	const EMPTY_BODIES: &[&str] = &["pass", "continue", "return;", "..."];

	if text.is_empty() || text == "}" {
		return true;
	}

	EMPTY_BODIES.contains(&text)
}

/// Reports blocks of code that were commented out rather than deleted.
///
/// Commented-out code is a maintenance hazard: it is never formatted, never compiled, and never
/// tested, so it drifts out of sync and misleads anyone who reads it. Version control already
/// remembers it, which is what the suggestion says.
#[must_use]
pub fn commented_out_code(file: &LexedFile, config: &RulesConfig) -> Vec<Finding> {
	/// Consecutive comment lines that look like code before a block is reported.
	const RUN_LIMIT: usize = 3;

	if !config.report_commented_out_code {
		return Vec::new();
	}

	let mut findings = Vec::new();
	let mut run_start: Option<usize> = None;
	let mut run_length = 0;
	let mut code_like = 0;

	for line in &file.lines {
		if line.kind == LineKind::Comment {
			run_start.get_or_insert(line.number);
			run_length += 1;

			if looks_like_code(line) {
				code_like += 1;
			}

			continue;
		}

		// A non-comment line ends the block. A blank line ends it too, because a commented-out block
		// is contiguous and a paragraph break means the comments either side are unrelated.
		if line.is_blank() || line.is_code() {
			report_run(
				&file.lines,
				run_start,
				run_length,
				code_like,
				RUN_LIMIT,
				&mut findings,
			);

			run_start = None;
			run_length = 0;
			code_like = 0;
		}
	}

	report_run(
		&file.lines,
		run_start,
		run_length,
		code_like,
		RUN_LIMIT,
		&mut findings,
	);
	findings
}

/// Reports a run of commented-out code when it is long enough to be a block.
fn report_run(
	lines: &[LexedLine],
	run_start: Option<usize>,
	run_length: usize,
	code_like: usize,
	limit: usize,
	findings: &mut Vec<Finding>,
) {
	// The threshold is on the code-like lines rather than on the run length, because an explanatory
	// comment block is often long while a commented-out one is short. Counting `a` on its own line as
	// code is unreliable, so requiring several lines that look like code keeps a prose block from being
	// reported while a real commented-out function still is.
	if code_like < limit {
		return;
	}

	let Some(start) = run_start else {
		return;
	};

	let Some(line) = lines.iter().find(|line| line.number == start) else {
		return;
	};

	findings.push(
		FindingBuilder::new(
			"readability/commented-out-code",
			Category::Readability,
			Span::new(line.start_byte, line.end_byte, start, start + run_length),
		)
		.severity(Severity::Minor)
		.weight(0.75)
		.message(format!("{code_like} lines of commented-out code"))
		.suggestion(
			"Delete this. Version control remembers it, and leaving it here means it will never be \
			 formatted, compiled, or tested again.",
		)
		.build(),
	);
}

/// A numeric literal found in a line.
struct NumericLiteral {
	/// The literal as written.
	text: String,
	/// Its parsed value.
	value: f64,
}

/// Extracts numeric literals from masked source.
fn numeric_literals(text: &str) -> Vec<NumericLiteral> {
	let mut literals = Vec::new();
	let characters: Vec<char> = text.chars().collect();
	let mut index = 0;

	while index < characters.len() {
		let Some(character) = characters.get(index).copied() else {
			break;
		};

		// A digit only starts a literal when it is not part of an identifier, so `x2` is skipped.
		let preceded_by_identifier = index
			.checked_sub(1)
			.and_then(|previous| characters.get(previous))
			.is_some_and(|previous| previous.is_alphanumeric() || *previous == '_');

		if !character.is_ascii_digit() || preceded_by_identifier {
			index += 1;
			continue;
		}

		let start = index;

		while characters.get(index).is_some_and(|character| {
			character.is_ascii_alphanumeric() || *character == '.' || *character == '_'
		}) {
			index += 1;
		}

		let literal: String = characters
			.get(start..index)
			.unwrap_or_default()
			.iter()
			.collect();
		let normalized = literal.replace('_', "");

		// A version string or hash looks numeric but is not a quantity; requiring a parse keeps them
		// out of the results.
		if let Ok(value) = normalized.parse::<f64>() {
			// Skip anything with too many digits to be a meaningful constant, such as a timestamp or
			// an embedded identifier.
			if normalized.chars().filter(char::is_ascii_digit).count() > 9 {
				continue;
			}

			literals.push(NumericLiteral {
				text: literal,
				value,
			});
		}
	}

	literals
}

/// Whether a value is conventional enough that a bare literal is fine.
fn is_conventional(value: f64) -> bool {
	/// Values that are idiomatic without a name.
	const CONVENTIONAL: &[f64] = &[
		0.0, 1.0, 2.0, 3.0, 4.0, 10.0, 100.0, 1000.0, 0.5, 0.1, 0.01, 1e3, 1e6, 1e9, 60.0, 24.0,
		3600.0, 255.0, 1024.0,
	];

	CONVENTIONAL
		.iter()
		.any(|candidate| (candidate - value).abs() < f64::EPSILON)
}

/// Whether a line declares a constant, in which case its literal is already named.
fn is_constant_declaration(line: &LexedLine) -> bool {
	/// Keywords that introduce a named constant.
	///
	/// `let` is deliberately absent: it binds a mutable variable in Rust and JavaScript, so a `let`
	/// holding a bare literal has not named anything. Treating it as a constant suppressed the rule
	/// on nearly every assignment.
	const CONSTANT_KEYWORDS: &[&str] = &["const", "static", "final", "define"];

	line.masked_code
		.split(|character: char| !character.is_alphanumeric() && character != '_')
		.any(|word| CONSTANT_KEYWORDS.contains(&word))
}

/// Extracts names from binding positions.
///
/// Only declarations and assignments are considered, because those are where a name is chosen rather
/// than reused. A short parameter name in a closure is idiomatic in a way a short variable is not.
fn binding_names(text: &str, language: Language) -> Vec<String> {
	// An import alias is the imported module's name, not a name the author chose: `import typing as T`
	// carries whatever the library is called, so reporting `T` asks for a rename nobody can make.
	if is_import(text) {
		return Vec::new();
	}

	let mut names = Vec::new();

	// `push_name` deduplicates, because a name can be found by both the assignment path and the declaration
	// path — `let ab = x` matches both — and reporting it twice inflates the count for one problem.
	push_name(&mut names, assigned_name(text));
	push_unique(&mut names, declared_names(text, language));

	names
}

/// Whether a line is an import, whose aliases are not the author's naming choice.
fn is_import(text: &str) -> bool {
	let trimmed = text.trim_start();

	trimmed.starts_with("import ") || trimmed.starts_with("use ") || trimmed.starts_with("from ")
}

/// Returns the name being assigned to, when the line is an assignment.
///
/// A comparison is skipped, because `a == b` binds nothing.
fn assigned_name(text: &str) -> Option<String> {
	let (head, _value) = text.split_once('=')?;

	// Strip the operator itself and any compound form such as `+=`.
	let head = head
		.trim_end_matches(['=', '!', '<', '>', ':', '+', '-', '*', '/', '%'])
		.trim_end();

	if head.ends_with(['!', '<', '>']) {
		return None;
	}

	last_identifier(head)
}

/// Returns the names introduced by declaration keywords on a line.
fn declared_names(text: &str, language: Language) -> Vec<String> {
	let words = words_of(text);
	let keywords = declaration_keywords(language);

	words
		.iter()
		.enumerate()
		.filter(|(_index, word)| keywords.contains(&word.to_ascii_lowercase().as_str()))
		.filter_map(|(index, _word)| words.get(index + 1))
		.filter(|name| !keywords.contains(&name.to_ascii_lowercase().as_str()))
		.map(|name| (*name).to_string())
		.collect()
}

/// The words that introduce a binding in `language`.
fn declaration_keywords(language: Language) -> &'static [&'static str] {
	match language {
		Language::Python | Language::Ruby | Language::Elixir | Language::Haskell => {
			&["def", "class", "for", "as"]
		}
		Language::Dart => &["var", "final", "const", "late", "for"],
		_ => &["let", "const", "var", "fn", "def", "function", "for", "val"],
	}
}

/// Splits a line into its identifier-like words.
fn words_of(text: &str) -> Vec<&str> {
	text.split(|character: char| !character.is_alphanumeric() && character != '_')
		.filter(|word| !word.is_empty())
		.collect()
}

/// Adds a name to `names` when there is one.
fn push_name(names: &mut Vec<String>, name: Option<String>) {
	if let Some(name) = name {
		push_unique(names, std::iter::once(name));
	}
}

/// Adds names to `names`, skipping those already present.
fn push_unique(names: &mut Vec<String>, candidates: impl IntoIterator<Item = String>) {
	for name in candidates {
		if !names.contains(&name) {
			names.push(name);
		}
	}
}

/// Returns the last identifier in `text`.
fn last_identifier(text: &str) -> Option<String> {
	let identifier: String = text
		.chars()
		.rev()
		.take_while(|character| character.is_alphanumeric() || *character == '_')
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

/// Whether a short name is conventional enough to keep.
fn is_conventional_name(name: &str) -> bool {
	/// Single letters that are idiomatic in math and iteration.
	const CONVENTIONAL: &[&str] = &[
		"i", "j", "k", "n", "x", "y", "z", "w", "h", "r", "g", "b", "a", "t", "id", "ok", "up",
		"db", "fs", "io", "os", "el", "ev", "to", "us", "me", "it", "_",
	];

	CONVENTIONAL.contains(&name)
}

/// Whether a comment line looks like code rather than prose.
///
/// The test is syntactic: code has brackets, semicolons, or an assignment, and prose does not.
fn looks_like_code(line: &LexedLine) -> bool {
	/// Markers that appear in code but not in prose about code.
	const CODE_MARKERS: &[&str] = &[";", "()", "=>", "->", "==", "!=", "::", "}"];

	let body = line.comment_body();

	if body.trim().is_empty() {
		return false;
	}

	// A sentence ends in a period and has several words; code rarely does.
	let ends_like_prose = body.trim_end().ends_with('.')
		&& body.split_whitespace().count() > 6
		&& !body.contains(';');

	if ends_like_prose {
		return false;
	}

	CODE_MARKERS.iter().any(|marker| body.contains(marker))
}
