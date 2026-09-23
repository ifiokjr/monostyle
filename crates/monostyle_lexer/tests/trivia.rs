//! Adversarial tests for the scanner's comment and literal handling.
//!
//! This is the heaviest test suite in the repository, and deliberately so: an error here
//! does not produce a slightly wrong number, it produces a confidently wrong verdict. A
//! `?` mis-counted inside a Dart string becomes a phantom ternary; a `/*` inside a Shell
//! heredoc becomes phantom nesting.
//!
//! Every case in the failure-mode table in `ARCHITECTURE.md` has a test here, and each one
//! asserts on the observable consequence — the masked code, the line kinds, or the
//! reported unterminated constructs — rather than on internal state, so the tests keep
//! their meaning if the scanner is rewritten.

use monostyle_core::Language;
use monostyle_lexer::LineKind;
use monostyle_lexer::UnterminatedKind;
use monostyle_lexer::lex;

/// Counts every signal that adds an independent path through the code.
///
/// This deliberately mirrors what cyclomatic complexity counts — keywords, short-circuiting
/// operators, ternaries, optional chaining, and jumps — rather than only keyword decisions,
/// so a test asserting "one decision" means one path and not one keyword.
fn decisions(source: &str, language: Language) -> usize {
	lex(source, language)
		.lines
		.iter()
		.map(|line| {
			line.decision_count()
				+ line.logical_operators
				+ line.null_coalescing
				+ line.jumps + usize::from(line.has_ternary)
		})
		.sum()
}

/// Returns the kinds of every line.
fn kinds(source: &str, language: Language) -> Vec<LineKind> {
	lex(source, language)
		.lines
		.iter()
		.map(|line| line.kind)
		.collect()
}

