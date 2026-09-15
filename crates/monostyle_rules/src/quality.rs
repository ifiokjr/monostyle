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

		for literal in numeric_literals(&line.masked_code) {
			if is_conventional(literal.value) {
				continue;
			}

			// A literal in a constant declaration is already named by the declaration it sits in.
			if is_constant_declaration(line) {
				continue;
			}

			findings.push(
				FindingBuilder::new(
					"readability/magic-number",
					Category::Readability,
					Span::new(line.start_byte, line.end_byte, line.number, line.number),
				)
				.severity(Severity::Minor)
				.weight(0.5)
				.message(format!(
					"the literal `{}` carries meaning without a name",
					literal.text
				))
				.suggestion(
					"Name this value as a constant so the reader knows what it represents and where \
					 else it is used.",
				)
				.build(),
			);
		}
	}

	findings
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

		for name in binding_names(&line.masked_code, file.language) {
			if name.chars().count() >= config.min_identifier_length {
				continue;
			}

			// Conventional short names are idiomatic in the positions they appear in, and a
			// coordinate or loop index named `x` or `i` is clearer than a long alternative.
			if is_conventional_name(&name) {
				continue;
			}

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

/// Reports exception handlers that discard the error.
///
/// An empty handler is almost always a bug rather than a decision: it turns a failure into silence,
/// which is the hardest kind of problem to diagnose. The rule fires on a handler whose body is empty
/// or is a bare pass, and the suggestion asks for the reason rather than for a change, because
/// sometimes swallowing is correct.
#[must_use]
pub fn empty_handlers(file: &LexedFile, config: &RulesConfig) -> Vec<Finding> {
	/// Keywords that introduce an error handler.
	///
	/// Rust has no such keyword: it handles errors with a `match` arm whose pattern is `Err`. That arm
	/// is checked separately below, because without it the rule never fired on Rust at all.
	const HANDLER_KEYWORDS: &[&str] = &["catch", "except", "rescue"];
	/// Bodies that do nothing.
	const EMPTY_BODIES: &[&str] = &["pass", "{}", "continue", "return;", "..."];

	if !config.report_empty_handlers {
		return Vec::new();
	}

	let mut findings = Vec::new();

	for (index, line) in file.lines.iter().enumerate() {
		if !line.is_code() {
			continue;
		}

		let words: Vec<&str> = line
			.masked_code
			.split(|character: char| !character.is_alphanumeric() && character != '_')
			.collect();

		// A Rust error arm is `Err(..) => {`, which has no handler keyword. The check is on the
		// pattern followed by a fat arrow so that an `Err` in an expression is not mistaken for a
		// handler.
		let trimmed = line.masked_code.trim_start();
		let is_rust_error_arm = trimmed.starts_with("Err")
			&& line.masked_code.contains("=>")
			&& line.masked_code.trim_end().ends_with('{');

		if !words.iter().any(|word| HANDLER_KEYWORDS.contains(word)) && !is_rust_error_arm {
			continue;
		}

		// The handler's body is the following code line, since a brace or an indent follows.
		let Some(body) = file
			.lines
			.iter()
			.skip(index + 1)
			.find(|candidate| candidate.is_code())
		else {
			continue;
		};

		// A handler on the same line as its body, as in `catch (e) {}`.
		let inline_body = line
			.masked_code
			.split_once('{')
			.map(|(_head, tail)| tail.trim());

		let body_text = inline_body.unwrap_or_else(|| body.masked_code.trim());

		// A single closing brace on the next line means the body was empty.
		let is_empty = EMPTY_BODIES.contains(&body_text)
			|| body_text.is_empty()
			|| body_text == "}"
			|| body.masked_code.trim() == "}";

		if !is_empty {
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
	let mut names: Vec<String> = Vec::new();

	// An import alias is the imported module's name, not a name the author chose: `import typing as T`
	// carries whatever the library is called, so reporting `T` asks for a rename the author cannot make.
	let trimmed = text.trim_start();

	if trimmed.starts_with("import ") || trimmed.starts_with("use ") || trimmed.starts_with("from ")
	{
		return names;
	}

	// `push_name` deduplicates, because a name can be found by both the assignment path and the
	// declaration path — `let ab = x` matches both — and reporting it twice inflates the finding count
	// for one problem.
	let push_name = |names: &mut Vec<String>, name: String| {
		if !names.contains(&name) {
			names.push(name);
		}
	};

	// The token before an assignment is being bound.
	if let Some((head, _tail)) = text.split_once('=') {
		// Skip comparisons, which are not bindings.
		let head = head.trim_end_matches(['=', '!', '<', '>', ':', '+', '-', '*', '/', '%']);

		let head = head.trim_end();

		if !head.ends_with(['!', '<', '>'])
			&& let Some(name) = last_identifier(head)
		{
			push_name(&mut names, name);
		}
	}

	// A declaration keyword introduces the name that follows it.
	let words: Vec<&str> = text
		.split(|character: char| !character.is_alphanumeric() && character != '_')
		.filter(|word| !word.is_empty())
		.collect();

	let declaration_words: &[&str] = match language {
		Language::Python | Language::Ruby | Language::Elixir | Language::Haskell => {
			&["def", "class", "for", "as"]
		}
		Language::Dart => &["var", "final", "const", "late", "for"],
		_ => &["let", "const", "var", "fn", "def", "function", "for", "val"],
	};

	for (index, word) in words.iter().enumerate() {
		if declaration_words.contains(word) {
			// The name is the next word, unless the next word is a type-like keyword.
			if let Some(name) = words.get(index + 1)
				&& !declaration_words.contains(name)
			{
				push_name(&mut names, (*name).to_string());
			}
		}
	}

	names
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
