//! Boundary checks for scalar indentation and complete heredoc opener consumption.

use monostyle_core::Language;
use monostyle_lexer::ProtectedKind;
use monostyle_lexer::lex;

#[test]
fn quoted_unicode_heredoc_openers_keep_raw_text_and_protected_byte_ranges() {
	let source = "cat <<'終端'\nif { # payload\n終端\necho next\n";
	let lexed = lex(source, Language::Shell);
	let start = source.find("<<").expect("the opener");
	let end = source.find("\necho").expect("the next command");

	assert!(lexed.is_clean(), "{:?}", lexed.unterminated);
	assert_eq!(lexed.source_line_count(), 2);
	assert_eq!(lexed.lines[0].text, "cat <<'終端'");
	assert!(
		lexed.lines[1..3]
			.iter()
			.all(monostyle_lexer::LexedLine::is_literal)
	);
	assert_eq!(lexed.lines[3].masked_code, "echo next");
	assert!(lexed.protected.iter().any(|range| {
		range.kind == ProtectedKind::Literal && range.start_byte == start && range.end_byte == end
	}));
}

#[test]
fn bare_and_nested_sequence_scalars_close_before_the_next_item() {
	for (header, body) in [("- |", "  body 'payload"), ("- - >-", "    body 'payload")] {
		let source = format!("{header}\n{body}\n- next\n");
		let lexed = lex(&source, Language::Yaml);

		assert!(lexed.is_clean(), "{header}: {:?}", lexed.unterminated);
		assert_eq!(lexed.source_line_count(), 2, "{header}");
		assert!(lexed.lines[1].is_literal());
		assert_eq!(lexed.lines[2].masked_code, "- next");
	}
}

#[test]
fn explicit_scalar_indent_is_not_replaced_by_the_first_payload_indent() {
	let source = "run: |2-\n    first\n  second 'payload\nnext: true\n";
	let lexed = lex(source, Language::Yaml);

	assert!(lexed.is_clean(), "{:?}", lexed.unterminated);
	assert_eq!(lexed.source_line_count(), 2);
	assert!(
		lexed.lines[1..3]
			.iter()
			.all(monostyle_lexer::LexedLine::is_literal)
	);
	assert_eq!(lexed.lines[3].masked_code, "next: true");
}

#[test]
fn an_empty_sequence_mapping_scalar_does_not_capture_its_sibling() {
	let source = "steps:\n  - run: |\n    name: next\n";
	let lexed = lex(source, Language::Yaml);

	assert!(lexed.is_clean(), "{:?}", lexed.unterminated);
	assert_eq!(lexed.source_line_count(), 3);
	assert_eq!(lexed.lines[2].masked_code.trim_start(), "name: next");
}

#[test]
fn a_pipe_after_plain_value_text_does_not_open_a_scalar() {
	let source = "plain: word: |\n  payload\nnext: true\n";
	let lexed = lex(source, Language::Yaml);

	assert!(lexed.is_clean(), "{:?}", lexed.unterminated);
	assert_eq!(lexed.source_line_count(), 3);
	assert!(lexed.lines[1].is_code());
	assert_eq!(lexed.lines[2].masked_code, "next: true");
}
