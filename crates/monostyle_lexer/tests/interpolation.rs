//! Interpolation and literal-region tests for every language that embeds expressions in strings.
//!
//! These are the contracts the fixer's safety net stands on: a line inside a string literal is
//! never code, so no rule may fire on it and no fix may land inside it. Each test is named after
//! the real-world shape that broke it.

use monostyle_core::Language;
use monostyle_lexer::lex;

/// Asserts the given 1-based lines are literal content, and shows the offending text on failure.
fn assert_literal(lexed: &monostyle_lexer::LexedFile, lines: &[usize]) {
	for number in lines {
		let line = lexed
			.lines
			.get(number - 1)
			.unwrap_or_else(|| panic!("file has no line {number}"));

		assert!(
			line.is_literal(),
			"line {number} should be string content but was classified {:?}: {:?}",
			line.kind,
			line.text
		);
	}
}

/// Asserts the given 1-based lines are code.
fn assert_code(lexed: &monostyle_lexer::LexedFile, lines: &[usize]) {
	for number in lines {
		let line = lexed
			.lines
			.get(number - 1)
			.unwrap_or_else(|| panic!("file has no line {number}"));

		assert!(
			line.is_code(),
			"line {number} should be code but was classified {:?}: {:?}",
			line.kind,
			line.text
		);
	}
}

