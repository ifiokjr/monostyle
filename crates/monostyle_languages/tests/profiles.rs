//! Tests for language profiles.
//!
//! A profile is data, and data is easy to get wrong in ways nothing else catches: a missing comment
//! token means a whole comment is read as code, and a missing decision keyword means complexity is
//! undercounted. These tests assert the properties each profile must have.

use monostyle_core::Language;
use monostyle_languages::BlockStyle;
use monostyle_languages::profile_for;

#[test]
fn every_language_has_a_profile() {
	for language in Language::ALL {
		let profile = profile_for(language);

		assert_eq!(
			profile.language, language,
			"{language}: profile reports the wrong language"
		);
	}
}

#[test]
fn every_source_language_declares_decision_keywords() {
	// A language with no decision keywords reports zero complexity for every function, which is worse
	// than reporting nothing because the number looks authoritative.
	for language in Language::ALL {
		if language == Language::Markdown {
			continue;
		}

		let profile = profile_for(language);

		assert!(
			!profile.decision_keywords.is_empty(),
			"{language}: no decision keywords, so complexity would always be 1"
		);
	}
}

#[test]
fn every_language_declares_comment_syntax() {
	// Markdown has no comments of its own; every programming language does.
	for language in Language::ALL {
		if language == Language::Markdown {
			continue;
		}

		let profile = profile_for(language);

		assert!(
			!profile.line_comments.is_empty() || !profile.block_comments.is_empty(),
			"{language}: no comment syntax, so comments would be read as code"
		);
	}
}

#[test]
fn every_language_declares_string_delimiters() {
	for language in Language::ALL {
		if language == Language::Markdown {
			continue;
		}

		let profile = profile_for(language);

		assert!(
			!profile.strings.is_empty(),
			"{language}: no string delimiters, so string contents would be read as code"
		);
	}
}

#[test]
fn end_keyword_languages_declare_their_closers() {
	for language in Language::ALL {
		let profile = profile_for(language);

		if profile.block_style == BlockStyle::EndKeyword {
			assert!(
				!profile.end_keywords.is_empty(),
				"{language}: an end-keyword language needs its closing keywords"
			);
		}
	}

	// The specific closers that matter, asserted so a typo is caught rather than silently accepted.
	assert_eq!(profile_for(Language::Ruby).end_keywords, ["end"]);
	assert!(profile_for(Language::Shell).end_keywords.contains(&"fi"));
	assert!(profile_for(Language::Shell).end_keywords.contains(&"done"));
	assert!(profile_for(Language::Lua).end_keywords.contains(&"until"));
}

#[test]
fn interpolating_languages_declare_their_interpolation_style() {
	// Interpolation matters because a quote inside `${...}` does not end the string, so a missing style
	// means a literal is read as ending early.
	for language in [
		Language::TypeScript,
		Language::JavaScript,
		Language::Dart,
		Language::Python,
		Language::Ruby,
		Language::Nix,
		Language::Kotlin,
		Language::Swift,
	] {
		assert!(
			profile_for(language).interpolation.is_some(),
			"{language}: declares interpolation but the profile does not say how"
		);
	}
}

#[test]
fn heredoc_languages_declare_their_marker() {
	for language in [Language::Shell, Language::Ruby, Language::Php] {
		let profile = profile_for(language);

		assert!(profile.heredoc.is_some(), "{language}: uses heredocs");
	}

	assert_eq!(
		profile_for(Language::Php)
			.heredoc
			.map(|syntax| syntax.marker),
		Some("<<<")
	);
	assert_eq!(
		profile_for(Language::Shell)
			.heredoc
			.map(|syntax| syntax.marker),
		Some("<<")
	);
}

#[test]
fn nestable_comment_languages_are_marked() {
	// Rust's `/* /* */ */` closes at the second terminator, so a profile that says otherwise ends a
	// comment early and reads the remainder as code.
	for language in [
		Language::Rust,
		Language::Kotlin,
		Language::Swift,
		Language::Haskell,
	] {
		assert!(
			profile_for(language).nestable_comments,
			"{language}: block comments nest"
		);
	}

	for language in [Language::C, Language::JavaScript, Language::Go] {
		assert!(
			!profile_for(language).nestable_comments,
			"{language}: block comments do not nest"
		);
	}
}

