//! Tests for the remaining profile and line-model accessors.
//!
//! These are the small methods the scanner and the rules call constantly. A wrong answer in one of them
//! is invisible: it changes a rule's input slightly, and the resulting score looks plausible.

use monostyle_core::Language;
use monostyle_languages::BlockStyle;
use monostyle_languages::Interpolation;
use monostyle_languages::profile_for;
use monostyle_lexer::LineKind;
use monostyle_lexer::lex;

// ---------------------------------------------------------------------------
// Profile accessors
// ---------------------------------------------------------------------------

#[test]
fn a_language_with_no_line_comments_reports_none() {
	// Markdown has no comment syntax of its own, so the lookup must return nothing rather than inventing
	// a token.
	assert_eq!(
		profile_for(Language::Markdown).line_comment_at("# a heading"),
		None
	);
}

#[test]
fn a_language_with_no_block_comments_reports_none() {
	assert_eq!(
		profile_for(Language::Python).block_comment_at("/* not a comment */"),
		None
	);
}

#[test]
fn block_comment_matching_prefers_the_longer_delimiter() {
	let profile = profile_for(Language::Lua);

	// `--[[` is a block comment and `--` is a line comment in the same language, so the two lookups must
	// not be confused: matching `--` as a block opener would read a long comment's body as code.
	assert_eq!(
		profile
			.block_comment_at("--[[ body ]]")
			.map(|(open, _)| open),
		Some("--[[")
	);
	assert_eq!(
		profile.block_comment_at("-- short").map(|(open, _)| open),
		None
	);
	assert_eq!(profile.line_comment_at("-- short"), Some("--"));
}

#[test]
fn a_language_can_have_both_comment_forms() {
	// Ruby has line comments and a block comment form, and both lookups must find their own syntax.
	let profile = profile_for(Language::Ruby);

	assert_eq!(profile.line_comment_at("# a note"), Some("#"));
	assert_eq!(
		profile.block_comment_at("=begin").map(|(open, _)| open),
		Some("=begin")
	);

	// A line comment opener that is also a prefix of nothing else must not match `=begin`.
	assert_eq!(profile.line_comment_at("=begin"), None);
}

#[test]
fn block_comment_matching_reports_nothing_for_a_non_comment() {
	assert_eq!(
		profile_for(Language::Rust).block_comment_at("let x = 1;"),
		None
	);
}

#[test]
fn string_matching_prefers_the_longer_delimiter() {
	let profile = profile_for(Language::Dart);

	assert_eq!(
		profile.string_at("'''text'''").map(|rule| rule.open),
		Some("'''")
	);
	assert_eq!(
		profile.string_at("\"\"\"text\"\"\"").map(|rule| rule.open),
		Some("\"\"\"")
	);
	assert_eq!(profile.string_at("'text'").map(|rule| rule.open), Some("'"));
}

#[test]
fn a_single_line_literal_is_not_multiline() {
	assert!(!profile_for(Language::Rust).strings[0].multiline);
}

#[test]
fn multiline_literal_forms_are_marked() {
	for language in [Language::Python, Language::Dart, Language::TypeScript] {
		let profile = profile_for(language);

		assert!(
			profile.strings.iter().any(|rule| rule.multiline),
			"{language}: has a multiline literal form"
		);
	}
}

#[test]
fn interpolating_literal_forms_are_marked() {
	let dart = profile_for(Language::Dart);

	assert!(
		dart.strings.iter().any(|rule| rule.interpolates),
		"Dart interpolates with `$`"
	);
}

#[test]
fn interpolation_styles_are_declared_per_language() {
	assert!(matches!(
		profile_for(Language::Ruby).interpolation,
		Some(Interpolation::Hash)
	));
	assert!(matches!(
		profile_for(Language::TypeScript).interpolation,
		Some(Interpolation::DollarBrace)
	));
	assert!(matches!(
		profile_for(Language::Python).interpolation,
		Some(Interpolation::Brace)
	));
}

#[test]
fn rust_raw_strings_carry_a_hash_count() {
	assert!(profile_for(Language::Rust).hashed_raw_strings);
	assert!(!profile_for(Language::Python).hashed_raw_strings);
}

#[test]
fn nix_declares_its_indented_string_escapes() {
	// These are what stop `''$` from closing a Nix string early, which was a real mis-scan.
	let profile = profile_for(Language::Nix);
	let indented = profile
		.strings
		.iter()
		.find(|rule| rule.open == "''")
		.expect("the indented string form");

	assert!(
		!indented.extra_escapes.is_empty(),
		"Nix needs its escape sequences declared"
	);
	assert!(indented.extra_escapes.contains(&"''$"));
}

#[test]
fn a_language_without_extra_escapes_reports_none() {
	assert!(
		profile_for(Language::Rust)
			.strings
			.iter()
			.all(|rule| rule.extra_escapes.is_empty())
	);
}

#[test]
fn shell_and_php_declare_parameter_list_absence_or_presence() {
	// Shell has no parameter lists, so the scanner must not look for them.
	assert_eq!(profile_for(Language::Shell).parameter_list_start, None);
	assert_eq!(profile_for(Language::Rust).parameter_list_start, Some('('));
}

#[test]
fn sigil_variables_are_marked() {
	for language in [Language::Php, Language::Ruby, Language::Shell] {
		assert!(
			profile_for(language).variables_use_sigil,
			"{language}: variables use a sigil"
		);
	}

	for language in [Language::Rust, Language::Go, Language::Python] {
		assert!(
			!profile_for(language).variables_use_sigil,
			"{language}: variables are bare"
		);
	}
}

