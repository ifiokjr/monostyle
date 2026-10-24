//! Rust lifetimes remain code while character literal contents stay masked.

use monostyle_core::Language;
use monostyle_lexer::lex;

#[test]
fn lifetime_parameters_references_and_labels_remain_visible() {
	let sources = [
		"fn exchange<'a>(&'a self) -> &'a Self { self }",
		"fn borrow<'value>(value: &'value str) -> &'value str { value }",
		"fn constant(value: &'static str) -> &'static str { value }",
		"fn anonymous(value: &'_ str) -> &'_ str { value }",
		"fn search() { 'search: loop { break 'search; } }",
	];

	for source in sources {
		let file = lex(source, Language::Rust);

		assert!(file.is_clean(), "{:?}", file.unterminated);
		assert_eq!(file.lines.len(), 1);
		assert_eq!(file.lines[0].masked_code, source);
	}
}

#[test]
fn character_and_byte_literals_keep_their_contents_masked() {
	let sources = [
		r"let letter = 'a'; let byte = b'b';",
		r"let quote = '\''; let slash = '\\';",
		r"let hexadecimal = '\x7f'; let scalar = '\u{7d}';",
		r"let unicode = 'λ'; let brace = '}'; if ready { next(); }",
	];

	for source in sources {
		let file = lex(source, Language::Rust);
		let masked = &file.lines[0].masked_code;

		assert!(file.is_clean(), "{:?}", file.unterminated);
		assert!(!masked.contains('\''), "{masked}");
		assert!(masked.contains("let "), "{masked}");

		if source.contains("if ready") {
			assert!(masked.contains("if ready { next(); }"), "{masked}");
			assert_eq!(file.total_decisions(), 1);
		}
	}
}

#[test]
fn lifetime_spelling_inside_strings_and_comments_is_not_code() {
	let source = "let text = \"'a if\"; // 'label if\nif ready { next(); }\n";
	let file = lex(source, Language::Rust);

	assert!(file.is_clean(), "{:?}", file.unterminated);
	assert!(!file.lines[0].masked_code.contains("'a"));
	assert!(!file.lines[0].masked_code.contains("'label"));
	assert_eq!(file.total_decisions(), 1);
}