#[test]
fn regex_literal_languages_are_marked() {
	for language in [Language::JavaScript, Language::TypeScript, Language::Ruby] {
		assert!(
			profile_for(language).regex_literals,
			"{language}: `/` can open a regex"
		);
	}
}

#[test]
fn indentation_based_languages_are_marked() {
	for language in [Language::Python, Language::Haskell] {
		assert_eq!(
			profile_for(language).block_style,
			BlockStyle::Indentation,
			"{language}: blocks are delimited by indentation"
		);
	}

	for language in [Language::Rust, Language::Go, Language::Dart] {
		assert_eq!(
			profile_for(language).block_style,
			BlockStyle::Brace,
			"{language}: blocks are delimited by braces"
		);
	}
}

#[test]
fn rust_declares_hash_counted_raw_strings() {
	let profile = profile_for(Language::Rust);

	assert!(profile.hashed_raw_strings, "`r#\"..\"#` needs a hash count");
	assert_eq!(profile.raw_string_prefix, Some('r'));
}

// ---------------------------------------------------------------------------
// Profile lookups
// ---------------------------------------------------------------------------

#[test]
fn the_longest_comment_token_wins() {
	// `///` must be recognized before `//`, or a doc comment is classified as an ordinary one.
	let profile = profile_for(Language::Rust);

	assert_eq!(profile.line_comment_at("/// doc"), Some("//"));
	assert!(profile.is_documentation_comment("/// doc"));
	assert!(!profile.is_documentation_comment("// ordinary"));
}

#[test]
fn the_longest_block_comment_opener_wins() {
	// Lua's `--[[` must beat `--`, or a long comment is read as a line comment.
	let profile = profile_for(Language::Lua);

	assert_eq!(
		profile
			.block_comment_at("--[[ long ]]")
			.map(|(open, _)| open),
		Some("--[[")
	);
}

#[test]
fn the_longest_string_opener_wins() {
	// Dart's `'''` must beat `'`, or a triple-quoted string ends at its first character.
	let profile = profile_for(Language::Dart);

	assert_eq!(
		profile.string_at("'''text'''").map(|rule| rule.open),
		Some("'''")
	);
	assert_eq!(profile.string_at("'text'").map(|rule| rule.open), Some("'"));
}

#[test]
fn no_string_opener_is_recognized_where_none_exists() {
	let profile = profile_for(Language::Rust);

	assert!(profile.string_at("just an identifier").is_none());
}

#[test]
fn documentation_syntax_is_recognized_per_language() {
	/// A language and a comment opening that marks documentation in it.
	const CASES: &[(Language, &str)] = &[
		(Language::Rust, "/// docs"),
		(Language::Rust, "//! docs"),
		(Language::TypeScript, "/** docs */"),
		(Language::JavaScript, "/** docs */"),
		(Language::Java, "/** docs */"),
		(Language::Php, "/** docs */"),
		(Language::Kotlin, "/** docs */"),
		(Language::CSharp, "/// docs"),
		(Language::Python, "## docs"),
	];

	for (language, text) in CASES {
		assert!(
			profile_for(*language).is_documentation_comment(text),
			"{language}: `{text}` should be documentation"
		);
	}
}

#[test]
fn ordinary_comments_are_not_documentation() {
	for language in [
		Language::Rust,
		Language::TypeScript,
		Language::Python,
		Language::Go,
	] {
		assert!(
			!profile_for(language).is_documentation_comment("# not a doc comment"),
			"{language}: a bare hash is not documentation syntax"
		);
	}
}

#[test]
fn doc_string_languages_are_marked() {
	for language in [Language::Python, Language::Elixir] {
		assert!(
			profile_for(language).uses_doc_strings(),
			"{language}: documents with string literals rather than comments"
		);
	}

	for language in [Language::Rust, Language::Go, Language::TypeScript] {
		assert!(
			!profile_for(language).uses_doc_strings(),
			"{language}: documents with comments"
		);
	}
}