// ---------------------------------------------------------------------------
// Dart
// ---------------------------------------------------------------------------
#[test]
fn dart_braces_inside_a_multiline_string_are_content() {
	// A JSON payload in a triple-quoted string: the `{` must not open an interpolation, or the
	// scanner reads the payload as code and the fixer edits the string's bytes.
	let source = "\
final query = '''
{
  \"user\": \"$name\",
  \"roles\": $roles
}
''';
";
	let lexed = lex(source, Language::Dart);

	assert!(lexed.is_clean(), "the scan should need no recovery");
	assert_code(&lexed, &[1, 6]);
	assert_literal(&lexed, &[2, 3, 4, 5]);
}

#[test]
fn dart_apostrophe_inside_braced_text_does_not_desync_the_string() {
	// The corruption repro from a downstream repository: prose with an apostrophe inside braces.
	// The apostrophe opened a fake nested string, the closing brace was swallowed as its content,
	// and the interpolation stayed open across the rest of the file.
	let source = "\
class Guide {
  final tip = '''
The {hero's journey} begins at dawn
and ends at dusk
''';

  String describe() {
    return 'done';
  }
}
";
	let lexed = lex(source, Language::Dart);

	assert!(lexed.is_clean(), "the scan should need no recovery");
	assert_code(&lexed, &[1, 2, 5, 7, 8, 9, 10]);
	assert_literal(&lexed, &[3, 4]);
}

#[test]
fn dart_brace_only_lines_inside_a_string_are_not_blank() {
	// A template whose content lines hold only interpolation was classified as blank, which let
	// the excessive-blank-line rule delete real string content.
	let source = "\
final page = '''
${header}
${footer}
''';
";
	let lexed = lex(source, Language::Dart);

	assert!(lexed.is_clean(), "the scan should need no recovery");
	assert_code(&lexed, &[1, 4]);
	assert_literal(&lexed, &[2, 3]);
}

#[test]
fn dart_interpolation_with_a_nested_string_stays_balanced() {
	// `"${describe("inner")}"` — the inner quote must open a nested literal rather than close
	// the outer one.
	let source = "\
final label = \"${describe(\"inner\")} done\";
final after = 'still code';
";
	let lexed = lex(source, Language::Dart);

	assert!(lexed.is_clean(), "the scan should need no recovery");
	assert_code(&lexed, &[1, 2]);
}

#[test]
fn dart_raw_string_keeps_braces_as_content() {
	let source = r"
final pattern = r'{a} ${b} \d+';
final after = 'still code';
";
	let lexed = lex(source, Language::Dart);

	assert!(lexed.is_clean(), "the scan should need no recovery");
	assert_code(&lexed, &[2, 3]);
}

#[test]
fn dart_double_braces_in_a_map_sample_stay_content() {
	// Dart documentation strings commonly embed code samples with braces and quotes.
	let source = r"
/// Sample:
///
/// ```dart
/// if (ready) {
///   go('now');
/// }
/// ```
void helper() {}
";
	let lexed = lex(source, Language::Dart);

	assert!(lexed.is_clean(), "the scan should need no recovery");
}

// ---------------------------------------------------------------------------
// TypeScript / JavaScript
// ---------------------------------------------------------------------------
#[test]
fn template_literal_json_is_content() {
	let source = "\
const query = `
{
  \"id\": ${id},
  \"name\": \"${name}\"
}
`;
const after = 'still code';
";
	let lexed = lex(source, Language::TypeScript);

	assert!(lexed.is_clean(), "the scan should need no recovery");
	assert_code(&lexed, &[1, 6, 7]);
	assert_literal(&lexed, &[2, 3, 4, 5]);
}

#[test]
fn template_literal_with_a_nested_brace_stays_balanced() {
	// `${ {a: 1} }` — an object literal inside a hole. The hole's braces are code, but they
	// must not leak outside the template.
	let source = "\
const value = `${ {a: 1} } done`;
const after = 'still code';
";
	let lexed = lex(source, Language::TypeScript);

	assert!(lexed.is_clean(), "the scan should need no recovery");
	assert_code(&lexed, &[1, 2]);
}

// ---------------------------------------------------------------------------
// Python
// ---------------------------------------------------------------------------
#[test]
fn f_string_braces_interpolate_and_doubled_braces_escape() {
	// The bare-brace style is real here: `{x}` interpolates and `{{` is a literal brace.
	let source = "\
message = f'hello {name} and {{literal}}'
after = 'still code'
";
	let lexed = lex(source, Language::Python);

	assert!(lexed.is_clean(), "the scan should need no recovery");
	assert_code(&lexed, &[1, 2]);
}

#[test]
fn f_string_hole_with_a_nested_string_stays_balanced() {
	let source = "\
message = f\"items: {', '.join(parts)} done\"
after = 'still code'
";
	let lexed = lex(source, Language::Python);

	assert!(lexed.is_clean(), "the scan should need no recovery");
	assert_code(&lexed, &[1, 2]);
}

#[test]
fn docstring_with_braces_and_quotes_is_content() {
	let source = r#"
def guide():
    '''Use {"key": value} and don't panic.'''
    return 'done'
"#;
	let lexed = lex(source, Language::Python);

	assert!(lexed.is_clean(), "the scan should need no recovery");
	assert_literal(&lexed, &[3]);
	assert_code(&lexed, &[2, 4]);
}

// ---------------------------------------------------------------------------
// C#
// ---------------------------------------------------------------------------
#[test]
fn interpolated_string_holes_with_nested_quotes_stay_balanced() {
	let source = "\
var label = $\"items: {Find(\"key\")} done\";
var after = \"still code\";
";
	let lexed = lex(source, Language::CSharp);

	assert!(lexed.is_clean(), "the scan should need no recovery");
	assert_code(&lexed, &[1, 2]);
}

#[test]
fn verbatim_string_doubled_quotes_stay_content() {
	// `@"He said ""hi"""` — a doubled quote is content, not a closer.
	let source = "\
var quote = @\"He said \"\"hi\"\" to me\";
var after = \"still code\";
";
	let lexed = lex(source, Language::CSharp);

	assert!(lexed.is_clean(), "the scan should need no recovery");
	assert_code(&lexed, &[1, 2]);
}

// ---------------------------------------------------------------------------
// Kotlin
// ---------------------------------------------------------------------------
#[test]
fn kotlin_raw_string_braces_are_content() {
	let source = "\
val query = \"\"\"
{
  \"user\": \"$name\"
}
\"\"\"
val after = \"code\"
";
	let lexed = lex(source, Language::Kotlin);

	assert!(lexed.is_clean(), "the scan should need no recovery");
	assert_literal(&lexed, &[2, 3, 4, 5]);
	assert_code(&lexed, &[1, 6]);
}

// ---------------------------------------------------------------------------
// Swift
// ---------------------------------------------------------------------------
#[test]
fn swift_interpolation_hole_with_a_nested_string_stays_balanced() {
	// Swift interpolates with `\(...)`, and the hole may contain a string with a quote.
	let source = "\
let label = \"items: \\(find(\"key\")) done\"
let after = \"still code\"
";
	let lexed = lex(source, Language::Swift);

	assert!(lexed.is_clean(), "the scan should need no recovery");
	assert_code(&lexed, &[1, 2]);
}

#[test]
fn swift_multiline_string_with_braces_is_content() {
	let source = "\
let payload = \"\"\"
{
  \"user\": \"\\(name)\"
}
\"\"\"
let after = \"code\"
";
	let lexed = lex(source, Language::Swift);

	assert!(lexed.is_clean(), "the scan should need no recovery");
	assert_literal(&lexed, &[2, 3, 4, 5]);
	assert_code(&lexed, &[1, 6]);
}

// ---------------------------------------------------------------------------
// Go
// ---------------------------------------------------------------------------
#[test]
fn go_raw_string_braces_are_content() {
	let source = "\
const query = `
{
  \"user\": \"{{name}}\"
}
`
var after = \"code\"
";
	let lexed = lex(source, Language::Go);

	assert!(lexed.is_clean(), "the scan should need no recovery");
	assert_literal(&lexed, &[2, 3, 4, 5]);
	assert_code(&lexed, &[1, 6]);
}

// ---------------------------------------------------------------------------
// Shell and Nix
// ---------------------------------------------------------------------------
#[test]
fn shell_braces_in_double_quotes_are_content() {
	let source = "\
echo \"{
  \\\"user\\\": \\\"$name\\\"
}\"
echo 'done'
";
	let lexed = lex(source, Language::Shell);

	assert!(lexed.is_clean(), "the scan should need no recovery");
	assert_code(&lexed, &[1, 4]);
}

#[test]
fn nix_indented_string_braces_are_content() {
	let source = "\
payload = ''
{
  \"user\": \"${name}\"
}
'';
after = \"code\";
";
	let lexed = lex(source, Language::Nix);

	assert!(lexed.is_clean(), "the scan should need no recovery");
	assert_code(&lexed, &[1, 5, 6]);
	assert_literal(&lexed, &[2, 3, 4]);
}

// ---------------------------------------------------------------------------
// Scala
// ---------------------------------------------------------------------------
#[test]
fn scala_multiline_string_braces_are_content() {
	let source = "\
val query = \"\"\"
{
  \"user\": \"$name\"
}
\"\"\"
val after = \"code\"
";
	let lexed = lex(source, Language::Scala);

	assert!(lexed.is_clean(), "the scan should need no recovery");
	assert_literal(&lexed, &[2, 3, 4, 5]);
	assert_code(&lexed, &[1, 6]);
}
