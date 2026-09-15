//! Tests for the Markdown analyzer.
//!
//! Markdown is where code people copy lives, so the extraction has to be right in both directions: a
//! fence must be found with its language and its exact line range, and prose must never be mistaken
//! for a fence.

use std::fmt::Write as _;

use monostyle_core::Language;
use monostyle_markdown::Heading;
use monostyle_markdown::analyze;
use monostyle_markdown::skipped_headings;

#[test]
fn a_fenced_block_is_extracted_with_its_language() {
	let document = analyze("# T\n\n```rust\nfn a() {}\n```\n");

	assert_eq!(document.fences.len(), 1);

	let fence = &document.fences[0];

	assert_eq!(fence.language, Some(Language::Rust));
	assert_eq!(fence.code, "fn a() {}");
	assert_eq!(fence.marker, '`');
	assert!(fence.is_recognized());
}

#[test]
fn a_tilde_fence_is_recognized() {
	let document = analyze("~~~python\nx = 1\n~~~\n");

	assert_eq!(document.fences.len(), 1);
	assert_eq!(document.fences[0].marker, '~');
	assert_eq!(document.fences[0].language, Some(Language::Python));
}

#[test]
fn fence_line_numbers_point_at_the_document() {
	// The line numbers are what a reader jumps to, so an off-by-one makes every finding in a fence
	// point at the wrong place.
	let source = "# Title\n\nIntro text.\n\n```rust\nfn a() {}\n```\n";
	let document = analyze(source);

	let fence = &document.fences[0];

	assert_eq!(fence.start_line, 5, "the opening fence is on line five");
	assert_eq!(fence.end_line, 7, "the closing fence is on line seven");
	assert_eq!(fence.line_count(), 1);
}

#[test]
fn a_multiline_fence_keeps_every_line() {
	let document = analyze("```rust\nfn a() {\n    work();\n}\n```\n");

	assert_eq!(document.fences[0].line_count(), 3);
	assert!(document.fences[0].code.contains("work();"));
}

#[test]
fn an_untagged_fence_is_reported_as_unrecognized() {
	let document = analyze("```\nsome text\n```\n");

	assert_eq!(document.fences.len(), 1);
	assert!(!document.fences[0].is_recognized());
	assert_eq!(document.fences[0].info_string, "");
}

#[test]
fn an_unknown_language_keeps_its_info_string() {
	let document = analyze("```notalanguage\nx\n```\n");

	assert!(!document.fences[0].is_recognized());
	assert_eq!(document.fences[0].info_string, "notalanguage");
}

#[test]
fn inline_code_is_not_a_fence() {
	// A backtick fence's info string may not contain a backtick, which is what keeps a line of inline
	// code from being read as the start of a block.
	let document = analyze("Use `cargo test` to run them.\n\nStill prose.\n");

	assert!(document.fences.is_empty(), "inline code is not a fence");
}

#[test]
fn a_fence_needs_three_markers() {
	let document = analyze("``\nnot a fence\n``\n");

	assert!(document.fences.is_empty(), "two backticks is not a fence");
}

#[test]
fn several_fences_are_all_extracted() {
	let document = analyze("```rust\na\n```\n\n```python\nb\n```\n");

	assert_eq!(document.fences.len(), 2);
	assert_eq!(document.fences[0].language, Some(Language::Rust));
	assert_eq!(document.fences[1].language, Some(Language::Python));
}

#[test]
fn an_unterminated_fence_takes_the_rest_of_the_document() {
	let document = analyze("```rust\nfn a() {}\n");

	assert_eq!(document.fences.len(), 1);
	assert_eq!(document.fences[0].code, "fn a() {}");
}

#[test]
fn a_longer_closing_fence_is_accepted() {
	let document = analyze("```rust\nfn a() {}\n`````\n");

	assert_eq!(document.fences.len(), 1);
	assert_eq!(document.fences[0].code, "fn a() {}");
}

// ---------------------------------------------------------------------------
// Headings
// ---------------------------------------------------------------------------

#[test]
fn headings_are_extracted_with_their_levels() {
	let document = analyze("# One\n\n## Two\n\n### Three\n");

	assert_eq!(document.headings.len(), 3);
	assert_eq!(document.headings[0].level, 1);
	assert_eq!(document.headings[0].text, "One");
	assert_eq!(document.headings[1].level, 2);
	assert_eq!(document.headings[2].level, 3);
}

