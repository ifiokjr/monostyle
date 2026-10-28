//! A heredoc body begins after its logical header, including continued filenames.

use monostyle_core::Language;
use monostyle_lexer::LexedFile;
use monostyle_lexer::LexedLine;
use monostyle_lexer::ProtectedKind;
use monostyle_lexer::Unterminated;
use monostyle_lexer::UnterminatedKind;
use monostyle_lexer::lex;

const MULTILINE_FILENAME: &str = r#"cat <<EOF >"if
done.txt"
payload
EOF
echo done
"#;

const ESCAPED_HEADER_NEWLINE: &str = r#"cat <<EOF \
>"if done.txt"
payload
EOF
echo done
"#;

const HEADER_COMMENT: &str = r"cat <<EOF # if hidden \
payload
EOF
echo done
";

const QUOTED_DELIMITER: &str = r"cat <<'EOF'
if \
done
EOF
echo done
";

const MISSING_TERMINATOR: &str = r#"cat <<EOF \
>"out.txt"
payload
"#;

#[test]
fn a_multiline_quoted_filename_closes_before_the_heredoc_body() {
	let lexed = lex(MULTILINE_FILENAME, Language::Shell);

	assert!(lexed.is_clean(), "{:?}", lexed.unterminated);
	assert_eq!(lexed.lines.len(), 5);
	assert_eq!(lexed.source_line_count(), 2);
	assert_eq!(lexed.total_decisions(), 0);
	assert!(lexed.lines[1..4].iter().all(LexedLine::is_literal));
	assert_eq!(lexed.lines[4].masked_code, "echo done");
	assert_complete_heredoc(&lexed, MULTILINE_FILENAME, "<<EOF");
	assert_filename(&lexed, MULTILINE_FILENAME, "\"if\ndone.txt\"");
}

#[test]
fn an_escaped_header_newline_keeps_the_next_redirect_as_code() {
	let lexed = lex(ESCAPED_HEADER_NEWLINE, Language::Shell);

	assert!(lexed.is_clean(), "{:?}", lexed.unterminated);
	assert_eq!(lexed.lines.len(), 5);
	assert_eq!(lexed.source_line_count(), 3);
	assert_eq!(lexed.total_decisions(), 0);
	assert!(lexed.lines[1].is_code());
	assert!(!lexed.lines[1].masked_code.contains("if"));
	assert!(lexed.lines[2..4].iter().all(LexedLine::is_literal));
	assert_eq!(lexed.lines[4].masked_code, "echo done");
	assert_complete_heredoc(&lexed, ESCAPED_HEADER_NEWLINE, "<<EOF");
	assert_filename(&lexed, ESCAPED_HEADER_NEWLINE, "\"if done.txt\"");
}

#[test]
fn a_backslash_in_a_header_comment_does_not_continue_the_header() {
	let lexed = lex(HEADER_COMMENT, Language::Shell);

	assert!(lexed.is_clean(), "{:?}", lexed.unterminated);
	assert_eq!(lexed.lines.len(), 4);
	assert_eq!(lexed.source_line_count(), 2);
	assert_eq!(lexed.total_decisions(), 0);
	assert_eq!(
		lexed.lines[0].trailing_comment_column,
		HEADER_COMMENT.find('#')
	);
	assert!(lexed.lines[1..3].iter().all(LexedLine::is_literal));
	assert_eq!(lexed.lines[3].masked_code, "echo done");
	assert_complete_heredoc(&lexed, HEADER_COMMENT, "<<EOF");
}

#[test]
fn a_backslash_in_a_quoted_delimiter_body_remains_literal_data() {
	let lexed = lex(QUOTED_DELIMITER, Language::Shell);

	assert!(lexed.is_clean(), "{:?}", lexed.unterminated);
	assert_eq!(lexed.lines.len(), 5);
	assert_eq!(lexed.source_line_count(), 2);
	assert_eq!(lexed.total_decisions(), 0);
	assert_eq!(lexed.lines[1].text, "if \\");
	assert!(lexed.lines[1..4].iter().all(LexedLine::is_literal));
	assert_eq!(lexed.lines[4].masked_code, "echo done");
	assert_complete_heredoc(&lexed, QUOTED_DELIMITER, "<<'EOF'");
}

#[test]
fn a_missing_terminator_retains_the_original_opener_line_after_continuation() {
	let lexed = lex(MISSING_TERMINATOR, Language::Shell);

	assert_eq!(
		lexed.unterminated,
		[Unterminated {
			kind: UnterminatedKind::Heredoc,
			line: 1,
			delimiter: "EOF".to_string(),
		}]
	);
}

/// The outer literal must close at its own terminator, before the following code.
fn assert_complete_heredoc(lexed: &LexedFile, source: &str, opener: &str) {
	let start = source.find(opener).expect("the original heredoc opener");
	let end = source.rfind("\nEOF\n").expect("the complete terminator") + "\nEOF".len();

	assert_literal_range(lexed, start, end);
}

/// Ordinary header quotes keep their own nested protected range.
fn assert_filename(lexed: &LexedFile, source: &str, filename: &str) {
	let start = source.find(filename).expect("the quoted filename");

	assert_literal_range(lexed, start, start + filename.len());
}

/// A containing or unterminated range cannot substitute for the exact boundary.
fn assert_literal_range(lexed: &LexedFile, start: usize, end: usize) {
	assert!(
		lexed.protected.iter().any(|range| {
			range.kind == ProtectedKind::Literal
				&& range.start_byte == start
				&& range.end_byte == end
		}),
		"expected literal {start}..{end}: {:?}",
		lexed.protected
	);
}
