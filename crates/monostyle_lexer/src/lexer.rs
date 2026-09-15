//! The profile-driven scanner.
//!
//! # Why it is a character state machine
//!
//! Scores are only as trustworthy as the tokenizer underneath them, so this is a single
//! forward pass over characters with one explicit state at a time. A previous draft kept a
//! general context stack and re-entered it at the head of each line; that made the
//! interactions between literals, interpolation, and nesting the hardest thing in the file
//! to reason about — precisely where mis-scans hide.
//!
//! The state is now flat and mutually exclusive. Multi-line constructs keep their own
//! small stacks (open literals, and braces within an interpolation), because those are the
//! only places where nesting is genuinely required:
//!
//! | Construct | Mis-scan it prevents |
//! | --- | --- |
//! | Nestable block comments | Rust's `/* /* */ */` closing at the first `*/` |
//! | Triple-quoted literals | Dart's `'''` ending at the first `'` |
//! | Hashed raw strings | Rust's `r#"…"#`, whose closer depends on the opener |
//! | Extra escapes | Nix's `''$`, `'''`, and `''\\`, where `''` does not always close |
//! | Interpolation | `"${foo("x")}"` ending at the inner quote |
//! | Heredocs | Shell and PHP bodies being read as code |
//! | Regex versus division | JavaScript `/` read as a literal where it divides |
//! | Single-line recovery | An unterminated `'` swallowing the rest of the file |
//!
//! # The masked-code contract
//!
//! Every line carries [`LexedLine::masked_code`]: the line with literal and comment
//! *contents* replaced by spaces. Masking rather than deleting keeps columns aligned while
//! guaranteeing no later pass can mistake a `?` inside a Dart string for a ternary, or a
//! `(` inside a Go raw string for a parameter list. All keyword, operator, and brace
//! counting reads the masked view, so those answers cannot disagree with one another.

// Indexing is used pervasively here and is bounded by construction: the scan loop is driven by
// `index < characters.len()`, and every step function advances the cursor by at least one, so
// `characters[index]` is always in range. Lookahead goes through `characters.get`, which handles
// running past the end.
//
// Rewriting each access as `.get(..).unwrap_or(..)` would add a branch and a noise word to the
// scanner's hot path without making a genuine out-of-bounds bug any less likely, and this is the
// file where clarity matters most. The exception is scoped to this module so the rest of the
// workspace keeps the lint.
#![allow(clippy::indexing_slicing)]

use monostyle_core::Language;
use monostyle_languages::Interpolation;
use monostyle_languages::LanguageProfile;
use monostyle_languages::StringRule;
use monostyle_languages::profile_for;

use crate::comment::CommentIntent;
use crate::comment::classify;
use crate::line::LexedLine;
use crate::line::LineKind;

/// The result of lexing one file.
#[derive(Debug, Clone)]
pub struct LexedFile {
	/// The language analyzed.
	pub language: Language,
	/// The profile used, kept so rules can consult language-specific settings.
	pub profile: LanguageProfile,
	/// One entry per physical line.
	pub lines: Vec<LexedLine>,
	/// Whether the file indents with tabs.
	pub uses_tabs: bool,
	/// Whether the file mixes tabs and spaces for indentation.
	pub mixed_indentation: bool,
	/// Constructs that reached end of file unclosed.
	///
	/// A non-empty list means the scanner had to make a recovery decision, so any score
	/// derived from this file is less trustworthy. The test suite asserts on this to catch
	/// regressions in the literal and comment rules.
	pub unterminated: Vec<Unterminated>,
}

/// A construct that was still open when the file ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unterminated {
	/// What kind of construct it was.
	pub kind: UnterminatedKind,
	/// The 1-based line where it opened.
	pub line: usize,
	/// The delimiter that would have closed it.
	pub delimiter: String,
}

/// The kind of unclosed construct.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnterminatedKind {
	/// A block comment.
	BlockComment,
	/// A string literal.
	Literal,
	/// A heredoc.
	Heredoc,
	/// A regular expression literal.
	Regex,
}

impl LexedFile {
	/// Lines that contain executable code.
	pub fn code_lines(&self) -> impl Iterator<Item = &LexedLine> {
		self.lines.iter().filter(|line| line.is_code())
	}

	/// The number of lines that are neither blank, comment-only, nor literal content.
	///
	/// This is the denominator for every density calculation, which is why literal and
	/// comment lines are excluded: a file with a large doc comment is not thereby more
	/// readable per line of code.
	#[must_use]
	pub fn source_line_count(&self) -> usize {
		self.lines
			.iter()
			.filter(|line| !line.is_blank() && !line.is_comment() && !line.is_literal())
			.count()
	}

	/// Total decision points across the file.
	#[must_use]
	pub fn total_decisions(&self) -> usize {
		self.lines.iter().map(LexedLine::decision_count).sum()
	}

	/// Whether scanning completed without recovery.
	#[must_use]
	pub fn is_clean(&self) -> bool {
		self.unterminated.is_empty()
	}
}

/// Lexes `source` in `language`.
#[must_use]
pub fn lex(source: &str, language: Language) -> LexedFile {
	lex_with_profile(source, profile_for(language))
}

/// Lexes `source` using an explicit profile.
#[must_use]
pub fn lex_with_profile(source: &str, profile: LanguageProfile) -> LexedFile {
	let mut scanner = Scanner::new(profile);
	let mut lines = scanner.run(source);
	let unterminated = scanner.finish();

	// Comment intent is decided per *block* here rather than per line during scanning. Reasoning
	// usually sits on a comment's first line and its continuation lines carry none of the
	// markers, so judging lines independently credited the opening and penalized the rest.
	classify_comment_blocks(&mut lines);

	let uses_tabs = lines.iter().any(|line| line.indent_text.contains('\t'));
	let mixed_indentation = uses_tabs && lines.iter().any(|line| line.indent_text.contains("    "));

	LexedFile {
		language: profile.language,
		profile,
		lines,
		uses_tabs,
		mixed_indentation,
		unterminated,
	}
}