#[test]
fn trailing_hashes_are_stripped_from_a_heading() {
	let document = analyze("## Title ##\n");

	assert_eq!(document.headings[0].text, "Title");
}

#[test]
fn more_than_six_hashes_is_not_a_heading() {
	let document = analyze("####### too many\n");

	assert!(document.headings.is_empty());
}

#[test]
fn a_skipped_level_is_reported() {
	let headings = vec![
		Heading {
			level: 1,
			text: "One".to_string(),
			line: 1,
		},
		Heading {
			level: 3,
			text: "Three".to_string(),
			line: 5,
		},
	];

	let skipped = skipped_headings(&headings);

	assert_eq!(skipped.len(), 1);
	assert_eq!(skipped[0].from, 1);
	assert_eq!(skipped[0].to, 3);
	assert_eq!(skipped[0].line, 5);
}

#[test]
fn sequential_levels_are_not_reported() {
	let headings = vec![
		Heading {
			level: 1,
			text: "One".to_string(),
			line: 1,
		},
		Heading {
			level: 2,
			text: "Two".to_string(),
			line: 3,
		},
		Heading {
			level: 3,
			text: "Three".to_string(),
			line: 5,
		},
	];

	assert!(skipped_headings(&headings).is_empty());
}

#[test]
fn a_level_repeated_is_not_a_skip() {
	let headings = vec![
		Heading {
			level: 2,
			text: "A".to_string(),
			line: 1,
		},
		Heading {
			level: 2,
			text: "B".to_string(),
			line: 3,
		},
	];

	assert!(skipped_headings(&headings).is_empty());
}

// ---------------------------------------------------------------------------
// Prose
// ---------------------------------------------------------------------------

#[test]
fn a_prose_run_is_recorded_with_its_length() {
	// The analyzer records every run and leaves the threshold to the rule, so `max-prose-run` is a real
	// setting rather than one the analysis has already applied.
	let prose: String = (0..20)
		.map(|index| format!("Line {index} of an unbroken paragraph.\n"))
		.fold(String::new(), |mut text, line| {
			text.push_str(&line);

			text
		});
	let document = analyze(&prose);

	assert_eq!(document.prose_runs.len(), 1);
	assert_eq!(document.prose_runs[0].length, 20);
}

#[test]
fn a_short_prose_run_is_still_recorded() {
	let document = analyze("One short paragraph.\n");

	assert_eq!(document.prose_runs.len(), 1);
	assert_eq!(document.prose_runs[0].length, 1);
}

#[test]
fn structure_breaks_a_prose_run() {
	// A heading, a list, or a fence gives the reader somewhere to rest, so a run ends there.
	let mut source: String =
		(0..10)
			.map(|index| format!("Line {index}.\n"))
			.fold(String::new(), |mut text, line| {
				text.push_str(&line);

				text
			});
	source.push_str("\n## A heading\n\n");
	for index in 0..10 {
		let _ = writeln!(source, "More line {index}.");
	}

	let document = analyze(&source);

	assert_eq!(
		document.prose_runs.len(),
		2,
		"the heading should split the runs"
	);
}

#[test]
fn a_list_is_not_prose() {
	let list: String =
		(0..20)
			.map(|index| format!("- item {index}\n"))
			.fold(String::new(), |mut text, line| {
				text.push_str(&line);

				text
			});
	let document = analyze(&list);

	assert!(
		document.prose_runs.is_empty(),
		"a list is structure, not a paragraph"
	);
}

#[test]
fn an_empty_document_is_handled() {
	let document = analyze("");

	assert_eq!(document.total_lines, 0);
	assert!(document.headings.is_empty());
	assert!(document.fences.is_empty());
}

#[test]
fn total_lines_matches_the_source() {
	let document = analyze("a\nb\nc\n");

	assert_eq!(document.total_lines, 3);
}

#[test]
fn a_fence_inside_a_list_is_still_extracted() {
	// Documentation indents fences under list items, which is common enough that missing them would
	// skip a large share of real examples.
	let source = "- Step one:\n\n  ```rust\n  fn a() {}\n  ```\n";
	let document = analyze(source);

	assert_eq!(document.fences.len(), 1);
	assert_eq!(document.fences[0].language, Some(Language::Rust));
}