#[test]
fn heredoc_syntax_details_are_declared() {
	let shell = profile_for(Language::Shell)
		.heredoc
		.expect("shell heredocs");

	assert!(shell.allows_dash, "`<<-EOF` strips leading tabs");
	assert!(!shell.allows_tilde, "shell has no `<<~`");

	let ruby = profile_for(Language::Ruby).heredoc.expect("ruby heredocs");

	assert!(ruby.allows_tilde, "ruby has `<<~`");
}

#[test]
fn every_block_style_is_used_by_at_least_one_language() {
	// A style with no languages behind it is dead code; a language with a style nobody implemented would
	// panic in the detector.
	let mut styles: Vec<BlockStyle> = Language::ALL
		.iter()
		.filter(|language| **language != Language::Markdown)
		.map(|language| profile_for(*language).block_style)
		.collect();

	styles.sort_by_key(|style| format!("{style:?}"));
	styles.dedup();

	for expected in [
		BlockStyle::Brace,
		BlockStyle::Indentation,
		BlockStyle::EndKeyword,
	] {
		assert!(
			styles.contains(&expected),
			"{expected:?} is not used by any language"
		);
	}
}

// ---------------------------------------------------------------------------
// Line model
// ---------------------------------------------------------------------------

/// Returns the first line of a snippet.
fn line_of(source: &str) -> monostyle_lexer::LexedLine {
	lex(source, Language::Rust)
		.lines
		.first()
		.cloned()
		.expect("a line")
}

#[test]
fn a_blank_line_is_classified_as_blank() {
	let line = line_of("   \n");

	assert_eq!(line.kind, LineKind::Blank);
	assert!(line.is_blank());
	assert!(!line.is_code());
	assert!(!line.is_comment());
}

#[test]
fn a_comment_line_is_classified_as_a_comment() {
	let line = line_of("// a note\n");

	assert!(line.is_comment());
	assert!(!line.is_code());
	assert!(!line.is_blank());
}

#[test]
fn a_code_line_is_classified_as_code() {
	let line = line_of("let x = 1;\n");

	assert!(line.is_code());
	assert!(!line.is_blank());
	assert!(!line.is_comment());
}

#[test]
fn a_trailing_comment_is_recorded_with_its_column() {
	let line = line_of("let x = 1; // why\n");

	assert_eq!(line.kind, LineKind::CodeWithComment);
	assert!(line.trailing_comment_column.is_some());
}

#[test]
fn a_code_line_reports_its_length_without_the_comment() {
	let with_comment = line_of("let x = 1; // a long trailing comment that should not count\n");
	let without = line_of("let x = 1;\n");

	assert!(
		with_comment.code_len() <= without.code_len() + 2,
		"the comment must not be counted as code width"
	);
}

#[test]
fn a_preview_is_truncated_with_an_ellipsis() {
	let line = line_of("let value = a_very_long_identifier_name;\n");
	let preview = line.preview(10);

	assert!(
		preview.chars().count() <= 11,
		"the preview should be truncated: {preview:?}"
	);
	assert!(preview.ends_with('…'));
}

#[test]
fn a_short_preview_is_not_truncated() {
	assert_eq!(line_of("let x = 1;\n").preview(40), "let x = 1;");
}

#[test]
fn a_line_that_opens_a_block_is_recognized() {
	assert!(line_of("fn a() {\n").opens_block());
	assert!(line_of("if x:\n").opens_block());
}

#[test]
fn a_line_that_does_not_open_a_block_is_recognized() {
	assert!(!line_of("let x = 1;\n").opens_block());
}

#[test]
fn a_decision_line_reports_its_decisions() {
	let line = line_of("if x > 0 { work(); }\n");

	assert_eq!(line.decision_count(), 1);
	assert_eq!(line.decisions, vec!["if"]);
}

#[test]
fn a_line_with_no_decisions_reports_zero() {
	assert_eq!(line_of("let x = 1;\n").decision_count(), 0);
}

#[test]
fn a_line_that_returns_reports_it() {
	assert!(line_of("return value;\n").is_return);
	assert!(!line_of("let value = 1;\n").is_return);
}

#[test]
fn byte_offsets_address_the_source() {
	let source = "let x = 1;\nlet y = 2;\n";
	let lexed = lex(source, Language::Rust);
	let second = &lexed.lines[1];

	assert_eq!(&source[second.start_byte..second.end_byte], "let y = 2;");
}

#[test]
fn a_statement_line_is_recognized() {
	let line = line_of("let value = compute();\n");

	assert!(line.starts_statement());
}

#[test]
fn a_continuation_line_is_not_a_statement_start() {
	// A chained call or a wrapped argument carries alignment indentation, so rules that judge indentation
	// must skip it.
	for source in [".method()\n", ")\n", "];\n", ",\n"] {
		assert!(
			!line_of(source).starts_statement(),
			"{source:?} continues a statement"
		);
	}
}

#[test]
fn an_indented_line_reports_its_indent_text() {
	let lexed = lex("fn a() {\n\twork();\n}\n", Language::Rust);
	let inner = &lexed.lines[1];

	assert_eq!(inner.indent_text, "\t");
	assert!(inner.indent > 0);
}

#[test]
fn a_masked_line_hides_literal_contents() {
	let line = line_of("let s = \"if for while\";\n");

	assert!(!line.masked_code.contains("if"));
	assert!(line.masked_code.contains("let s"));
}
