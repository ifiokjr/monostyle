//! Tests for the fixer's safety net.
//!
//! The fixer is the one component that writes to a user's files, so its guarantees are tested
//! directly rather than trusted from the rules' own tests: a fix may not edit the bytes of a
//! string or comment, a file the lexer could not scan cleanly is left alone, and a whitespace
//! rewrite that changes the code's shape is thrown away instead of written.

use std::path::PathBuf;

use monostyle::fix::enters_protected;
use monostyle::fix::fix_file;
use monostyle_core::Fix;
use monostyle_core::Span;

/// Writes `source` to a unique temporary file with the given extension.
fn temp_file(name: &str, extension: &str, source: &str) -> PathBuf {
	let mut path = std::env::temp_dir();
	path.push(format!("monostyle-fix-safety-{name}.{extension}"));
	std::fs::write(&path, source).expect("temporary file should be writable");

	path
}

fn insertion(offset: usize) -> Fix {
	Fix::insert(Span::new(offset, offset, 1, 1), "\n", "test insertion")
}

#[test]
fn an_insertion_inside_a_literal_interior_is_refused() {
	// A Dart triple-quoted string spans bytes 18..91; an insertion anywhere strictly inside it
	// would add a line to the string's content.
	let source = "final tip = '''\nThe journey begins\nat dawn\n''';\n";
	let lexed = monostyle_lexer::lex(source, monostyle_core::Language::Dart);
	let ranges = &lexed.protected;

	assert!(
		!ranges.is_empty(),
		"the string should be a protected region"
	);
	assert!(enters_protected(&insertion(30), ranges));
	assert!(enters_protected(&insertion(45), ranges));
}

#[test]
fn an_insertion_at_a_range_start_is_allowed_but_above_a_closer_is_not() {
	// Placing a blank line immediately before a string that *starts* a position is a layout
	// change; placing one immediately before the line that *closes* a multi-line string would
	// add a blank to the string's content, so it is refused.
	let source = "final items = {\n  'first': 'a',\n  'tail': '''\nspanning\n''',\n};\n";
	let lexed = monostyle_lexer::lex(source, monostyle_core::Language::Dart);
	let ranges = &lexed.protected;

	let entry_start = source.find("  'tail'").expect("entry should exist") + 2;
	let closer_start = source.rfind("'''").expect("closer should exist");

	assert!(
		!enters_protected(&insertion(entry_start), ranges),
		"the boundary where a string starts is a layout position"
	);
	assert!(
		enters_protected(&insertion(closer_start), ranges),
		"the boundary where a string ends is inside its bytes"
	);
}

#[test]
fn a_replacement_overlapping_a_literal_is_refused() {
	let source = "final tip = 'content';\n";
	let lexed = monostyle_lexer::lex(source, monostyle_core::Language::Dart);
	let ranges = &lexed.protected;
	let string_start = source.find("'content'").expect("string should exist");

	let overlapping = Fix::replace(
		Span::new(string_start + 2, string_start + 6, 1, 1),
		"    ",
		"test replacement",
	);

	assert!(enters_protected(&overlapping, ranges));
}

#[test]
fn a_fix_inside_a_string_is_rejected_and_the_file_is_untouched() {
	let source = "final tip = '''\nfirst line of the story\nsecond line\n''';\n";
	let path = temp_file("rejected", "dart", source);
	let string_start = source.find("first line").expect("content should exist");
	let inside = Fix::insert(
		Span::new(string_start, string_start, 3, 3),
		"\n",
		"corrupting edit",
	);

	let outcome = fix_file(&path, &[inside], false).expect("the fix should run");

	assert_eq!(outcome.rejected, 1, "the interior edit should be refused");
	assert_eq!(outcome.applied, 0);
	assert!(!outcome.written);
	assert_eq!(
		std::fs::read_to_string(&path).expect("the file should read back"),
		source,
		"the file must be byte-identical"
	);
}

#[test]
fn a_boundary_fix_is_applied_and_the_string_is_untouched() {
	let source = "final items = {\n  'first': 'a',\n  'second': 'b',\n};\n";
	let path = temp_file("boundary", "dart", source);

	// Insert a blank line above the `'second'` entry: the anchor is that line's start, which is
	// the boundary of the string that begins there.
	let entry_offset = source.find("  'second'").expect("entry should exist");
	let fix = Fix::insert(
		Span::new(entry_offset, entry_offset, 3, 3),
		"\n",
		"blank line",
	);

	let outcome = fix_file(&path, &[fix], false).expect("the fix should run");

	assert_eq!(outcome.rejected, 0, "a boundary edit is a layout change");
	assert_eq!(outcome.applied, 1);
	assert!(outcome.written);
	assert_eq!(
		std::fs::read_to_string(&path).expect("the file should read back"),
		"final items = {\n  'first': 'a',\n\n  'second': 'b',\n};\n"
	);
}

#[test]
fn a_file_with_an_unterminated_construct_is_skipped() {
	// A string that never closes means the lexer guessed where everything after it begins, so
	// its byte ranges are untrustworthy.
	let source = "final tip = '''\nthe story never ends\n";
	let path = temp_file("unterminated", "dart", source);
	let fix = insertion(0);

	let outcome = fix_file(&path, &[fix], false).expect("the fix should run");

	assert!(outcome.skipped_untrusted);
	assert_eq!(outcome.applied, 0);
	assert!(!outcome.written);
	assert_eq!(
		std::fs::read_to_string(&path).expect("the file should read back"),
		source
	);
}