/// An open string literal.
#[derive(Debug, Clone)]
struct Literal {
	/// The delimiter that closes it.
	end: String,
	/// Whether backslash escapes apply.
	escapes: bool,
	/// Whether it may span physical lines.
	multiline: bool,
	/// The interpolation style this literal uses, when it embeds expressions.
	///
	/// The style rather than an open interpolation, because nothing is open until an
	/// opener is actually seen.
	interpolation: Option<InterpolationStyle>,
	/// Delimiter-like escape sequences that must not be read as a closer.
	extra_escapes: &'static [&'static str],
	/// The line it opened on.
	opened_at: usize,
}

/// An open interpolation inside a literal.
#[derive(Debug, Clone)]
struct OpenInterpolation {
	/// Brace depth, starting at 1.
	depth: usize,
	/// The interpolation style, needed to know the closer.
	style: InterpolationStyle,
	/// How many literals were open when this interpolation began.
	///
	/// This is what makes nesting unambiguous: a literal opened *inside* the interpolation
	/// sits above this depth and must be scanned as a literal, while the interpolation's own
	/// text sits at this depth and must be scanned as code. Without it, the inner quote in
	/// `"${describe("inner")}"` is treated as another opener and the stack never unwinds.
	literal_depth: usize,
}

/// The style of the currently open interpolation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InterpolationStyle {
	/// Closes with `}`.
	Brace,
	/// Closes with `#}`.
	HashBrace,
}

/// An open block comment.
#[derive(Debug, Clone)]
struct BlockComment {
	/// The delimiter that closes it.
	end: &'static str,
	/// Nesting depth, which only exceeds 1 for languages that nest comments.
	depth: usize,
	/// The line it opened on.
	opened_at: usize,
}

/// An open heredoc waiting for its terminator line.
#[derive(Debug, Clone)]
struct Heredoc {
	/// The identifier that terminates the body.
	delimiter: String,
	/// Whether a leading tab may precede the terminator.
	strip_tabs: bool,
	/// The line it opened on.
	opened_at: usize,
	/// Whether the current line is the first of the body.
	first_line: bool,
}

/// An open regex literal.
#[derive(Debug, Clone)]
struct Regex {
	/// Whether the scan is inside a `[...]` character class.
	in_class: bool,
	/// The line it opened on.
	opened_at: usize,
}

/// Accumulates one physical line while scanning.
#[derive(Debug)]
struct LineBuilder {
	number: usize,
	raw: String,
	masked: String,
	comment: Option<String>,
	comment_column: Option<usize>,
	is_doc_comment: bool,
	literal_only: bool,
	saw_code: bool,
}

impl LineBuilder {
	/// Starts a new line.
	fn new(number: usize) -> Self {
		Self {
			number,
			raw: String::new(),
			masked: String::new(),
			comment: None,
			comment_column: None,
			is_doc_comment: false,
			literal_only: false,
			saw_code: false,
		}
	}

	/// Records a character that is part of the code.
	fn push_code(&mut self, character: char) {
		self.raw.push(character);
		self.masked.push(character);

		if !character.is_whitespace() {
			self.saw_code = true;
		}
	}

	/// Records a character whose content must be hidden from later passes.
	fn push_masked(&mut self, character: char) {
		self.raw.push(character);

		if character.is_whitespace() {
			self.masked.push(character);
		} else {
			self.masked.push(' ');
		}
	}

	/// Records the first comment on the line.
	fn push_comment(&mut self, body: char) {
		self.raw.push(body);

		if self.comment.is_none() {
			self.comment_column = Some(self.raw.chars().count() - 1);
			self.comment = Some(String::new());
		}

		if let Some(comment) = self.comment.as_mut() {
			comment.push(body);
		}
	}

	/// Records a character that belongs to a literal's content.
	fn push_literal(&mut self, character: char) {
		self.raw.push(character);
		self.masked.push(if character.is_whitespace() {
			character
		} else {
			' '
		});
	}

	/// Finishes the line.
	fn build(self, profile: &LanguageProfile) -> LexedLine {
		let indent_text: String = self
			.raw
			.chars()
			.take_while(|character| character.is_whitespace())
			.collect();
		let indent = expanded_width(&indent_text);
		let code_start_column = self.raw.chars().count() - self.raw.trim_start().chars().count();

		let kind = classify_line(
			&self.masked,
			self.comment.as_deref(),
			self.literal_only,
			self.saw_code,
		);

		let mut line = LexedLine {
			number: self.number,
			text: self.raw,
			masked_code: self.masked,
			indent,
			indent_text,
			kind,
			code_start_column,
			trailing_comment_column: self.comment_column,
			comment_intent: self
				.comment
				.as_deref()
				.map(|body| classify(body, self.is_doc_comment))
				.or_else(|| (kind == LineKind::Literal).then_some(CommentIntent::Neutral)),
			decisions: Vec::new(),
			nesting: Vec::new(),
			logical_operators: 0,
			null_coalescing: 0,
			has_ternary: false,
			jumps: 0,
			parameter_span: 0,
			is_return: false,
		};

		if line.is_code() {
			apply_code_signals(&mut line, profile);
		}

		line
	}
}

/// The scanner's state.
struct Scanner {
	profile: LanguageProfile,
	lines: Vec<LexedLine>,
	/// Open string literals, innermost last.
	literals: Vec<Literal>,
	/// The interpolation currently open, if any.
	interpolation: Option<OpenInterpolation>,
	/// The open block comment, if any.
	block_comment: Option<BlockComment>,
	/// The open heredoc, if any.
	heredoc: Option<Heredoc>,
	/// The open regex, if any.
	regex: Option<Regex>,
	/// Whether a line comment is in progress.
	line_comment: bool,
	/// Lines still awaiting their closing construct.
	unterminated: Vec<Unterminated>,
	/// The previous significant character, used for the regex-versus-division decision.
	previous_code: Option<char>,
}

