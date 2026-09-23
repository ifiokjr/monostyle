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
	/// Whether this rule runs against a prose document.
	///
	/// A Markdown file is lexed with a profile that has no comments, strings, or decisions, so every
	/// line of prose is classified as code. That is deliberate — it lets the layout rules measure the
	/// code inside a fence — but it means a rule written for source code will happily read a numbered
	/// list as a statement run and a list marker as a magic number. Rules that analyze prose say so;
	/// every other rule is skipped for a Markdown file rather than being left to guard itself, because
	/// a guard that one rule forgets is a false positive nobody can explain from the config.
	pub prose: bool,
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
			prose: false,
			run: whitespace::blank_line_before_control_flow,
		},
		Rule {
			name: "readability/blank-line-after-control-flow",
			description: "Requires a blank line after a control-flow block when statements follow it.",
			prose: false,
			run: whitespace::blank_line_after_control_flow,
		},
		Rule {
			name: "readability/detached-comment",
			description: "Flags comments separated by a blank line from the code they document.",
			prose: false,
			run: whitespace::detached_comment,
		},
		Rule {
			name: "readability/blank-line-before-return",
			description: "Requires a blank line before a return that follows complex code.",
			prose: false,
			run: whitespace::blank_line_before_return,
		},
		Rule {
			name: "readability/group-separation",
			description: "Requires blank lines between logical groups of statements.",
			prose: false,
			run: whitespace::group_separation,
		},
		Rule {
			name: "readability/excessive-blank-lines",
			description: "Flags runs of blank lines longer than the configured maximum.",
			prose: false,
			run: whitespace::excessive_blank_lines,
		},
		Rule {
			name: "readability/excessive-indentation",
			description: "Flags lines indented beyond the configured width.",
			prose: false,
			run: whitespace::excessive_indentation,
		},
		Rule {
			name: "readability/mixed-indentation",
			description: "Flags files that mix tabs and spaces.",
			prose: false,
			run: |file, _config| whitespace::mixed_indentation(file),
		},
		Rule {
			name: "readability/deep-nesting",
			description: "Flags control flow nested beyond the configured depth.",
			prose: false,
			run: structure::deep_nesting,
		},
		Rule {
			name: "readability/long-parameter-list",
			description: "Flags calls whose arguments should be split across lines.",
			prose: false,
			run: structure::long_parameter_list,
		},
		Rule {
			name: "readability/overlong-line",
			description: "Flags lines wider than the configured limit.",
			prose: false,
			run: line_length::overlong_lines,
		},
		Rule {
			name: "readability/oversized-unit",
			description: "Flags functions longer than the configured limit.",
			prose: false,
			run: structure::oversized_units,
		},
		Rule {
			name: "readability/oversized-file",
			description: "Flags files longer than the configured limit.",
			prose: false,
			run: structure::oversized_file,
		},
		Rule {
			name: "readability/comment-required-on-complex-unit",
			description: "Requires an explanatory comment on complex functions.",
			prose: false,
			run: comments::comment_required_on_complex_units,
		},
		Rule {
			name: "readability/comment-explains-why",
			description: "Credits comments that explain why rather than what.",
			prose: false,
			run: comments::comments_explaining_why,
		},
		Rule {
			name: "readability/comment-narrates-code",
			description: "Penalizes comments that restate what the code already says.",
			prose: false,
			run: comments::comments_narrating_code,
		},
		Rule {
			name: "readability/excessive-comments",
			description: "Flags files where comments outweigh the code.",
			prose: false,
			run: comments::excessive_comments,
		},
		Rule {
			name: "readability/thin-documentation",
			description: "Flags documentation blocks that list structure without explaining purpose.",
			prose: false,
			run: |file, _config| comments::thin_documentation(file),
		},
		Rule {
			name: "readability/magic-number",
			description: "Flags meaningful numeric literals that should be named constants.",
			prose: false,
			run: quality::magic_numbers,
		},
		Rule {
			name: "readability/short-identifier",
			description: "Flags identifiers too short to convey meaning.",
			prose: false,
			run: quality::short_identifiers,
		},
		Rule {
			name: "readability/empty-handler",
			description: "Flags error handlers that discard the error.",
			prose: false,
			run: quality::empty_handlers,
		},
		Rule {
			name: "readability/commented-out-code",
			description: "Flags blocks of code that were commented out instead of deleted.",
			prose: false,
			run: quality::commented_out_code,
		},
		Rule {
			name: "complexity/cyclomatic-per-unit",
			description: "Flags functions with too many independent paths.",
			prose: false,
			run: complexity::cyclomatic_per_unit,
		},
		Rule {
			name: "complexity/cognitive-per-unit",
			description: "Flags functions that are hard to follow.",
			prose: false,
			run: complexity::cognitive_per_unit,
		},
		Rule {
			name: "complexity/npath-per-unit",
			description: "Flags functions with too many execution paths.",
			prose: false,
			run: complexity::npath_per_unit,
		},
		Rule {
			name: "complexity/exits-per-unit",
			description: "Flags functions that return from too many places.",
			prose: false,
			run: complexity::exits_per_unit,
		},
		Rule {
			name: "complexity/low-maintainability",
			description: "Flags functions with a low maintainability index.",
			prose: false,
			run: complexity::maintainability_per_unit,
		},
		Rule {
			name: "complexity/cyclomatic-per-file",
			description: "Flags files with a high decision density.",
			prose: false,
			run: complexity::complexity_per_file,
		},
		Rule {
			name: "markdown/fence-readability",
			description: "Scores the code inside Markdown fences.",
			prose: true,
			run: markdown::fence_readability,
		},
		Rule {
			name: "markdown/fence-without-language",
			description: "Flags fences with no language tag.",
			prose: true,
			run: markdown::fence_without_language,
		},
		Rule {
			name: "markdown/fence-language-unknown",
			description: "Flags fences declaring a language monostyle cannot read.",
			prose: true,
			run: markdown::fence_language_unknown,
		},
		Rule {
			name: "markdown/prose-run",
			description: "Flags long unbroken runs of prose.",
			prose: true,
			run: markdown::prose_runs,
		},
		Rule {
			name: "markdown/no-title",
			description: "Flags documents that do not start with a top-level heading.",
			prose: true,
			run: |file, _config| markdown::missing_title(file),
		},
		Rule {
			name: "markdown/skipped-heading-level",
			description: "Flags heading levels that skip a step.",
			prose: true,
			run: |file, _config| markdown::skipped_heading_levels(file),
		},
	]
}

