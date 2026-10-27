//! Heredoc tests for the languages that carry them.
//!
//! Heredoc bodies are data: the fixer must not read them as code, and the terminator must
//! match the form the language actually accepts. Each test is named after the real-world shape
//! that broke it.

use monostyle_core::Language;
use monostyle_lexer::lex;

#[test]
fn a_ruby_squiggly_heredoc_closes_on_an_indented_delimiter() {
	// The delimiter was extracted from a six-character peek window, truncating `CONFIG` to
	// `CON`, which matched no terminator and swallowed the rest of the file as heredoc body.
	let source = "\
def config
  <<~CONFIG
    host = example
    port = '{port}'
  CONFIG
end

show
";
	let lexed = lex(source, Language::Ruby);

	assert!(lexed.is_clean(), "the scan should need no recovery");
	assert!(
		lexed.lines[7].is_code(),
		"the code after the heredoc should be code"
	);
}

#[test]
fn a_php_heredoc_closes_on_a_terminator_carrying_code() {
	// `EOT;` is the common PHP terminator form: the identifier followed by a statement end.
	let source = "<?php\n\nfunction render(): string {\n    $text = <<<EOT\nTable {$table} with {braces}\nEOT;\n    echo $text;\n}\n";
	let lexed = lex(source, Language::Php);

	assert!(lexed.is_clean(), "the scan should need no recovery");
	assert!(
		lexed.lines[7].is_code(),
		"the code after the heredoc should be code"
	);
}

#[test]
fn a_shell_heredoc_closes_on_a_bare_delimiter() {
	let source = "\
cat <<EOF
body line {kept}
${also_kept}
EOF
echo done
";
	let lexed = lex(source, Language::Shell);

	assert!(lexed.is_clean(), "the scan should need no recovery");
	assert!(
		lexed.lines[4].is_code(),
		"the code after the heredoc should be code"
	);
}

#[test]
fn a_shell_heredoc_closer_must_start_the_line() {
	// A body line that merely mentions the delimiter is content, not the end of the heredoc.
	let source = "cat <<EOF\nthe delimiter is EOF here\nEOF\necho done\n";
	let lexed = lex(source, Language::Shell);

	assert!(lexed.is_clean(), "the scan should need no recovery");
	assert!(lexed.lines[3].is_code(), "the last line should be code");
}

#[test]
fn a_ruby_append_is_not_a_heredoc() {
	// `<<` with a space after it is Ruby's append operator. Reading it as a heredoc opener with
	// the delimiter `line` swallowed every line after it as string content.
	let source = "picked << line\nnext_step = 1\n";
	let lexed = lex(source, Language::Ruby);

	assert!(
		lexed.is_clean(),
		"an append should open nothing: {:?}",
		lexed.unterminated
	);
	assert!(lexed.lines[1].is_code(), "the line after an append is code");
}

#[test]
fn a_ruby_singleton_class_opener_is_not_a_heredoc() {
	let source = "class << self\n  def build; end\nend\n";
	let lexed = lex(source, Language::Ruby);

	assert!(
		lexed.is_clean(),
		"`class << self` should open nothing: {:?}",
		lexed.unterminated
	);
	assert!(lexed.lines[1].is_code());
}

#[test]
fn a_ruby_heredoc_still_opens_when_the_delimiter_follows_the_marker() {
	let source = "text = <<~EOS\n  body\nEOS\nafter = 1\n";
	let lexed = lex(source, Language::Ruby);

	assert!(lexed.is_clean(), "the scan should need no recovery");
	assert!(
		lexed.lines[2].is_literal(),
		"the terminator line is what an opened heredoc leaves behind"
	);
	assert!(
		lexed.lines[3].is_code(),
		"the code after the heredoc is code"
	);
}

#[test]
fn a_shell_heredoc_allows_a_space_after_the_marker() {
	// `cat << EOF` is valid shell, so the space that rules out a Ruby heredoc rules nothing out here.
	let source = "cat << EOF\nbody\nEOF\necho done\n";
	let lexed = lex(source, Language::Shell);

	assert!(lexed.is_clean(), "the scan should need no recovery");
	assert!(
		lexed.lines[2].is_literal(),
		"the terminator line is what an opened heredoc leaves behind"
	);
	assert!(
		lexed.lines[3].is_code(),
		"the code after the heredoc is code"
	);
}