impl Scanner {
	/// Creates a scanner for `profile`.
	fn new(profile: LanguageProfile) -> Self {
		Self {
			profile,
			lines: Vec::new(),
			literals: Vec::new(),
			interpolation: None,
			block_comment: None,
			heredoc: None,
			regex: None,
			line_comment: false,
			unterminated: Vec::new(),
			previous_code: None,
		}
	}

	/// Reports constructs that never closed.
	fn finish(self) -> Vec<Unterminated> {
		self.unterminated
	}

	/// Runs the scan over `source`.
	fn run(&mut self, source: &str) -> Vec<LexedLine> {
		let characters: Vec<char> = source.chars().collect();
		let mut line = LineBuilder::new(1);
		let mut index = 0;

		while index < characters.len() {
			let character = characters[index];

			if character == '\n' {
				let number = line.number;
				let finished = std::mem::replace(&mut line, LineBuilder::new(number + 1));

				self.close_line(finished);
				index += 1;
				continue;
			}

			// A carriage return is either skipped (CRLF) or treated as a line break (CR).
			if character == '\r' {
				let next = characters.get(index + 1).copied();

				if next == Some('\n') {
					index += 1;
					continue;
				}

				let number = line.number;
				let finished = std::mem::replace(&mut line, LineBuilder::new(number + 1));

				self.close_line(finished);
				index += 1;
				continue;
			}

			// A new line that begins inside an open literal is literal content until the closer
			// is found, however many lines later. Recording that here is what keeps a Python
			// snippet embedded in a Rust string from being measured as indented Rust code.
			if self.inside_multiline_construct() {
				line.literal_only = true;
			}

			index = self.step(&characters, index, &mut line);
		}

		self.close_final_line(line);
		records_for_open_constructs(
			&mut self.unterminated,
			&self.literals,
			self.block_comment.as_ref(),
			self.heredoc.as_ref(),
			self.regex.as_ref(),
		);
		std::mem::take(&mut self.lines)
	}

	/// Closes the last line, unless a trailing newline already terminated it.
	///
	/// A file ending in a newline leaves an empty builder behind. Emitting it would report one
	/// more line than the file has, and would make every line count and density denominator
	/// slightly wrong.
	fn close_final_line(&mut self, line: LineBuilder) {
		if line.raw.is_empty() && !self.lines.is_empty() {
			return;
		}

		self.close_line(line);
	}

	/// Handles the construct that is currently open, or opens a new one.
	///
	/// Returns the index of the next character to consume.
	///
	/// # Dispatch order
	///
	/// Heredocs, line comments, and block comments are exclusive: nothing else can be open
	/// while one of them is. Literals and interpolations are not exclusive, because a literal
	/// may contain an interpolation which may itself contain further literals. They are
	/// therefore ordered by stack depth rather than by kind: whatever was opened most
	/// recently is what owns the current character. A literal opened *inside* an
	/// interpolation sits above it and is scanned first, which is what keeps
	/// `"${describe("inner")}"` from being read as two sibling strings.
	fn step(&mut self, characters: &[char], index: usize, line: &mut LineBuilder) -> usize {
		if self.heredoc.is_some() {
			return self.step_heredoc(characters, index, line);
		}

		if self.line_comment {
			line.push_comment(characters[index]);

			return index + 1;
		}

		if self.block_comment.is_some() {
			return self.step_block_comment(characters, index, line);
		}

		// A literal opened within the current interpolation outranks the interpolation.
		let literal_owns_position = match &self.interpolation {
			Some(interpolation) => self.literals.len() > interpolation.literal_depth,
			None => !self.literals.is_empty(),
		};

		if literal_owns_position {
			return self.step_literal(characters, index, line);
		}

		if self.interpolation.is_some() {
			return self.step_interpolation(characters, index, line);
		}

		if self.regex.is_some() {
			return self.step_regex(characters, index, line);
		}

		self.step_code(characters, index, line)
	}

	/// Handles a character in ordinary code position.
	///
	/// Each opener is checked before the character is emitted, so no delimiter ever leaks into
	/// the masked view and gets read as code by a later pass.
	fn step_code(&mut self, characters: &[char], index: usize, line: &mut LineBuilder) -> usize {
		let rest = peek(characters, index);
		let character = characters[index];

		if let Some((open, end)) = self.profile.block_comment_at(&rest) {
			self.block_comment = Some(BlockComment {
				end,
				depth: 1,
				opened_at: line.number,
			});

			let length = open.chars().count();
			consume_comment(characters, index, length, line);

			return index + length;
		}

		if let Some(token) = self.profile.line_comment_at(&rest) {
			return self.open_line_comment(characters, index, &rest, token, line);
		}

		if let Some(start) = self.literal_at(&rest, characters, index) {
			let line_remainder = peek_line(characters, index, usize::MAX);

			return self.open_literal(start, &rest, &line_remainder, line, index);
		}

		if self.try_open_heredoc(&rest, line) {
			return index + self.heredoc_marker_length(&rest);
		}

		if character == '/'
			&& self.regex_allowed(characters, index)
			&& let Some(offset) = find_regex_close(&rest[1..])
		{
			consume_masked(characters, index, offset + 1, line);

			return index + offset + 1;
		}

		line.push_code(character);

		if !character.is_whitespace() {
			self.previous_code = Some(character);
		}

		index + 1
	}