/// Rule names that are no longer registered but still disable their replacement.
///
/// `markdown/heading-structure` bundled two checks, and `markdown/fence-readability` used to emit the
/// fence-language findings itself. Both were documented as standalone rules while living inside another
/// one, so a project that wrote `disabled-rules = ["markdown/no-title"]` was silently ignored. The names
/// are kept here so that configuration which named the old composite still turns the right thing off
/// rather than quietly changing the project's scores.
const ALIASES: &[(&str, &[&str])] = &[(
	"markdown/heading-structure",
	&["markdown/no-title", "markdown/skipped-heading-level"],
)];

/// Expands alias names to the rules they stand for.
#[must_use]
pub fn expand_disabled(disabled: &[String]) -> Vec<String> {
	let mut expanded = disabled.to_vec();

	for (alias, replacements) in ALIASES {
		if !disabled.iter().any(|name| name == alias) {
			continue;
		}

		for replacement in *replacements {
			if !expanded.iter().any(|name| name == replacement) {
				expanded.push((*replacement).to_string());
			}
		}
	}

	expanded
}

/// Runs every enabled rule against `file`.
///
/// A Markdown file only runs the rules that declared themselves for prose, because its lexed form
/// looks like source code to a rule that was written for a real language. This is enforced here rather
/// than in each rule so that forgetting a guard in a new rule cannot produce prose findings that the
/// report cannot justify.
#[must_use]
pub fn run_rules(file: &LexedFile, config: &RulesConfig) -> Vec<Finding> {
	let mut findings = Vec::new();
	let prose = file.language == monostyle_core::Language::Markdown;
	let disabled = expand_disabled(&config.disabled_rules);

	for rule in all_rules() {
		if disabled.iter().any(|name| name == rule.name) {
			continue;
		}

		if prose && !rule.prose {
			continue;
		}

		findings.extend((rule.run)(file, config));
	}

	findings
}
