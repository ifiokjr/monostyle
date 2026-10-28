//! YAML block-scalar payloads are data, including embedded quotes and comment markers.

use monostyle_core::Language;
use monostyle_lexer::LexedFile;
use monostyle_lexer::LineKind;
use monostyle_lexer::ProtectedKind;
use monostyle_lexer::Unterminated;
use monostyle_lexer::UnterminatedKind;
use monostyle_lexer::lex;

#[test]
fn a_literal_block_protects_shell_quotes_and_a_continued_pipeline() {
	let source = "run: |\n  echo 'MODE=\"private\"' \\\n    | cat\nnext: true\n";
	let lexed = lex(source, Language::Yaml);
	let body = source.find("  echo").expect("the scalar body");
	let next = source.find("next:").expect("the sibling mapping");

	assert!(lexed.is_clean(), "{:?}", lexed.unterminated);
	assert_eq!(lexed.source_line_count(), 2);
	assert!(
		lexed.lines[1..3]
			.iter()
			.all(monostyle_lexer::LexedLine::is_literal)
	);
	assert_eq!(lexed.lines[3].masked_code, "next: true");
	assert_protected(&lexed, body, next);
}

#[test]
fn an_unclosed_payload_quote_does_not_make_a_block_scalar_unterminated_at_eof() {
	let source = "run: |\n  echo 'payload \\\n";
	let lexed = lex(source, Language::Yaml);
	let body = source.find("  echo").expect("the scalar body");

	assert!(lexed.is_clean(), "{:?}", lexed.unterminated);
	assert_eq!(lexed.source_line_count(), 1);
	assert!(lexed.lines[1].is_literal());
	assert_protected(&lexed, body, source.len());
}

#[test]
fn literal_and_folded_chomping_forms_keep_quotes_hashes_and_blank_lines_as_data() {
	for indicator in ["|", "|-", "|+", ">", ">-", ">+"] {
		let source =
			format!("description: {indicator}\n  it's # all data\n\n  \"payload\nnext: true\n");
		let lexed = lex(&source, Language::Yaml);
		let body = source.find("  it's").expect("the scalar body");
		let next = source.find("next:").expect("the sibling mapping");

		assert!(lexed.is_clean(), "{indicator}: {:?}", lexed.unterminated);
		assert_eq!(lexed.source_line_count(), 2, "{indicator}");
		assert!(lexed.lines[1].is_literal(), "{indicator}");
		assert!(lexed.lines[2].is_blank() || lexed.lines[2].is_literal());
		assert!(lexed.lines[3].is_literal(), "{indicator}");
		assert_eq!(lexed.lines[4].masked_code, "next: true");
		assert_protected(&lexed, body, next);
	}
}

#[test]
fn a_scalar_in_a_sequence_mapping_ends_before_its_mapping_sibling() {
	let source = "steps:\n  - run: |\n      echo 'payload' \\\n        | cat\n    name: restore\nnext: true\n";
	let lexed = lex(source, Language::Yaml);
	let body = source.find("      echo").expect("the scalar body");
	let sibling = source.find("    name:").expect("the mapping sibling");

	assert!(lexed.is_clean(), "{:?}", lexed.unterminated);
	assert_eq!(lexed.source_line_count(), 4);
	assert!(
		lexed.lines[2..4]
			.iter()
			.all(monostyle_lexer::LexedLine::is_literal)
	);
	assert_eq!(lexed.lines[4].masked_code.trim_start(), "name: restore");
	assert_eq!(lexed.lines[5].masked_code, "next: true");
	assert_protected(&lexed, body, sibling);
}

#[test]
fn an_empty_scalar_does_not_consume_its_sibling() {
	let source = "run: |\nnext: true\n";
	let lexed = lex(source, Language::Yaml);

	assert!(lexed.is_clean(), "{:?}", lexed.unterminated);
	assert_eq!(lexed.source_line_count(), 2);
	assert_eq!(lexed.lines[1].masked_code, "next: true");
}

#[test]
fn scalar_indicators_inside_quoted_or_plain_values_do_not_open_blocks() {
	let source = "quoted: \"|-\"\nplain: left > right\nnext: true\n";
	let lexed = lex(source, Language::Yaml);

	assert!(lexed.is_clean(), "{:?}", lexed.unterminated);
	assert_eq!(lexed.source_line_count(), 3);
	assert!(lexed.lines.iter().all(monostyle_lexer::LexedLine::is_code));
	assert_eq!(lexed.lines[2].masked_code, "next: true");
}

#[test]
fn a_real_unterminated_quote_after_a_block_is_still_reported() {
	let source = "run: |\n  echo safe\nbroken: \"open \\\n";
	let lexed = lex(source, Language::Yaml);

	assert_eq!(
		lexed.unterminated,
		[Unterminated {
			kind: UnterminatedKind::Literal,
			line: 3,
			delimiter: "\"".to_string(),
		}]
	);
}

#[test]
fn ordinary_quotes_and_trailing_comments_keep_their_yaml_rules() {
	let source = "quoted: \"# | > ' payload\" # trailing\nnext: true\n";
	let lexed = lex(source, Language::Yaml);

	assert!(lexed.is_clean(), "{:?}", lexed.unterminated);
	assert_eq!(lexed.source_line_count(), 2);
	assert_eq!(lexed.lines[0].kind, LineKind::CodeWithComment);
	assert!(!lexed.lines[0].masked_code.contains("payload"));
	assert!(!lexed.lines[0].masked_code.contains("trailing"));
	assert_eq!(lexed.lines[1].masked_code, "next: true");
}

#[test]
fn crlf_and_unicode_payloads_keep_byte_ranges_before_dedented_code() {
	let source = "run: |+\r\n  λ says 'open \\\r\nnext: true\r\n";
	let lexed = lex(source, Language::Yaml);
	let body = source.find("  λ").expect("the scalar body");
	let next = source.find("next:").expect("the sibling mapping");

	assert!(lexed.is_clean(), "{:?}", lexed.unterminated);
	assert_eq!(lexed.lines.len(), 3);
	assert!(lexed.lines[1].is_literal());
	assert_eq!(lexed.lines[2].start_byte, next);
	assert_eq!(lexed.lines[2].masked_code, "next: true");
	assert_protected(&lexed, body, next);
}

/// Requires contiguous protection of scalar bytes without protecting sibling code.
fn assert_protected(lexed: &LexedFile, start: usize, end: usize) {
	assert!(
		lexed.protected.iter().any(|range| {
			range.kind == ProtectedKind::Literal
				&& range.start_byte <= start
				&& range.end_byte >= end
		}),
		"scalar bytes must be protected: {:?}",
		lexed.protected
	);
	assert!(lexed.protected.iter().all(|range| range.end_byte <= end));
}