	/// Opens a line comment and consumes its token.
	fn open_line_comment(
		&mut self,
		characters: &[char],
		index: usize,
		rest: &str,
		token: &str,
		line: &mut LineBuilder,
	) -> usize {
		self.line_comment = true;
		line.is_doc_comment = self.profile.is_documentation_comment(rest);

		let length = token.chars().count();
		consume_comment(characters, index, length, line);

		index + length
	}

	/// Handles a character inside a string literal.
	/// Handles a character inside a string literal.
	///
	/// The checks are ordered by specificity: a delimiter-like escape must beat the closer, or
	/// Nix's `''$` closes the literal early; an escape must beat interpolation, or a `\$` opens
	/// one; and interpolation must beat ordinary content.
	fn step_literal(&mut self, characters: &[char], index: usize, line: &mut LineBuilder) -> usize {
		let character = characters[index];
		let Some(literal) = self.literals.last().cloned() else {
			return index + 1;
		};

		let rest = peek(characters, index);

		if let Some(extra) = literal
			.extra_escapes
			.iter()
			.find(|extra| rest.starts_with(**extra))
		{
			let length = extra.chars().count();
			consume_masked(characters, index, length, line);

			return index + length;
		}

		if rest.starts_with(&literal.end) {
			let length = literal.end.chars().count();
			consume_masked(characters, index, length, line);
			self.literals.pop();

			return index + length;
		}

		if literal.escapes && character == '\\' {
			// A backslash immediately before a newline is a line continuation, not an escape of
			// the newline. Consuming both would swallow the line break and merge the continuation
			// into the declaration line, which is how an embedded Python snippet ended up being
			// measured as indented Rust code.
			if characters.get(index + 1) == Some(&'\n') {
				line.push_masked(character);

				return index + 1;
			}

			consume_masked(characters, index, 2, line);

			return index + 2;
		}

		if let Some(style) = literal.interpolation {
			// Interpolation only opens when it also closes ahead of here. Requiring a closer is
			// what stops a lone brace in ordinary prose from swallowing the rest of the file.
			if let Some(length) = interpolation_opener(&rest, style)
				.filter(|length| interpolation_has_close(characters, index + length, style))
			{
				consume_masked(characters, index, length, line);
				self.interpolation = Some(OpenInterpolation {
					depth: 1,
					style,
					literal_depth: self.literals.len(),
				});

				return index + length;
			}
		}

		line.push_literal(character);
		line.literal_only = true;

		index + 1
	}

	/// Handles a character inside an interpolation.
	fn step_interpolation(
		&mut self,
		characters: &[char],
		index: usize,
		line: &mut LineBuilder,
	) -> usize {
		let character = characters[index];
		let rest = peek(characters, index);
		let Some(interpolation) = self.interpolation.clone() else {
			return index + 1;
		};

		if let Some(rule) = self.literal_at(&rest, characters, index) {
			let line_remainder = peek_line(characters, index, usize::MAX);

			return self.open_literal(rule, &rest, &line_remainder, line, index);
		}

		match (character, interpolation.style) {
			('{', InterpolationStyle::Brace) => {
				line.push_masked(character);
				self.interpolation = Some(OpenInterpolation {
					depth: interpolation.depth + 1,
					style: interpolation.style,
					literal_depth: interpolation.literal_depth,
				});

				index + 1
			}
			('}', InterpolationStyle::Brace) => {
				line.push_masked(character);

				if interpolation.depth <= 1 {
					self.interpolation = None;
				} else {
					self.interpolation = Some(OpenInterpolation {
						depth: interpolation.depth - 1,
						style: interpolation.style,
						literal_depth: interpolation.literal_depth,
					});
				}

				index + 1
			}
			('}', InterpolationStyle::HashBrace) => {
				line.push_masked(character);
				self.interpolation = None;

				index + 1
			}
			_ => {
				// Interpolated expressions are code: they are masked so they cannot inflate
				// complexity counts, but they still mark the line as containing code.
				line.push_masked(character);
				line.literal_only = false;
				line.saw_code = line.saw_code || !character.is_whitespace();

				index + 1
			}
		}
	}

	/// Handles a character inside a block comment.
	///
	/// Nestable languages raise the depth on each opener so the matching number of closers is
	/// required; a scanner that ignored this would end Rust's `/* /* */ */` at the first `*/`
	/// and read the remainder as code.
	fn step_block_comment(
		&mut self,
		characters: &[char],
		index: usize,
		line: &mut LineBuilder,
	) -> usize {
		let rest = peek(characters, index);
		let Some(comment) = self.block_comment.clone() else {
			return index + 1;
		};

		if rest.starts_with(comment.end) {
			let length = comment.end.chars().count();
			consume_comment(characters, index, length, line);

			self.block_comment = if comment.depth <= 1 {
				None
			} else {
				Some(BlockComment {
					depth: comment.depth - 1,
					..comment
				})
			};

			return index + length;
		}

		if let Some(open) = self.nested_comment_opener(&rest) {
			self.block_comment = Some(BlockComment {
				depth: comment.depth + 1,
				..comment
			});

			let length = open.chars().count();
			consume_comment(characters, index, length, line);

			return index + length;
		}

		line.push_comment(characters[index]);
		line.literal_only = true;

		index + 1
	}

	/// Returns a nested comment opener that `rest` starts with, if nesting is enabled.
	fn nested_comment_opener(&self, rest: &str) -> Option<&'static str> {
		if !self.profile.nestable_comments {
			return None;
		}