#[test]
fn regex_position_detection_matches_operand_context() {
	let profile = profile_for(Language::JavaScript);

	// After an operator or an opening delimiter, a slash opens a literal.
	for character in ['(', ',', '=', ':', '[', '!', '&', '|', '?', '{', '}'] {
		assert!(
			profile.regex_allowed_after(character),
			"`/` after `{character}` opens a regex"
		);
	}

	// After a value, it divides.
	for character in ['a', '1', ')', ']', '"'] {
		assert!(
			!profile.regex_allowed_after(character),
			"`/` after `{character}` divides"
		);
	}
}

#[test]
fn regex_position_is_never_allowed_in_languages_without_regex_literals() {
	let profile = profile_for(Language::Rust);

	for character in ['(', '=', ','] {
		assert!(!profile.regex_allowed_after(character));
	}
}

// ---------------------------------------------------------------------------
// Language resolution
// ---------------------------------------------------------------------------

#[test]
fn languages_resolve_from_their_own_names() {
	for language in Language::ALL {
		assert_eq!(
			Language::from_name(language.name()),
			Some(language),
			"{language}: does not resolve from its own name"
		);
	}
}

#[test]
fn language_names_are_unique() {
	let mut names: Vec<&str> = Language::ALL
		.iter()
		.map(|language| language.name())
		.collect();

	names.sort_unstable();
	let count = names.len();
	names.dedup();

	assert_eq!(names.len(), count, "two languages share a name");
}

#[test]
fn extensions_are_unique_across_languages() {
	let mut seen: Vec<(&str, Language)> = Vec::new();

	for language in Language::ALL {
		for extension in language.extensions() {
			if let Some((_existing, owner)) = seen.iter().find(|(name, _)| name == extension) {
				panic!("`.{extension}` is claimed by both {owner} and {language}");
			}

			seen.push((extension, language));
		}
	}
}

#[test]
fn every_language_resolves_from_each_of_its_extensions() {
	for language in Language::ALL {
		for extension in language.extensions() {
			assert_eq!(
				Language::from_extension(extension),
				Some(language),
				".{extension} should resolve to {language}"
			);
		}
	}
}

#[test]
fn an_unknown_extension_resolves_to_nothing() {
	assert_eq!(Language::from_extension("unknownext"), None);
	assert_eq!(Language::from_extension(""), None);
}

#[test]
fn a_leading_dot_is_tolerated() {
	assert_eq!(Language::from_extension(".rs"), Some(Language::Rust));
}

#[test]
fn extension_matching_ignores_case() {
	assert_eq!(Language::from_extension("RS"), Some(Language::Rust));
}

#[test]
fn fence_tags_resolve_common_aliases() {
	/// A fence tag and the language it should resolve to.
	const CASES: &[(&str, Language)] = &[
		("rust", Language::Rust),
		("rs", Language::Rust),
		("typescript", Language::TypeScript),
		("ts", Language::TypeScript),
		("js", Language::JavaScript),
		("javascript", Language::JavaScript),
		("golang", Language::Go),
		("go", Language::Go),
		("c++", Language::Cpp),
		("c#", Language::CSharp),
		("py", Language::Python),
		("python", Language::Python),
		("sh", Language::Shell),
		("bash", Language::Shell),
		("shell", Language::Shell),
		("dart", Language::Dart),
	];

	for (tag, expected) in CASES {
		assert_eq!(
			Language::from_fence_tag(tag),
			Some(*expected),
			"`{tag}` should resolve"
		);
	}
}

#[test]
fn fence_tags_tolerate_extra_information() {
	// A fence often carries more than a language, such as `rust,no_run` or `js title="x"`.
	assert_eq!(
		Language::from_fence_tag("rust,no_run"),
		Some(Language::Rust)
	);
	assert_eq!(
		Language::from_fence_tag("rust title=\"a\""),
		Some(Language::Rust)
	);
}

#[test]
fn an_empty_fence_tag_resolves_to_nothing() {
	assert_eq!(Language::from_fence_tag(""), None);
	assert_eq!(Language::from_fence_tag("   "), None);
}

#[test]
fn an_unknown_fence_tag_resolves_to_nothing() {
	assert_eq!(Language::from_fence_tag("notalanguage"), None);
}

#[test]
fn canonical_extensions_are_declared() {
	for language in Language::ALL {
		assert!(
			!language.canonical_extension().is_empty(),
			"{language}: needs a canonical extension for synthetic files"
		);
	}
}
