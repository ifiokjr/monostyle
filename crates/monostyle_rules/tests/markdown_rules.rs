//! Tests for the Markdown rules.
//!
//! These rules score documentation, which is unusual enough to have its own failure modes: a fence can
//! be untagged, name a language nobody supports, or contain code that is deliberately messy to
//! illustrate a point. The tests assert both that the rules fire and that they stay quiet where
//! documentation differs from source.

use monostyle_core::Language;
use monostyle_lexer::lex;
use monostyle_rules::RulesConfig;
use monostyle_rules::markdown;
use monostyle_rules::run_rules;

/// Runs every rule over a Markdown document.
///
/// The whole set rather than only the `markdown/` namespace, because a finding inside a fence keeps the
/// nested rule's name — `readability/blank-line-before-control-flow` — and is only distinguishable by
/// its message. Filtering on the namespace would hide exactly the findings these tests are about.
fn analyze(source: &str) -> Vec<monostyle_core::Finding> {
	let lexed = lex(source, Language::Markdown);

	run_rules(&lexed, &RulesConfig::default())
}

/// Returns the rule names that fired.
fn rules(source: &str) -> Vec<String> {
	let mut names: Vec<String> = analyze(source)
		.into_iter()
		.map(|finding| finding.rule)
		.collect();

	names.sort_unstable();
	names.dedup();
	names
}

/// Returns the findings whose rule is the given one.
fn findings_for(source: &str, rule: &str) -> Vec<monostyle_core::Finding> {
	analyze(source)
		.into_iter()
		.filter(|finding| finding.rule == rule)
		.collect()
}

// ---------------------------------------------------------------------------
// Fence language tags
// ---------------------------------------------------------------------------

#[test]
fn an_untagged_fence_is_reported() {
	let names = rules("```\nsome content\n```\n");

	assert!(
		names.contains(&"markdown/fence-without-language".to_string()),
		"an untagged fence should be reported, got {names:?}"
	);
}

#[test]
fn an_unknown_language_is_reported_separately_from_an_untagged_fence() {
	// The two cases need different advice: one needs a tag added, the other needs the spelling checked.
	let names = rules("```notalanguage\nx\n```\n");

	assert!(
		names.contains(&"markdown/fence-language-unknown".to_string()),
		"an unknown language should be reported, got {names:?}"
	);
	assert!(
		!names.contains(&"markdown/fence-without-language".to_string()),
		"a tagged fence is not untagged"
	);
}

#[test]
fn a_tagged_fence_is_not_reported_for_its_language() {
	let names = rules("```rust\nfn a() {}\n```\n");

	assert!(
		!names.iter().any(|name| name.starts_with("markdown/fence-")),
		"a valid tag should not be reported, got {names:?}"
	);
}

#[test]
fn an_empty_fence_is_not_reported_for_its_contents() {
	// An empty fence is a placeholder, not code with problems, so scoring it would be noise.
	let names = rules("```rust\n```\n");

	assert!(
		!names.contains(&"readability/blank-line-before-return".to_string()),
		"an empty fence has nothing to score, got {names:?}"
	);
}

// ---------------------------------------------------------------------------
// Fence contents
// ---------------------------------------------------------------------------

#[test]
fn cramped_code_inside_a_fence_is_scored() {
	// The layout rules apply to fences with their locations mapped back to the document.
	let findings = findings_for(
		"```rust\nfn a() {\n    work();\n    if x {\n        work();\n    }\n}\n```\n",
		"readability/blank-line-before-control-flow",
	);

	assert!(
		!findings.is_empty(),
		"cramped code in a fence should be reported"
	);
}

#[test]
fn a_finding_inside_a_fence_points_at_a_document_line() {
	// A location in fence-relative coordinates would send a reader to the wrong place in the file.
	let findings = findings_for(
		"Intro.\n\n```rust\nfn a() {\n    work();\n    if x {\n        work();\n    }\n}\n```\n",
		"readability/blank-line-before-control-flow",
	);

	let finding = findings.first().expect("a finding inside the fence");

	// The fence's code starts on line four, so a finding on the fourth code line is on line eight.
	assert!(
		finding.span.start_line > 3,
		"the location should be a document line, got {}",
		finding.span.start_line
	);
}

#[test]
fn a_finding_inside_a_fence_names_the_language() {
	let findings = findings_for(
		"```rust\nfn a() {\n    work();\n    if x {\n        work();\n    }\n}\n```\n",
		"readability/blank-line-before-control-flow",
	);

	let finding = findings.first().expect("a finding");

	assert!(
		finding.message.contains("rust"),
		"the message should say which example it is about, got {:?}",
		finding.message
	);
}

#[test]
fn complexity_is_not_scored_inside_a_fence() {
	// A documentation example often walks through a messy state on purpose. Penalizing complexity there
	// would push authors toward hiding the thing they are explaining.
	let names = rules(
		"```rust\nfn a(x: i32) -> i32 {\n    if x > 0 {\n        if x > 1 {\n            if x > 2 {\n                return x;\n            }\n        }\n    }\n    x\n}\n```\n",
	);

	assert!(
		!names.iter().any(|name| name.starts_with("complexity/")),
		"fences should not be scored for complexity, got {names:?}"
	);
}