		self.profile
			.block_comments
			.iter()
			.find(|(open, _)| rest.starts_with(open))
			.map(|(open, _)| *open)
	}

	/// Handles a character inside a regex literal.
	fn step_regex(&mut self, characters: &[char], index: usize, line: &mut LineBuilder) -> usize {
		let character = characters[index];
		let Some(regex) = self.regex.clone() else {
			return index + 1;
		};

		if character == '\\' {
			line.push_masked(character);

			if let Some(escaped) = characters.get(index + 1) {
				line.push_masked(*escaped);
			}

			return index + 2;
		}

		// A slash inside `[...]` is a member of the class, not the terminator.
		if character == '[' {
			self.regex = Some(Regex {
				in_class: true,
				..regex
			});

			line.push_masked(character);

			return index + 1;
		}

		if character == ']' && regex.in_class {
			self.regex = Some(Regex {
				in_class: false,
				..regex
			});

			line.push_masked(character);

			return index + 1;
		}

		if character == '/' && !regex.in_class {
			line.push_masked(character);
			self.regex = None;

			return index + 1;
		}

		line.push_masked(character);
		line.literal_only = false;

		index + 1
	}

	/// Handles a character while a heredoc body is open.
	fn step_heredoc(&mut self, characters: &[char], index: usize, line: &mut LineBuilder) -> usize {
		let Some(heredoc) = self.heredoc.clone() else {
			return index + 1;
		};

		let character = characters[index];

		if heredoc.first_line {
			line.push_code(character);

			if !character.is_whitespace() {
				self.heredoc = Some(Heredoc {
					first_line: false,
					..heredoc
				});
			}

			return index + 1;
		}

		line.push_literal(character);
		line.literal_only = true;

		index + 1
	}

	/// Closes a finished line, applying heredoc termination and literal recovery.
	fn close_line(&mut self, builder: LineBuilder) {
		let raw = builder.raw.clone();

		// A line comment always ends at the line break.
		if self.line_comment {
			self.line_comment = false;
		}

		if let Some(heredoc) = self.heredoc.clone()
			&& !heredoc.first_line
		{
			let candidate = if heredoc.strip_tabs {
				raw.trim_start_matches('\t').trim_end()
			} else {
				raw.trim()
			};

			if candidate == heredoc.delimiter {
				self.heredoc = None;
			}
		}

		// A single-line literal that reached the line break cannot legally continue, and
		// carrying it would corrupt every following line.
		if let Some(literal) = self.literals.last().cloned()
			&& !literal.multiline
		{
			self.literals.pop();
		}

		if let Some(regex) = self.regex.clone() {
			self.unterminated.push(Unterminated {
				kind: UnterminatedKind::Regex,
				line: regex.opened_at,
				delimiter: "/".to_string(),
			});

			self.regex = None;
		}

		// A regex literal only lives on one line, so it is closed by the line break rather
		// than reported as an error when the pattern simply had no closing slash.
		self.previous_code = None;

		let line = builder.build(&self.profile);
		self.lines.push(line);
	}

	/// Returns the string rule whose opener begins `rest`, allowing for prefixes.
	///
	/// Rust's `r#"…"#` needs special handling: after the `r` prefix comes a run of hashes and
	/// only then the quote, so the ordinary "prefix followed by a delimiter" check cannot see
	/// it. The hashes are resolved into the closing delimiter by [`resolve_literal_end`].
	fn literal_at(&self, rest: &str, characters: &[char], index: usize) -> Option<LiteralStart> {
		if let Some(rule) = self.profile.string_at(rest) {
			return Some(LiteralStart {
				rule,
				opener_length: rule.open.chars().count(),
				hashes: 0,
			});
		}

		let character = characters.get(index)?;

		// A hashed raw string: prefix, then hashes, then a quote.
		if self.profile.hashed_raw_strings && self.profile.raw_string_prefix == Some(*character) {
			let after_prefix: String = characters[index + 1..].iter().collect();
			let hashes = after_prefix.chars().take_while(|c| *c == '#').count();

			if after_prefix.chars().nth(hashes) == Some('"') {
				return Some(LiteralStart {
					rule: self.profile.string_at("\"")?,
					// The prefix, the hashes, and the opening quote are all consumed.
					opener_length: 1 + hashes + 1,
					hashes,
				});
			}
		}

		// A prefixed literal: Dart's `r"…"`, Python's `f"…"`, Rust's `b"…"`.
		if !self.profile.string_prefixes.contains(character) {
			return None;
		}

		let remainder: String = characters[index + 1..].iter().collect();
		let rule = self.profile.string_at(&remainder)?;

		Some(LiteralStart {
			rule,
			opener_length: 1 + rule.open.chars().count(),
			hashes: 0,
		})
	}

	/// Opens a literal and returns the next index.
	fn open_literal(
		&mut self,
		start: LiteralStart,
		rest: &str,
		line_remainder: &str,
		line: &mut LineBuilder,
		index: usize,
	) -> usize {
		let LiteralStart {
			rule,
			opener_length,
			hashes,
		} = start;
		let end = closing_delimiter(rule, hashes);

		// Whether a short literal closes on this line can only be answered from the whole line, so the
		// caller passes the remainder of the line rather than the bounded peek window used for
		// delimiter matching.
		let body = line_remainder.get(rule.open.len()..).unwrap_or_default();
		let has_close = hashes > 0
			|| find_literal_close(body, &end, rule.escapes, rule.extra_escapes).is_some();

		// A backslash at end of line continues a string in most languages, so the literal really
		// does span lines even though it was declared single-line. Rust and Shell both rely on
		// this, and rejecting it would read the following lines as code.
		let continues = body.trim_end().ends_with('\\');

		// A single-line literal with no closer and no continuation is a mis-read rather than an
		// unterminated literal — an apostrophe in a comment, most often — so it is emitted as code
		// and never opened.
		if !rule.multiline && hashes == 0 && !has_close && !continues {
			line.push_code(rest.chars().next().unwrap_or('"'));

			return index + 1;
		}

		// A literal opened inside an interpolation records that nesting so the dispatcher can
		// tell which of the two owns the current character.
		self.literals.push(Literal {
			end,
			escapes: rule.escapes,
			multiline: rule.multiline || hashes > 0 || continues,
			interpolation: rule.interpolates.then(|| {
				// Every interpolating language in the profile table has a style; defaulting to
				// a braced form keeps an unconfigured language from silently losing
				// interpolation handling.
				match self
					.profile
					.interpolation
					.unwrap_or(Interpolation::Dollar { braced: true })
				{
					Interpolation::Hash => InterpolationStyle::HashBrace,
					Interpolation::Dollar { .. }
					| Interpolation::DollarBrace
					| Interpolation::Brace => InterpolationStyle::Brace,
				}
			}),
			extra_escapes: rule.extra_escapes,
			opened_at: line.number,
		});

		// The whole opener — prefix, hashes, and delimiters — is masked so that none of it can
		// be read as code.
		for offset in 0..opener_length {
			if let Some(opener) = rest.chars().nth(offset) {
				line.push_masked(opener);
			}
		}

		index + opener_length
	}

	/// Opens a heredoc if `rest` begins one.
	fn try_open_heredoc(&mut self, rest: &str, line: &mut LineBuilder) -> bool {
		let Some(syntax) = self.profile.heredoc else {
			return false;
		};

		if !rest.starts_with(syntax.marker) {
			return false;
		}

		let after = &rest[syntax.marker.len()..];
		let after = if syntax.allows_dash {
			after.strip_prefix('-').unwrap_or(after)
		} else {
			after
		};
		let after = if syntax.allows_tilde {
			after.strip_prefix('~').unwrap_or(after)
		} else {
			after
		};
		let after = after.trim_start();

		let delimiter: String = match after.chars().next() {
			Some(quote @ ('"' | '\'')) => after[1..].chars().take_while(|c| *c != quote).collect(),
			_ => {
				after
					.chars()
					.take_while(|c| c.is_alphanumeric() || *c == '_')
					.collect()
			}
		};

		if delimiter.is_empty() {
			return false;
		}

		self.heredoc = Some(Heredoc {
			delimiter,
			strip_tabs: syntax.allows_dash,
			opened_at: line.number,
			first_line: true,
		});

		true
	}

	/// Whether a construct that can span lines is currently open.
	///
	/// Used to mark continuation lines as literal content rather than indented code. Only
	/// constructs that genuinely continue are considered: a single-line literal never survives a
	/// line break, so it cannot make the next line literal.
	fn inside_multiline_construct(&self) -> bool {
		self.literals.iter().any(|literal| literal.multiline)
			|| self.block_comment.is_some()
			|| self
				.heredoc
				.as_ref()
				.is_some_and(|heredoc| !heredoc.first_line)
	}

	/// The length of the heredoc marker that was just accepted.
	fn heredoc_marker_length(&self, rest: &str) -> usize {
		let Some(syntax) = self.profile.heredoc else {
			return 0;
		};

		let after = &rest[syntax.marker.len()..];
		let stripped = if syntax.allows_dash {
			after.strip_prefix('-').unwrap_or(after)
		} else {
			after
		};
		let stripped = if syntax.allows_tilde {
			stripped.strip_prefix('~').unwrap_or(stripped)
		} else {
			stripped
		};
		let delimiter = match stripped.trim_start().chars().next() {
			Some(quote @ ('"' | '\'')) => {
				let body: String = stripped.trim_start()[1..]
					.chars()
					.take_while(|c| *c != quote)
					.collect();

				format!("{quote}{body}{quote}")
			}
			_ => {
				stripped
					.trim_start()
					.chars()
					.take_while(|c| c.is_alphanumeric() || *c == '_')
					.collect()
			}
		};

		syntax.marker.len() + (rest.len() - after.len()) + delimiter.len()
	}

	/// Whether a `/` at `index` begins a regex rather than being division.
	fn regex_allowed(&self, characters: &[char], index: usize) -> bool {
		if !self.profile.regex_literals {
			return false;
		}

		// A regex in operand position is a literal; after a value it is division. Without
		// this, `/` in `total / count` would open a literal and hide the rest of the line.
		let previous = self.previous_code.or_else(|| {
			characters[..index]
				.iter()
				.rev()
				.find(|character| !character.is_whitespace())
				.copied()
		});

		previous.is_none_or(|character| self.profile.regex_allowed_after(character))
	}
}

