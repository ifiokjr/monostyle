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