#[test]
fn several_fences_are_each_scored() {
	let document = "\
```rust
fn a() {
    work();
    if x {
        work();
    }
}
```

```python
def b():
    work()
    if x:
        pass
```
";
	let findings = findings_for(document, "readability/blank-line-before-control-flow");
	let lines: Vec<usize> = findings
		.iter()
		.map(|finding| finding.span.start_line)
		.collect();

	// Findings from both fences should be present, and they should be in different parts of the file.
	assert!(
		findings.len() >= 2,
		"both fences should be scored: {findings:?}"
	);
	assert!(
		lines.iter().any(|line| *line < 10) && lines.iter().any(|line| *line > 10),
		"findings should come from both fences, got lines {lines:?}"
	);
}

#[test]
fn fence_scoring_can_be_turned_off() {
	let source = "```rust\nfn a() {\n    work();\n    if x {\n        work();\n    }\n}\n```\n";
	let lexed = lex(source, Language::Markdown);

	let config = RulesConfig {
		score_markdown_fences: false,

		..RulesConfig::default()
	};

	assert_eq!(
		markdown::fence_readability(&lexed, &config),
		[] as [monostyle_core::Finding; 0]
	);
}

#[test]
fn a_fence_with_extra_info_still_resolves_its_language() {
	let names = rules("```rust,no_run\nfn a() {}\n```\n");

	assert!(
		!names.contains(&"markdown/fence-language-unknown".to_string()),
		"`rust,no_run` should resolve to Rust, got {names:?}"
	);
}

// ---------------------------------------------------------------------------
// Prose and structure
// ---------------------------------------------------------------------------

#[test]
fn a_long_prose_run_is_reported() {
	let prose: String = (0..20)
		.map(|index| {
			format!(
				"Line {index} of unbroken text.
"
			)
		})
		.fold(String::new(), |mut text, line| {
			text.push_str(&line);

			text
		});
	let names = rules(&prose);

	assert!(
		names.contains(&"markdown/prose-run".to_string()),
		"a wall of text should be reported, got {names:?}"
	);
}

#[test]
fn a_document_without_a_title_is_reported() {
	let names = rules("## A heading at level two\n\nSome text.\n");

	assert!(
		names.contains(&"markdown/no-title".to_string()),
		"a document starting below level one should be reported, got {names:?}"
	);
}

#[test]
fn a_document_with_a_title_is_not_reported() {
	let names = rules("# A title\n\n## A section\n\nSome text.\n");

	assert!(
		!names.contains(&"markdown/no-title".to_string()),
		"a document with a title is fine, got {names:?}"
	);
}

#[test]
fn a_skipped_heading_level_is_reported() {
	let names = rules("# Title\n\n### Skipped level two\n\nText.\n");

	assert!(
		names.contains(&"markdown/skipped-heading-level".to_string()),
		"a skipped level should be reported, got {names:?}"
	);
}

#[test]
fn sequential_headings_are_accepted() {
	let names = rules("# Title\n\n## Section\n\n### Subsection\n\nText.\n");

	assert!(
		!names.contains(&"markdown/skipped-heading-level".to_string()),
		"sequential levels are correct, got {names:?}"
	);
}

#[test]
fn markdown_rules_only_apply_to_markdown() {
	// A Rust file containing a `#` line is not a document, so the Markdown rules must not touch it.
	let lexed = lex("// a comment\nfn a() {}\n", Language::Rust);

	assert_eq!(
		markdown::prose_runs(&lexed, &RulesConfig::default()),
		[] as [monostyle_core::Finding; 0]
	);
	assert_eq!(
		markdown::heading_structure(&lexed),
		[] as [monostyle_core::Finding; 0]
	);
}

#[test]
fn an_empty_document_produces_no_findings() {
	assert_eq!(analyze(""), [] as [monostyle_core::Finding; 0]);
}

#[test]
fn a_prose_run_can_be_tuned() {
	let prose: String = (0..8).map(|index| format!("Line {index} of text.\n")).fold(
		String::new(),
		|mut text, line| {
			text.push_str(&line);

			text
		},
	);
	let lexed = lex(&prose, Language::Markdown);

	let strict = RulesConfig {
		max_prose_run: 2,

		..RulesConfig::default()
	};
	let relaxed = RulesConfig {
		max_prose_run: 100,

		..RulesConfig::default()
	};

	assert_ne!(
		markdown::prose_runs(&lexed, &strict),
		[] as [monostyle_core::Finding; 0]
	);
	assert_eq!(
		markdown::prose_runs(&lexed, &relaxed),
		[] as [monostyle_core::Finding; 0]
	);
}