/// Re-classifies comment intent over whole comment blocks.
///
/// A block is a run of adjacent comment or literal lines at the same indentation, which is how a
/// multi-line `//` comment and a `/* ... */` body both appear. Grouping them means the reasoning
/// in the opening line is still visible when a later line is weighed against it.
///
/// Block comment bodies are always classified as documentation when they opened with `/**`, and
/// otherwise judged as one unit; a run whose lines disagree is left as whatever the whole-block
/// verdict is, so the score no longer depends on where the author wrapped their text.
fn classify_comment_blocks(lines: &mut [LexedLine]) {
	let mut index = 0;

	while index < lines.len() {
		let starts_block = lines
			.get(index)
			.is_some_and(|line| line.is_comment() || line.is_trailing_comment_only());

		if !starts_block {
			index += 1;
			continue;
		}

		// Extend the run while lines stay comment-ish and keep the same indentation, so a blank
		// line or a dedent correctly ends the block.
		let indent = lines.get(index).map_or(0, |line| line.indent);
		let mut end = index + 1;

		while let Some(candidate) = lines.get(end) {
			let continues = (candidate.is_comment() || candidate.is_trailing_comment_only())
				&& candidate.indent == indent;

			if !continues {
				break;
			}

			end += 1;
		}

		let is_doc = lines.get(index).and_then(|line| line.comment_intent)
			== Some(CommentIntent::Documentation);

		let body = lines
			.get(index..end)
			.unwrap_or_default()
			.iter()
			.map(LexedLine::comment_body)
			.collect::<Vec<_>>()
			.join("\n");

		let block_intent = classify(&body, is_doc);

		for line in lines.get_mut(index..end).unwrap_or_default() {
			line.comment_intent = Some(block_intent);
		}

		index = end;
	}
}

