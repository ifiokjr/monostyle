//! The rule registry.
//!
//! One place lists every rule, which is what makes `monostyle rules` able to print the full
//! set and `disabled-rules` able to turn any of them off by name.

use monostyle_core::Finding;
use monostyle_lexer::LexedFile;

use crate::comments;
use crate::complexity;
use crate::config::RulesConfig;
use crate::line_length;
use crate::markdown;
use crate::quality;
use crate::structure;
use crate::whitespace;

/// A rule's name and the function that runs it.
pub struct Rule {
	/// The stable identifier used in configuration and reports.
	pub name: &'static str,
	/// One sentence describing what the rule looks for.
	pub description: &'static str,
	/// Runs the rule.
	pub run: fn(&LexedFile, &RulesConfig) -> Vec<Finding>,
}

/// Every rule, in report order.
///
/// Whitespace rules come first so that a report reads from the most visible problems to the
/// most structural.
#[must_use]
pub fn all_rules() -> Vec<Rule> {
	vec![
		Rule {
			name: "readability/blank-line-before-control-flow",
			description: "Requires a blank line before each control-flow statement.",
			run: whitespace::blank_line_before_control_flow,
		},
		Rule {
			name: "readability/blank-line-before-return",
			description: "Requires a blank line before a return that follows complex code.",
			run: whitespace::blank_line_before_return,
		},
		Rule {
			name: "readability/group-separation",
			description: "Requires blank lines between logical groups of statements.",
			run: whitespace::group_separation,
		},
		Rule {
			name: "readability/excessive-indentation",
			description: "Flags lines indented beyond the configured width.",
			run: whitespace::excessive_indentation,
		},
		Rule {
			name: "readability/mixed-indentation",
			description: "Flags files that mix tabs and spaces.",
			run: |file, _config| whitespace::mixed_indentation(file),
		},
		Rule {
			name: "readability/deep-nesting",
			description: "Flags control flow nested beyond the configured depth.",
			run: structure::deep_nesting,
		},
		Rule {
			name: "readability/long-parameter-list",
			description: "Flags calls whose arguments should be split across lines.",
			run: structure::long_parameter_list,
		},
		Rule {
			name: "readability/overlong-line",
			description: "Flags lines wider than the configured limit.",
			run: line_length::overlong_lines,
		},
		Rule {
			name: "readability/oversized-unit",
			description: "Flags functions longer than the configured limit.",
			run: structure::oversized_units,
		},
		Rule {
			name: "readability/oversized-file",
			description: "Flags files longer than the configured limit.",
			run: structure::oversized_file,
		},
		Rule {
			name: "readability/comment-required-on-complex-unit",
			description: "Requires an explanatory comment on complex functions.",
			run: comments::comment_required_on_complex_units,
		},
		Rule {
			name: "readability/comment-explains-why",
			description: "Credits comments that explain why rather than what.",
			run: comments::comments_explaining_why,
		},
		Rule {
			name: "readability/comment-narrates-code",
			description: "Penalizes comments that restate what the code already says.",
			run: comments::comments_narrating_code,
		},
		Rule {
			name: "readability/excessive-comments",
			description: "Flags files where comments outweigh the code.",
			run: comments::excessive_comments,
		},
		Rule {
			name: "readability/thin-documentation",
			description: "Flags documentation blocks that list structure without explaining purpose.",
			run: |file, _config| comments::thin_documentation(file),
		},
		Rule {
			name: "readability/magic-number",
			description: "Flags meaningful numeric literals that should be named constants.",
			run: quality::magic_numbers,
		},
		Rule {
			name: "readability/short-identifier",
			description: "Flags identifiers too short to convey meaning.",
			run: quality::short_identifiers,
		},
		Rule {
			name: "readability/empty-handler",
			description: "Flags error handlers that discard the error.",
			run: quality::empty_handlers,
		},
		Rule {
			name: "readability/commented-out-code",
			description: "Flags blocks of code that were commented out instead of deleted.",
			run: quality::commented_out_code,
		},
		Rule {
			name: "complexity/cyclomatic-per-unit",
			description: "Flags functions with too many independent paths.",
			run: complexity::cyclomatic_per_unit,
		},
		Rule {
			name: "complexity/cognitive-per-unit",
			description: "Flags functions that are hard to follow.",
			run: complexity::cognitive_per_unit,
		},
		Rule {
			name: "complexity/npath-per-unit",
			description: "Flags functions with too many execution paths.",
			run: complexity::npath_per_unit,
		},
		Rule {
			name: "complexity/exits-per-unit",
			description: "Flags functions that return from too many places.",
			run: complexity::exits_per_unit,
		},
		Rule {
			name: "complexity/low-maintainability",
			description: "Flags functions with a low maintainability index.",
			run: complexity::maintainability_per_unit,
		},
		Rule {
			name: "complexity/cyclomatic-per-file",
			description: "Flags files with a high decision density.",
			run: complexity::complexity_per_file,
		},
		Rule {
			name: "markdown/fence-readability",
			description: "Scores the code inside Markdown fences.",
			run: markdown::fence_readability,
		},
		Rule {
			name: "markdown/prose-run",
			description: "Flags long unbroken runs of prose.",
			run: markdown::prose_runs,
		},
		Rule {
			name: "markdown/heading-structure",
			description: "Flags skipped heading levels and missing document titles.",
			run: |file, _config| markdown::heading_structure(file),
		},
	]
}

/// Runs every enabled rule against `file`.
#[must_use]
pub fn run_rules(file: &LexedFile, config: &RulesConfig) -> Vec<Finding> {
	let mut findings = Vec::new();

	for rule in all_rules() {
		if !config.is_enabled(rule.name) {
			continue;
		}

		findings.extend((rule.run)(file, config));
	}

	findings
}