#[test]
fn a_numbered_prose_list_is_not_read_as_code() {
	// A Markdown file is lexed with a profile that has no comments or strings, so every prose line is
	// classified as code. The layout rules then read a list marker as a numeric literal and a list of
	// steps as an unbroken statement run — findings about source code, reported against prose, with no
	// edit that could clear them. Prose is not code, so the code rules do not run against it.
	let findings = analyze(
		"# Steps\n\n\
		 1. First step of the process.\n\
		 2. Second step of the process.\n\
		 3. Third step of the process.\n\
		 4. Fourth step of the process.\n\
		 5. Fifth step of the process.\n\
		 6. Sixth step of the process.\n",
	);

	assert!(
		findings.is_empty(),
		"a prose list should produce no code findings: {findings:?}"
	);
}

#[test]
fn code_inside_a_fence_is_still_scored() {
	// The other half: skipping the code rules for prose must not stop them from scoring the code inside
	// a fence, which is the whole reason documentation is analyzed.
	let findings = analyze(
		"# Example\n\nSome prose.\n\n\
		 ```rust\nfn demo() {\n    let a = 1;\n    if a > 0 {\n        work();\n    }\n}\n```\n",
	);

	assert!(
		findings
			.iter()
			.any(|finding| finding.rule == "readability/blank-line-before-control-flow"),
		"a fence should still be scored: {findings:?}"
	);
}

#[test]
fn markdown_structure_rules_still_run_on_prose() {
	// The rules written for documentation declare themselves for prose, so restricting the code rules
	// must not silence them.
	let findings = analyze("# Title\n\n## Skipped to four\n\n#### Too deep\n");

	assert!(
		findings
			.iter()
			.any(|finding| finding.rule == "markdown/skipped-heading-level"),
		"a skipped heading level should still be reported: {findings:?}"
	);
}

#[test]
fn each_fence_problem_can_be_disabled_on_its_own() {
	// The two fence findings used to be emitted from inside `markdown/fence-readability`, so naming
	// either one in `disabled-rules` did nothing. They are registry entries now, which is what makes a
	// project able to say that untagged fences are fine while misspelled ones are not.
	let source = "# Title\n\n```\nplain\n```\n\n```notalanguage\nx\n```\n";
	let disabled = |rule: &str| {
		let config = RulesConfig {
			disabled_rules: vec![rule.to_string()],
			..RulesConfig::default()
		};
		let lexed = lex(source, Language::Markdown);

		run_rules(&lexed, &config)
	};

	let all = disabled("markdown/nothing");
	let names: Vec<String> = all.iter().map(|finding| finding.rule.clone()).collect();

	assert!(
		names.contains(&"markdown/fence-without-language".to_string()),
		"the untagged fence should be reported first: {names:?}"
	);
	assert!(
		names.contains(&"markdown/fence-language-unknown".to_string()),
		"the unknown-language fence should be reported first: {names:?}"
	);

	let without_tag_rule = disabled("markdown/fence-without-language");
	assert!(
		!without_tag_rule
			.iter()
			.any(|finding| finding.rule == "markdown/fence-without-language"),
		"disabling the untagged-fence rule should silence it: {without_tag_rule:?}"
	);
	assert!(
		without_tag_rule
			.iter()
			.any(|finding| finding.rule == "markdown/fence-language-unknown"),
		"disabling one fence rule must not silence the other: {without_tag_rule:?}"
	);
}

#[test]
fn each_heading_problem_can_be_disabled_on_its_own() {
	// The document starts below level one *and* jumps two levels, so both rules have something to
	// report and disabling one leaves the other's finding in place.
	let source = "## No title\n\n#### Skipped two levels\n\nText.\n";
	let lexed = lex(source, Language::Markdown);
	let findings = run_rules(
		&lexed,
		&RulesConfig {
			disabled_rules: vec!["markdown/no-title".to_string()],
			..RulesConfig::default()
		},
	);

	assert!(
		!findings
			.iter()
			.any(|finding| finding.rule == "markdown/no-title"),
		"the disabled heading rule should be silent: {findings:?}"
	);
	assert!(
		findings
			.iter()
			.any(|finding| finding.rule == "markdown/skipped-heading-level"),
		"the other heading rule should still run: {findings:?}"
	);
}

#[test]
fn the_retired_composite_name_still_disables_its_rules() {
	// `markdown/heading-structure` was documented as a rule while living inside another one. A project
	// that configured it must keep the behavior it asked for rather than having its scores change
	// silently, so the old name still turns both replacement rules off.
	let source = "## No title\n\n#### Skipped two levels\n\nText.\n";
	let lexed = lex(source, Language::Markdown);
	let findings = run_rules(
		&lexed,
		&RulesConfig {
			disabled_rules: vec!["markdown/heading-structure".to_string()],
			..RulesConfig::default()
		},
	);

	assert!(
		!findings.iter().any(|finding| {
			finding.rule == "markdown/no-title" || finding.rule == "markdown/skipped-heading-level"
		}),
		"the retired composite name should disable both rules: {findings:?}"
	);
}