/// Records every construct that is still open at end of file.
fn records_for_open_constructs(
	unterminated: &mut Vec<Unterminated>,
	literals: &[Literal],
	block_comment: Option<&BlockComment>,
	heredoc: Option<&Heredoc>,
	regex: Option<&Regex>,
) {
	for literal in literals {
		unterminated.push(Unterminated {
			kind: UnterminatedKind::Literal,
			line: literal.opened_at,
			delimiter: literal.end.clone(),
		});
	}

	if let Some(comment) = block_comment {
		unterminated.push(Unterminated {
			kind: UnterminatedKind::BlockComment,
			line: comment.opened_at,
			delimiter: comment.end.to_string(),
		});
	}

	if let Some(heredoc) = heredoc {
		unterminated.push(Unterminated {
			kind: UnterminatedKind::Heredoc,
			line: heredoc.opened_at,
			delimiter: heredoc.delimiter.clone(),
		});
	}

	if let Some(regex) = regex {
		unterminated.push(Unterminated {
			kind: UnterminatedKind::Regex,
			line: regex.opened_at,
			delimiter: "/".to_string(),
		});
	}
}

/// Returns the character length of an interpolation opener at the start of `rest`.
///
/// The style alone determines this: a braced interpolation opens on `${`, a hash
/// interpolation on `#{`, and a bare-brace interpolation on `{`. A bare `$name` never opens
/// one, because it cannot contain an expression and therefore cannot nest.
fn interpolation_opener(rest: &str, style: InterpolationStyle) -> Option<usize> {
	match style {
		InterpolationStyle::HashBrace => rest.starts_with("#{").then_some(2),
		InterpolationStyle::Brace => {
			if rest.starts_with("${") {
				Some(2)
			} else if rest.starts_with("{{") {
				// An escaped literal brace in a Python f-string or C# interpolation.
				None
			} else {
				rest.starts_with('{').then_some(1)
			}
		}
	}
}

/// Returns true when the interpolation opened at `start` closes within the file.
///
/// Requiring a closer is what prevents an ordinary brace in prose or a lone `$` from being
/// read as interpolation and swallowing the rest of the file.
fn interpolation_has_close(characters: &[char], start: usize, style: InterpolationStyle) -> bool {
	let mut depth = 0;
	let mut index = start;

	// The search is bounded to a few thousand characters: an interpolation that has not
	// closed by then is far more likely to be a mis-read than a genuine giant expression.
	let limit = (start + 4096).min(characters.len());

	while index < limit {
		match (characters[index], style) {
			('\\', _) => {
				index += 2;
				continue;
			}
			('{', InterpolationStyle::Brace) => depth += 1,
			('}', InterpolationStyle::Brace) => {
				depth -= 1;

				if depth <= 0 {
					return true;
				}
			}
			('}', InterpolationStyle::HashBrace) => return true,
			_ => {}
		}

		index += 1;
	}

	false
}

/// Finds a literal's closer, returning the offset of the closer's end.
fn find_literal_close(
	text: &str,
	end: &str,
	escapes: bool,
	extra_escapes: &[&str],
) -> Option<usize> {
	let characters: Vec<char> = text.chars().collect();
	let mut index = 0;

	while index < characters.len() {
		let rest = peek(&characters, index);

		if let Some(extra) = extra_escapes.iter().find(|extra| rest.starts_with(**extra)) {
			index += extra.chars().count();
			continue;
		}

		if rest.starts_with(end) {
			return Some(index + end.chars().count());
		}

		if escapes && characters[index] == '\\' {
			index += 2;
			continue;
		}

		index += 1;
	}

	None
}

/// Finds a regex literal's closing slash, returning the length including it.
fn find_regex_close(text: &str) -> Option<usize> {
	let characters: Vec<char> = text.chars().collect();
	let mut index = 0;
	let mut in_class = false;

	while index < characters.len() {
		match characters[index] {
			'\\' => {
				index += 2;
				continue;
			}
			'[' => in_class = true,
			']' => in_class = false,
			'/' if !in_class => return Some(index + 1),
			'\n' => return None,
			_ => {}
		}

		index += 1;
	}

	None
}

/// The outcome of recognizing a literal's opening.
#[derive(Debug, Clone, Copy)]
struct LiteralStart {
	/// The rule describing the literal's delimiters and escapes.
	rule: &'static StringRule,
	/// How many characters the whole opener spans, including any prefix and hashes.
	opener_length: usize,
	/// How many hashes the opener carried, for Rust-style raw strings.
	hashes: usize,
}

/// Builds the delimiter that closes a literal opened with `hashes` hashes.
///
/// Rust's `r#"…"#` is why this cannot be a constant: the closer's hash count must match the
/// opener's exactly, so it can only be known once the opener has been read.
fn closing_delimiter(rule: &StringRule, hashes: usize) -> String {
	if hashes == 0 {
		rule.close.to_string()
	} else {
		format!("\"{}", "#".repeat(hashes))
	}
}

/// Records `length` characters starting at `index` as masked content.
///
/// Masking hides literal and comment text from later passes while keeping columns aligned, so
/// this is the single place that rule is applied when consuming a run rather than a character.
fn consume_masked(characters: &[char], index: usize, length: usize, line: &mut LineBuilder) {
	for offset in 0..length {
		if let Some(character) = characters.get(index + offset) {
			line.push_masked(*character);
		}
	}
}

