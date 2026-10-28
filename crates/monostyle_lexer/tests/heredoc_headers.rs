//! Pending heredoc bodies do not hide quotes or comments on their opener line.

use monostyle_core::Language;
use monostyle_lexer::lex;

#[test]
fn a_trailing_header_comment_does_not_add_decisions() {
	let source = "cat <<EOF # if hidden\npayload\nEOF\necho done\n";
	let lexed = lex(source, Language::Shell);

	assert!(lexed.is_clean(), "{:?}", lexed.unterminated);
	assert_eq!(lexed.lines[0].trailing_comment_column, source.find('#'));
	assert!(!lexed.lines[0].masked_code.contains("if"));
	assert_eq!(lexed.total_decisions(), 0);
	assert!(lexed.lines[1].is_literal());
	assert!(lexed.lines[3].is_code());
}

#[test]
fn a_quoted_redirect_filename_does_not_add_decisions() {
	let source = "cat <<EOF > \"if done.txt\"\npayload\nEOF\necho done\n";
	let lexed = lex(source, Language::Shell);

	assert!(lexed.is_clean(), "{:?}", lexed.unterminated);
	assert!(!lexed.lines[0].masked_code.contains("if"));
	assert!(!lexed.lines[0].masked_code.contains("done.txt"));
	assert_eq!(lexed.total_decisions(), 0);
	assert!(lexed.lines[1].is_literal());
	assert!(lexed.lines[3].is_code());
}
