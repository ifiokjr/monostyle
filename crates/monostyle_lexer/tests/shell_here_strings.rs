//! Here-strings are redirections whose payload ends on the same line, not heredocs.

mod fixtures {
	include!("fixtures/shell_here_strings.rs");
}

use monostyle_core::Language;
use monostyle_lexer::ProtectedKind;
use monostyle_lexer::ProtectedRange;
use monostyle_lexer::Unterminated;
use monostyle_lexer::UnterminatedKind;
use monostyle_lexer::lex;

#[test]
fn a_snapshot_reader_keeps_code_after_its_here_string() {
	let source = fixtures::SNAPSHOT_AND_IDENTITY;
	let lexed = lex(source, Language::Shell);

	assert!(lexed.is_clean(), "{:?}", lexed.unterminated);
	assert_eq!(lexed.lines.len(), 21);
	assert_eq!(lexed.source_line_count(), 19);
	assert!(lexed.lines[15].is_code());
	assert!(lexed.lines[16].is_code());
	assert_eq!(lexed.lines[18].masked_code, "owned_next() {");
	assert!(
		lexed.protected.iter().all(|range| {
			!source
				.get(range.start_byte..range.end_byte)
				.expect("a protected range within the source")
				.contains('\n')
		}),
		"only the same-line quoted payloads are protected"
	);
}

#[test]
fn shell_here_strings_leave_following_lines_as_code() {
	for payload in ["word", "$rows", "\"$rows\"", "'if # data { }'"] {
		let source = format!("read -r value <<< {payload}\necho done\n");
		let lexed = lex(&source, Language::Shell);

		assert!(lexed.is_clean(), "{payload}: {:?}", lexed.unterminated);
		assert_eq!(lexed.lines.len(), 2);
		assert_eq!(lexed.source_line_count(), 2, "{payload}");
		assert!(
			lexed.lines.iter().all(monostyle_lexer::LexedLine::is_code),
			"{payload}"
		);
		assert!(
			lexed.lines[0].masked_code.starts_with("read -r value <<<"),
			"the entire operator must remain code: {payload}"
		);
	}
}

#[test]
fn shell_here_strings_protect_only_their_quoted_payload() {
	let payload = "\"if # data { }\"";
	let source = format!("read -r value <<<{payload}\nnext() {{\n  echo done\n}}\n");
	let start = source.find(payload).expect("the quoted payload");
	let lexed = lex(&source, Language::Shell);

	assert!(lexed.is_clean(), "{:?}", lexed.unterminated);
	assert_eq!(lexed.source_line_count(), 4);
	assert_eq!(lexed.lines[0].masked_code.trim_end(), "read -r value <<<");
	assert_eq!(lexed.lines[1].masked_code, "next() {");
	assert_eq!(
		lexed.protected,
		[ProtectedRange {
			start_byte: start,
			end_byte: start + payload.len(),
			kind: ProtectedKind::Literal,
		}]
	);
}

#[test]
fn multiple_shell_here_strings_do_not_open_a_heredoc_from_the_operator_suffix() {
	let source = "read -r left<<<first; read -r right<<<second\necho done\n";
	let lexed = lex(source, Language::Shell);

	assert!(lexed.is_clean(), "{:?}", lexed.unterminated);
	assert_eq!(lexed.source_line_count(), 2);
	assert_eq!(lexed.protected, Vec::<ProtectedRange>::new());

	for (line, expected) in lexed.lines.iter().zip(source.lines()) {
		assert_eq!(line.masked_code, expected);
	}
}

#[test]
fn a_shell_here_string_can_precede_a_real_heredoc_on_the_same_line() {
	let source = "cat <<<word <<EOF\nif { # data\nEOF\necho done\n";
	let start = source.find("<<EOF").expect("the heredoc opener");
	let end = source.find("\necho").expect("the line after the heredoc");
	let lexed = lex(source, Language::Shell);

	assert!(lexed.is_clean(), "{:?}", lexed.unterminated);
	assert_eq!(lexed.lines.len(), 4);
	assert_eq!(lexed.source_line_count(), 2);
	assert!(lexed.lines[1].is_literal());
	assert!(lexed.lines[2].is_literal());
	assert!(lexed.lines[3].is_code());
	assert_eq!(
		lexed.protected,
		[ProtectedRange {
			start_byte: start,
			end_byte: end,
			kind: ProtectedKind::Literal,
		}]
	);
}

#[test]
fn shell_heredocs_keep_their_delimiter_and_tab_stripping_rules() {
	for (opener, terminator) in [
		("<<EOF", "EOF"),
		("<<-EOF", "\tEOF"),
		("<<'EOF'", "EOF"),
		("<<\"EOF\"", "EOF"),
	] {
		let source = format!("cat {opener}\nif {{ # data\n{terminator}\necho done\n");
		let lexed = lex(&source, Language::Shell);

		assert!(lexed.is_clean(), "{opener}: {:?}", lexed.unterminated);
		assert_eq!(lexed.source_line_count(), 2, "{opener}");
		assert!(lexed.lines[1].is_literal(), "{opener}");
		assert!(lexed.lines[3].is_code(), "{opener}");
	}
}

#[test]
fn a_real_unterminated_heredoc_after_a_here_string_is_still_reported() {
	let source = "read -r value <<<word\ncat <<EOF\nmissing terminator\n";
	let lexed = lex(source, Language::Shell);

	assert_eq!(
		lexed.unterminated,
		[Unterminated {
			kind: UnterminatedKind::Heredoc,
			line: 2,
			delimiter: "EOF".to_string(),
		}]
	);
}

#[test]
fn php_triple_angle_heredocs_and_nowdocs_remain_multiline_literals() {
	for opener in ["<<<EOF", "<<<'EOF'", "<<<\"EOF\""] {
		let source = format!("<?php\n$text = {opener}\nif {{ # data\nEOF;\necho $text;\n");
		let lexed = lex(&source, Language::Php);

		assert!(lexed.is_clean(), "{opener}: {:?}", lexed.unterminated);
		assert_eq!(lexed.source_line_count(), 3, "{opener}");
		assert!(lexed.lines[2].is_literal(), "{opener}");
		assert!(lexed.lines[4].is_code(), "{opener}");
	}
}

#[test]
fn ruby_heredoc_modifiers_remain_multiline_literals() {
	for opener in ["<<EOF", "<<-EOF", "<<~EOF", "<<'EOF'"] {
		let source = format!("text = {opener}\nif {{ # data\nEOF\nputs text\n");
		let lexed = lex(&source, Language::Ruby);

		assert!(lexed.is_clean(), "{opener}: {:?}", lexed.unterminated);
		assert_eq!(lexed.source_line_count(), 2, "{opener}");
		assert!(lexed.lines[1].is_literal(), "{opener}");
		assert!(lexed.lines[3].is_code(), "{opener}");
	}
}