#[test]
fn a_whitespace_rewrite_that_changes_the_codes_shape_is_reverted() {
	// A hand-made edit that merges two lines: its replacement is whitespace, so it passes the
	// whitespace gate, but the merged lines change the file's shape and the rewrite is thrown
	// away rather than written.
	let source = "fn first() {\n}\n\nfn second() {\n}\n";
	let path = temp_file("reverted", "rs", source);

	let newline = source.find("{\n}").expect("a block should exist") + 2;
	let merger = Fix::replace(Span::new(newline, newline + 1, 1, 1), " ", "merging edit");

	let outcome = fix_file(&path, &[merger], false).expect("the fix should run");

	assert!(outcome.reverted, "the shape change should be caught");
	assert!(!outcome.written);
	assert_eq!(
		std::fs::read_to_string(&path).expect("the file should read back"),
		source,
		"the file must be byte-identical"
	);
}

#[test]
fn a_legitimate_blank_line_fix_survives_the_structural_check() {
	// The check must not false-positive on the fixes it exists to protect.
	let source = "fn work() {\n    let a = 1;\n\n\n\n    let b = 2;\n}\n";
	let path = temp_file("legitimate", "rs", source);

	let lexed = monostyle_lexer::lex(source, monostyle_core::Language::Rust);
	let fixes: Vec<Fix> = monostyle_rules::whitespace::excessive_blank_lines(
		&lexed,
		&monostyle_rules::RulesConfig::default(),
	)
	.into_iter()
	.filter_map(|finding| finding.fix)
	.collect();

	assert!(!fixes.is_empty(), "the fixture should have a fixable run");

	let outcome = fix_file(&path, &fixes, false).expect("the fix should run");

	assert!(!outcome.reverted, "a real fix must not be reverted");
	assert!(outcome.written);
	assert_eq!(
		std::fs::read_to_string(&path).expect("the file should read back"),
		"fn work() {\n    let a = 1;\n\n    let b = 2;\n}\n"
	);
}

#[test]
fn a_dart_story_fixture_is_fixed_without_touching_its_strings() {
	// The end-to-end corruption scenario from a downstream repository: prose with an apostrophe
	// inside braces, a JSON payload with braces and sigils, and one genuine stacked-blank run.
	let source = "class Guide {\n  final tip = '''\nThe {hero's journey} begins at dawn\nand ends at dusk\n''';\n\n  final payload = '''\n{\n  \"user\": \"$name\",\n  \"roles\": $roles\n}\n''';\n\n  String describe() {\n    var label = 'journey';\n\n\n\n    return label;\n  }\n}\n";
	let path = temp_file("story", "dart", source);

	let lexed = monostyle_lexer::lex(source, monostyle_core::Language::Dart);
	let config = monostyle_rules::RulesConfig::default();
	let fixes: Vec<Fix> = monostyle_rules::whitespace::excessive_blank_lines(&lexed, &config)
		.into_iter()
		.filter_map(|finding| finding.fix)
		.collect();

	assert!(!fixes.is_empty(), "the stacked run should be fixable");

	let outcome = fix_file(&path, &fixes, false).expect("the fix should run");

	assert!(!outcome.reverted);
	assert_eq!(outcome.rejected, 0, "no fix should even approach a string");
	assert_eq!(
		std::fs::read_to_string(&path).expect("the file should read back"),
		"class Guide {\n  final tip = '''\nThe {hero's journey} begins at dawn\nand ends at dusk\n''';\n\n  final payload = '''\n{\n  \"user\": \"$name\",\n  \"roles\": $roles\n}\n''';\n\n  String describe() {\n    var label = 'journey';\n\n\n    return label;\n  }\n}\n",
		"only the stacked run changes; every string byte is identical"
	);
}

#[test]
fn a_crlf_file_keeps_its_line_endings_through_the_safety_net() {
	// The collapse replacement is written with bare line feeds; a CRLF file must come back with
	// its own endings everywhere, including the lines the fixer touched.
	let source = "fn work() {\r\n    let a = 1;\r\n\r\n\r\n\r\n    let b = 2;\r\n}\r\n";
	let path = temp_file("crlf", "rs", source);

	let lexed = monostyle_lexer::lex(source, monostyle_core::Language::Rust);
	let fixes: Vec<Fix> = monostyle_rules::whitespace::excessive_blank_lines(
		&lexed,
		&monostyle_rules::RulesConfig::default(),
	)
	.into_iter()
	.filter_map(|finding| finding.fix)
	.collect();

	let outcome = fix_file(&path, &fixes, false).expect("the fix should run");

	assert!(outcome.written, "the stacked run should be collapsed");

	let after = std::fs::read_to_string(&path).expect("the file should read back");

	assert_eq!(
		after, "fn work() {\r\n    let a = 1;\r\n\r\n    let b = 2;\r\n}\r\n",
		"every line ending stays CRLF, including the collapsed run"
	);
}