// ---------------------------------------------------------------------------
// Dart
// ---------------------------------------------------------------------------
#[test]
fn dart_triple_quoted_string_does_not_end_at_first_quote() {
	// The opening delimiter is three characters; a scanner that closes on the first `'`
	// reads the next two lines as code and invents decisions that are not there.
	let source = "\
var text = '''
  if (this is prose) { and so is this }
  for (var i = 0; i < 10; i++) {}
''';
var after = 1;
";

	let decisions = decisions(source, Language::Dart);

	// The only real decision point is the `for` inside the prose, which must NOT be counted.
	assert_eq!(
		decisions, 0,
		"prose inside a triple-quoted string produced decisions"
	);

	let kinds = kinds(source, Language::Dart);

	assert_eq!(kinds.first(), Some(&LineKind::Code));
	assert!(
		kinds.contains(&LineKind::Literal),
		"interior lines should be literal content"
	);
}

#[test]
fn dart_interpolation_hides_inner_quotes() {
	// `"${foo("x")}"` ends at the outer quote, not the inner one.
	let source = r#"var message = "value: ${describe("inner")}";
var real = 1;
"#;

	let lexed = lex(source, Language::Dart);

	assert!(
		lexed.is_clean(),
		"interpolation was not resolved: {:?}",
		lexed.unterminated
	);
	assert_eq!(
		lexed.lines.len(),
		2,
		"the inner quote ended the literal early"
	);

	// `inner` must not be readable as code.
	assert!(
		!lexed.lines[0].masked_code.contains("inner"),
		"interpolated literal content leaked into the masked view: {}",
		lexed.lines[0].masked_code
	);
}

#[test]
fn dart_question_mark_inside_string_is_not_a_ternary() {
	// The phantom-ternary case: `?` in a string must not add a path.
	let with_string = "var label = \"ready?\";\n";
	let with_ternary = "var label = ready ? \"yes\" : \"no\";\n";

	assert_eq!(
		decisions(with_string, Language::Dart),
		0,
		"a `?` inside a string is not a ternary"
	);
	assert_eq!(
		decisions(with_ternary, Language::Dart),
		1,
		"a real ternary is one decision"
	);
}

// ---------------------------------------------------------------------------
// Rust
// ---------------------------------------------------------------------------
#[test]
fn rust_nested_block_comments_close_at_the_matching_terminator() {
	// `/* /* */ */` closes at the second terminator. A scanner that closes at the first
	// reads the remainder as code and counts phantom decisions.
	let source = "\
/* outer /* inner */ still a comment: if x { } */
fn real() { if true { } }
";

	let lexed = lex(source, Language::Rust);
	let comment_line = &lexed.lines[0];

	assert_eq!(comment_line.kind, LineKind::Comment);
	assert!(
		comment_line.masked_code.trim().is_empty(),
		"comment content leaked into masked code: {}",
		comment_line.masked_code
	);

	// Exactly one real decision: the `if` in `real`.
	assert_eq!(lexed.total_decisions(), 1);
}

#[test]
fn rust_hashed_raw_string_uses_matching_hash_count() {
	// `r#"…"#` closes on `"#`, so a `"` inside does not end it, and a `#` alone does not
	// either.
	let source = "\
let sql = r#\"SELECT * FROM t WHERE a = \"quoted\" AND b = #1\"#;
let after = if true { 1 } else { 2 };
";

	let lexed = lex(source, Language::Rust);

	assert!(
		lexed.is_clean(),
		"raw string not closed: {:?}",
		lexed.unterminated
	);
	assert_eq!(lexed.lines.len(), 2, "the raw string ended early");

	// The `if`/`else` on line two is one decision; nothing from line one counts.
	assert_eq!(
		lexed.total_decisions(),
		1,
		"raw string content produced decisions"
	);
}

#[test]
fn rust_raw_string_with_double_hashes() {
	let source = "let sql = r##\"contains \"# but not \"##;\nlet x = 1;\n";

	let lexed = lex(source, Language::Rust);

	assert!(
		lexed.is_clean(),
		"double-hash raw string not closed: {:?}",
		lexed.unterminated
	);
	assert_eq!(lexed.lines.len(), 2);
}

#[test]
fn rust_apostrophe_in_lifetime_is_not_an_unterminated_string() {
	// `&'a str` is a lifetime, not a string. Treating it as one would swallow the line.
	let source = "fn borrow<'a>(value: &'a str) -> &'a str {\n\tif value.is_empty() { return value; }\n\tvalue\n}\n";

	let lexed = lex(source, Language::Rust);

	assert!(
		lexed.is_clean(),
		"a lifetime was read as an unterminated string: {:?}",
		lexed.unterminated
	);
	assert_eq!(
		lexed.total_decisions(),
		1,
		"the `if` should be the only decision"
	);
}

// ---------------------------------------------------------------------------
// JavaScript and TypeScript
// ---------------------------------------------------------------------------
#[test]
fn javascript_regex_is_not_division() {
	// In operand position `/` opens a regex; a scanner that ignores this hides the rest of
	// the line and loses real decisions.
	let source = "const matches = value.replace(/a\\/b/g, \"x\");\nif (matches) { go(); }\n";

	let lexed = lex(source, Language::JavaScript);

	assert!(
		lexed.is_clean(),
		"regex mishandled: {:?}",
		lexed.unterminated
	);
	assert_eq!(
		lexed.total_decisions(),
		1,
		"the `if` should be the only decision"
	);
}

#[test]
fn javascript_division_is_not_a_regex() {
	// After an identifier, `/` divides. Reading it as a regex would hide the rest of the
	// line and drop the real `if` that follows on the same line.
	let source = "const ratio = total / count;\nif (ratio > 1) { go(); }\n";

	let lexed = lex(source, Language::JavaScript);

	assert!(
		lexed.is_clean(),
		"division read as a regex: {:?}",
		lexed.unterminated
	);
	assert_eq!(lexed.total_decisions(), 1);
}

#[test]
fn javascript_regex_character_class_containing_slash() {
	// Inside `[...]`, `/` belongs to the class rather than terminating the literal.
	let source = "const re = /[/]/;\nif (re) { go(); }\n";

	let lexed = lex(source, Language::JavaScript);

	assert!(
		lexed.is_clean(),
		"character class mishandled: {:?}",
		lexed.unterminated
	);
	assert_eq!(lexed.total_decisions(), 1);
}

#[test]
fn typescript_template_literal_with_nested_interpolation() {
	// `${…}` may contain strings, braces, and further templates.
	let source = r#"const message = `outer ${ inner.map((x) => `nested ${x}`).join(",") } end`;
if (message) { log(message); }
"#;

	let lexed = lex(source, Language::TypeScript);

	assert!(
		lexed.is_clean(),
		"template literal mishandled: {:?}",
		lexed.unterminated
	);

	// The arrow function and the nested template contain no decision points, so the `if` is
	// the only one.
	assert_eq!(
		lexed.total_decisions(),
		1,
		"template contents produced decisions"
	);
}

#[test]
fn typescript_template_literal_spanning_lines() {
	let source = r"const query = `
  SELECT *
  FROM users
  WHERE id = ${id}
`;
if (query) { run(query); }
";

	let lexed = lex(source, Language::TypeScript);

	assert!(
		lexed.is_clean(),
		"multiline template not closed: {:?}",
		lexed.unterminated
	);
	assert_eq!(lexed.total_decisions(), 1);
}

#[test]
fn typescript_optional_chaining_counts_as_a_path() {
	let with_chain = "const name = user?.profile?.name;\n";
	let without = "const name = user.profile.name;\n";

	assert_eq!(
		decisions(with_chain, Language::TypeScript),
		2,
		"each `?.` adds a path"
	);
	assert_eq!(decisions(without, Language::TypeScript), 0);
}

// ---------------------------------------------------------------------------
// Python
// ---------------------------------------------------------------------------
#[test]
fn python_fstring_nested_braces_do_not_end_the_literal() {
	// `{…}` nests inside an f-string, and the inner expression may contain quotes.
	let source = "\
label = f\"value: {describe({'key': 'value'})} end\"
if label:
    pass
";

	let lexed = lex(source, Language::Python);

	assert!(
		lexed.is_clean(),
		"f-string mishandled: {:?}",
		lexed.unterminated
	);

	// Only the `if` is a decision; the dict literal and quotes must not leak.
	assert_eq!(
		lexed.total_decisions(),
		1,
		"f-string contents produced decisions"
	);
}

#[test]
fn python_fstring_doubled_braces_are_literal_braces() {
	// `{{` is an escaped literal brace, not an interpolation, so it must not open a context
	// that then swallows following lines.
	let source = "\
label = f\"{{literal}}\"
if label:
    pass
";

	let lexed = lex(source, Language::Python);

	assert!(
		lexed.is_clean(),
		"escaped braces mishandled: {:?}",
		lexed.unterminated
	);
	assert_eq!(lexed.total_decisions(), 1);
}

#[test]
fn python_triple_quoted_docstring_is_comment_like() {
	let source = "\
def handler(request):
    \"\"\"Handle a request.

    This prose mentions if and for and while but is not code.
    \"\"\"
    return request
";

	let lexed = lex(source, Language::Python);

	assert!(
		lexed.is_clean(),
		"docstring not closed: {:?}",
		lexed.unterminated
	);

	// `if`/`for`/`while` in the prose must not count.
	assert_eq!(
		lexed.total_decisions(),
		0,
		"docstring prose produced decisions"
	);
}

#[test]
fn python_single_quote_apostrophe_does_not_swallow_the_rest() {
	// A single-line literal with no closer on its line is a mis-read, not a literal.
	let source = "value = 'unterminated\nif value:\n    pass\n";

	let lexed = lex(source, Language::Python);

	assert_eq!(
		lexed.total_decisions(),
		1,
		"an unterminated single-quoted literal swallowed following code"
	);
}

// ---------------------------------------------------------------------------
// Shell
// ---------------------------------------------------------------------------
#[test]
fn shell_heredoc_body_is_not_code() {
	// The body is data until the delimiter line, even when it looks like shell.
	let source = "\
cat <<EOF
if [ -f \"$file\" ]; then
  echo \"found\"
fi
EOF
if [ -d \"$dir\" ]; then
  echo \"dir\"
fi
";

	let lexed = lex(source, Language::Shell);

	assert!(
		lexed.is_clean(),
		"heredoc not closed: {:?}",
		lexed.unterminated
	);

	// Two real `if` statements: one inside the heredoc body (which must not count) and one
	// after it (which must).
	assert_eq!(
		lexed.total_decisions(),
		1,
		"heredoc body produced decisions"
	);
}

#[test]
fn shell_quoted_heredoc_delimiter_disables_interpolation() {
	let source = "\
cat <<'EOF'
if [ -f \"$file\" ]; then echo yes; fi
EOF
echo done
";

	let lexed = lex(source, Language::Shell);

	assert!(
		lexed.is_clean(),
		"quoted heredoc not closed: {:?}",
		lexed.unterminated
	);
	assert_eq!(
		lexed.total_decisions(),
		0,
		"quoted heredoc body produced decisions"
	);
}

#[test]
fn shell_dash_heredoc_allows_indented_delimiter() {
	// `<<-EOF` permits a leading tab before the terminator.
	let source = "\
cat <<-EOF
\tif [ -f x ]; then echo yes; fi
\tEOF
echo done
";

	let lexed = lex(source, Language::Shell);

	assert!(
		lexed.is_clean(),
		"dash heredoc not closed: {:?}",
		lexed.unterminated
	);
	assert_eq!(lexed.total_decisions(), 0);
}

// ---------------------------------------------------------------------------
// Nix
// ---------------------------------------------------------------------------
#[test]
fn nix_indented_string_escape_sequences_do_not_close_it() {
	// `''$`, `'''`, and `''\` are escapes. A scanner that ignores them ends the string at the
	// first `''` and reads the rest as Nix code, inventing `if`/`let` decisions.
	let source = "\
{
  script = ''
    echo ''${HOME}
    if [ -f x ]; then echo yes; fi
  '';
  after = 1;
}
";

	let lexed = lex(source, Language::Nix);

	assert!(
		lexed.is_clean(),
		"indented string not closed: {:?}",
		lexed.unterminated
	);
	assert_eq!(
		lexed.total_decisions(),
		0,
		"indented string content produced decisions"
	);
}

#[test]
fn nix_quoted_string_with_escaped_interpolation() {
	let source = "\"literal \\${not.interpolated}\"\n";

	let lexed = lex(source, Language::Nix);

	assert!(
		lexed.is_clean(),
		"escaped interpolation mishandled: {:?}",
		lexed.unterminated
	);
}

#[test]
fn nix_real_interpolation_does_not_leak_into_masked_code() {
	let source = "\"{config.networking.hostName}\"\n";

	let lexed = lex(source, Language::Nix);

	assert!(
		lexed.is_clean(),
		"interpolation mishandled: {:?}",
		lexed.unterminated
	);
	assert!(
		!lexed.lines[0].masked_code.contains("hostName"),
		"interpolated expression leaked into masked code: {}",
		lexed.lines[0].masked_code
	);
}

// ---------------------------------------------------------------------------
// Ruby, Lua, and Go
// ---------------------------------------------------------------------------
#[test]
fn ruby_hash_interpolation_hides_contents() {
	let source = r#"message = "value: #{compute("inner")} end"
if message
  puts message
end
"#;

	let lexed = lex(source, Language::Ruby);

	assert!(
		lexed.is_clean(),
		"Ruby interpolation mishandled: {:?}",
		lexed.unterminated
	);
	assert_eq!(
		lexed.total_decisions(),
		1,
		"interpolated contents produced decisions"
	);
}

#[test]
fn ruby_end_blocks_are_tracked() {
	let source = "\
def handler(request)
  if request.nil?
    return nil
  end
  request
end
";

	let lexed = lex(source, Language::Ruby);

	assert!(
		lexed.is_clean(),
		"Ruby blocks not tracked: {:?}",
		lexed.unterminated
	);
	assert_eq!(lexed.total_decisions(), 1);
}

#[test]
fn lua_long_bracket_string_spans_lines() {
	let source = "\
local text = [[
if x then return y end
]]
local real = 1
";

	let lexed = lex(source, Language::Lua);

	assert!(
		lexed.is_clean(),
		"Lua long string not closed: {:?}",
		lexed.unterminated
	);
	assert_eq!(
		lexed.total_decisions(),
		0,
		"long string content produced decisions"
	);
}

#[test]
fn go_backtick_raw_string_preserves_braces_literally() {
	let source = "\
query := `
  if this were code { it would nest }
  for i := range items { }
`
if query != \"\" {
	run(query)
}
";

	let lexed = lex(source, Language::Go);

	assert!(
		lexed.is_clean(),
		"Go raw string not closed: {:?}",
		lexed.unterminated
	);
	assert_eq!(
		lexed.total_decisions(),
		1,
		"raw string content produced decisions"
	);
}

// ---------------------------------------------------------------------------
// Cross-language invariants
// ---------------------------------------------------------------------------
#[test]
fn unterminated_multiline_construct_is_reported_not_silently_ignored() {
	// An unclosed triple-quoted string is a real problem, and the scanner must say so rather
	// than quietly producing a score that looks fine.
	let source = "var text = '''\nnever closed\n";

	let lexed = lex(source, Language::Dart);

	assert!(!lexed.is_clean(), "an unclosed literal should be reported");
	assert_eq!(lexed.unterminated.len(), 1);
	assert_eq!(lexed.unterminated[0].kind, UnterminatedKind::Literal);
	assert_eq!(lexed.unterminated[0].line, 1);
}

#[test]
fn masked_code_never_contains_literal_or_comment_contents() {
	// The central invariant: across every language, no keyword that appears only inside a
	// string or comment may survive into the masked view.
	let cases: &[(Language, &str, &str)] = &[
		(Language::Dart, "var a = \"if for while\";", "if"),
		(Language::Rust, "let a = \"if for while\";", "if"),
		(Language::Python, "a = \"if for while\"", "if"),
		(Language::Go, "a := \"if for while\"", "if"),
		(Language::Ruby, "a = \"if for while\"", "if"),
		(Language::Shell, "a=\"if for while\"", "if"),
		(Language::Php, "$a = \"if for while\";", "if"),
		(Language::JavaScript, "const a = \"if for while\";", "if"),
		(
			Language::TypeScript,
			"const a: string = \"if for while\";",
			"if",
		),
		(Language::CSharp, "var a = \"if for while\";", "if"),
		(Language::Kotlin, "val a = \"if for while\"", "if"),
		(Language::Swift, "let a = \"if for while\"", "if"),
		(Language::Nix, "a = \"if for while\";", "if"),
		(Language::Java, "String a = \"if for while\";", "if"),
		(Language::C, "char *a = \"if for while\";", "if"),
		(Language::Cpp, "auto a = \"if for while\";", "if"),
		(Language::Lua, "local a = \"if for while\"", "if"),
		(Language::Scala, "val a = \"if for while\"", "if"),
		(Language::Elixir, "a = \"if for while\"", "if"),
		(Language::Haskell, "a = \"if for while\"", "if"),
		(Language::Mozjs, "const a = \"if for while\";", "if"),
		(Language::Tsx, "const a = \"if for while\";", "if"),
	];

	for (language, source, forbidden) in cases {
		let lexed = lex(source, *language);
		let masked = &lexed.lines[0].masked_code;

		assert!(
			!masked.contains(forbidden),
			"{language}: `{forbidden}` from a string literal leaked into masked code `{masked}`"
		);

		assert_eq!(
			lexed.total_decisions(),
			0,
			"{language}: string contents produced decisions"
		);
	}
}

#[test]
fn comment_keywords_never_produce_decisions() {
	// The same invariant for comments: a documented `if` is not a branch.
	let cases: &[(Language, &str, &str)] = &[
		(Language::Rust, "// if for while\nlet a = 1;", "if"),
		(Language::Python, "# if for while\na = 1", "if"),
		(Language::Shell, "# if for while\na=1", "if"),
		(Language::Ruby, "# if for while\na = 1", "if"),
		(Language::JavaScript, "// if for while\nconst a = 1;", "if"),
		(Language::Dart, "// if for while\nvar a = 1;", "if"),
		(Language::Go, "// if for while\na := 1", "if"),
		(Language::Nix, "# if for while\na = 1;", "if"),
	];

	for (language, source, forbidden) in cases {
		let lexed = lex(source, *language);
		let comment_line = lexed.lines.first().expect("a first line");

		assert_eq!(
			comment_line.kind,
			LineKind::Comment,
			"{language}: expected a comment line"
		);
		assert!(
			!comment_line.masked_code.contains(forbidden),
			"{language}: comment content leaked into masked code `{}`",
			comment_line.masked_code
		);
		assert_eq!(
			lexed.total_decisions(),
			0,
			"{language}: comment produced decisions"
		);
	}
}

#[test]
fn line_continuation_opens_a_multiline_literal() {
	// A backslash at end of line continues a string even though the literal was declared with
	// single quotes. Rejecting it reads the following lines as code — which is how a Python
	// snippet embedded in a Rust test was measured as indented Rust.
	let source = "\
let source = \"\\
    indented prose that is string content
\";
let after = 1;
";

	let lexed = lex(source, Language::Rust);

	assert!(
		lexed.is_clean(),
		"continued string not closed: {:?}",
		lexed.unterminated
	);

	// The interior line is literal content, not code, so it carries no decisions and is not
	// mistaken for indented statements.
	let interior = lexed
		.lines
		.iter()
		.find(|line| line.number == 2)
		.expect("line two");

	assert_eq!(
		interior.kind,
		LineKind::Literal,
		"a continued string's interior should be literal content, got {:?}",
		interior.kind
	);
	assert!(
		!interior.is_code(),
		"interior string content must not count as code"
	);
}

#[test]
fn multiline_string_interior_lines_are_not_code() {
	// The invariant for triple-quoted and raw strings: interior lines are data rather than code.
	// The closing line is expected to be code, because it carries whatever follows the literal —
	// a semicolon, an argument, a chained call.
	let cases: &[(Language, &str)] = &[
		(Language::Rust, "let s = r#\"\n    if x { }\n\"#;\n"),
		(Language::Python, "s = '''\n    if x:\n        pass\n'''\n"),
		(Language::Dart, "var s = '''\n    if (x) { }\n''';\n"),
		(Language::Go, "s := `\n    if x { }\n`\n"),
	];

	for (language, source) in cases {
		let lexed = lex(source, *language);

		// The declaration line and the closing line are code; the interior is not.
		let interior_is_code = lexed
			.lines
			.iter()
			.any(|line| line.number == 2 && line.is_code());

		assert!(
			!interior_is_code,
			"{language}: the interior line was treated as code"
		);
		assert_eq!(
			lexed.total_decisions(),
			0,
			"{language}: literal content produced decisions"
		);
	}
}

#[test]
fn every_supported_language_lexes_without_panicking() {
	// A smoke test across the whole language table: whatever the profile says, scanning must
	// terminate and produce one line record per physical line.
	for language in Language::ALL {
		let source = "line one\n\n// a comment\nif x { y(); }\n";

		let lexed = lex(source, language);

		assert_eq!(
			lexed.lines.len(),
			source.lines().count(),
			"{language}: line count mismatch"
		);
	}
}

#[test]
fn line_numbers_are_one_based_and_contiguous() {
	let source = "a\nb\nc\nd\n";

	for language in Language::ALL {
		let lexed = lex(source, language);

		for (index, line) in lexed.lines.iter().enumerate() {
			assert_eq!(
				line.number,
				index + 1,
				"{language}: wrong line number at index {index}"
			);
		}
	}
}

#[test]
fn a_dart_raw_string_backslash_does_not_swallow_the_file() {
	// `r'\'` is a raw string whose content is one backslash: in a raw string a backslash is data, not
	// an escape. Honoring it as an escape left the literal open, every following line was classified as
	// blank string content, and the blank-line fixer deleted them as formatting — seven lines of real
	// Dart disappeared from a script in a downstream repository.
	let source = "\
void main() {
    if (char == r'\\') {
        index += 1;
    }
    return index;
}
";
	let lexed = lex(source, Language::Dart);

	let kinds: Vec<_> = lexed
		.lines
		.iter()
		.map(|line| (line.number, line.kind))
		.collect();

	assert!(
		lexed.unterminated.is_empty(),
		"the string should close: {:?}",
		lexed.unterminated
	);

	for (number, kind) in kinds {
		if [2, 3, 4, 5].contains(&number) {
			assert_eq!(
				kind,
				LineKind::Code,
				"line {number} is real Dart and must be code, got {kind:?}"
			);
		}
	}
}