/// Records `length` characters starting at `index` as comment text.
fn consume_comment(characters: &[char], index: usize, length: usize, line: &mut LineBuilder) {
	for offset in 0..length {
		if let Some(character) = characters.get(index + offset) {
			line.push_comment(*character);
		}
	}
}

/// The number of characters a delimiter peek needs.
///
/// The longest opener in any profile is three characters (a triple quote, `<<<`, or `--[[`), and the
/// longest single-token line comment is two. Six leaves room for a hashed raw-string prefix without
/// truncating a match.
const PEEK: usize = 6;

/// Returns up to the next [`PEEK`] characters, starting at `index`.
///
/// Scanning must not allocate per character. Every step used to collect the whole remaining file
/// into a `String` purely to test a two-character delimiter, which made the scanner quadratic in
/// file size — a seven-thousand-line file did not finish within a minute. A bounded window answers
/// the same questions in constant work per character.
fn peek(characters: &[char], index: usize) -> String {
	characters.iter().skip(index).take(PEEK).collect()
}

/// Returns characters from `index` up to the next newline, capped at `limit`.
///
/// Used where a check must reason about the rest of the line rather than the rest of the file, such
/// as finding a regex literal's closing slash.
fn peek_line(characters: &[char], index: usize, limit: usize) -> String {
	characters
		.iter()
		.skip(index)
		.take_while(|character| **character != '\n')
		.take(limit)
		.collect()
}

/// Assigns a kind to a finished line.
///
/// Comment presence is checked before the literal flag because a line inside a block comment
/// carries comment text, and that is more informative than the fact that its content was
/// masked. Only a literal that produced no comment — a line inside a multi-line string — is
/// classified as literal content.
fn classify_line(
	masked: &str,
	comment: Option<&str>,
	literal_only: bool,
	saw_code: bool,
) -> LineKind {
	if comment.is_some() {
		return if masked.trim().is_empty() {
			LineKind::Comment
		} else {
			LineKind::CodeWithComment
		};
	}

	if literal_only && !saw_code {
		return LineKind::Literal;
	}

	if masked.trim().is_empty() {
		return LineKind::Blank;
	}

	LineKind::Code
}

/// Populates a line's complexity signals from its masked code.
fn apply_code_signals(line: &mut LexedLine, profile: &LanguageProfile) {
	let masked = line.masked_code.clone();
	let words = tokenize_words(&masked);
	let joined = words.join(" ");

	for keyword in profile.decision_keywords {
		for _ in 0..count_keyword(&joined, keyword) {
			line.decisions.push((*keyword).to_string());
		}
	}

	for keyword in profile.nesting_keywords {
		for _ in 0..count_keyword(&joined, keyword) {
			line.nesting.push((*keyword).to_string());
		}
	}

	line.logical_operators = profile
		.logical_operators
		.iter()
		.map(|operator| masked.matches(operator).count())
		.sum();

	line.null_coalescing = profile
		.null_coalescing
		.iter()
		.map(|operator| masked.matches(operator).count())
		.sum();

	line.has_ternary = profile.ternary_operator.is_some_and(|operator| {
		// A ternary needs a `:` after the `?` on the same line; otherwise it is a nullable
		// type annotation, an optional parameter marker, or a map literal.
		matches!(
			(masked.find(operator), masked.rfind(':')),
			(Some(question), Some(colon)) if colon > question
		)
	});

	line.jumps = profile
		.jump_keywords
		.iter()
		.map(|keyword| count_keyword(&joined, keyword))
		.sum();

	line.is_return = joined
		.split_whitespace()
		.any(|word| word == "return" || word == "yield");
}

/// Splits code into identifier-like words so multi-word keywords stay adjacent.
fn tokenize_words(code: &str) -> Vec<String> {
	let mut words = Vec::new();
	let mut current = String::new();

	for character in code.chars() {
		if character.is_alphanumeric() || character == '_' || character == '$' || character == '!' {
			current.push(character);
		} else if !current.is_empty() {
			words.push(std::mem::take(&mut current));
		}
	}

	if !current.is_empty() {
		words.push(current);
	}

	words
}

/// Counts occurrences of a keyword in a joined word stream.
fn count_keyword(joined: &str, keyword: &str) -> usize {
	if keyword.contains(' ') {
		return joined.matches(keyword).count();
	}

	joined
		.split_whitespace()
		.filter(|word| *word == keyword)
		.count()
}

/// Expands leading whitespace to a column width, counting tabs as four columns.
fn expanded_width(indent: &str) -> usize {
	indent.chars().fold(0, |width, character| {
		if character == '\t' {
			width + 4
		} else {
			width + 1
		}
	})
}

/// Records multi-line parameter lists on the lines that open them.
///
/// This runs over the masked view, so parentheses inside strings and comments can no longer
/// inflate a parameter count.
pub fn resolve_parameter_spans(lines: &mut [LexedLine], profile: &LanguageProfile) {
	let Some(open) = profile.parameter_list_start else {
		return;
	};

	let depths: Vec<(usize, usize)> = lines
		.iter()
		.map(|line| {
			(
				line.masked_code.matches(open).count(),
				line.masked_code.matches(')').count(),
			)
		})
		.collect();

	let mut index = 0;

	while index < lines.len() {
		let (opens, closes) = depths.get(index).copied().unwrap_or((0, 0));

		if opens <= closes {
			index += 1;
			continue;
		}

		let mut depth = opens as isize - closes as isize;
		let mut span = 1;
		let mut cursor = index;

		while cursor + 1 < lines.len() {
			cursor += 1;
			span += 1;

			let (opens, closes) = depths.get(cursor).copied().unwrap_or((0, 0));
			depth += opens as isize - closes as isize;

			if depth <= 0 {
				break;
			}
		}

		if let Some(line) = lines.get_mut(index) {
			line.parameter_span = span;
		}

		index += 1;
	}
}
