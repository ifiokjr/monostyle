//! Rule configuration.
//!
//! Thresholds live in configuration rather than in rule bodies so that a project can tighten or
//! relax a rule without forking the tool, and so that a report can always state the threshold it
//! measured against.

use monostyle_core::IgnoreConfig;
use serde::Deserialize;
use serde::Serialize;

/// Thresholds and toggles for the rule set.
///
/// The boolean fields are independent switches rather than a state machine, so they stay flat:
/// grouping them into sub-structs would add a level of nesting to read without hiding anything.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct RulesConfig {
	/// Rules that are turned off entirely.
	pub disabled_rules: Vec<String>,

	// --- Whitespace ---
	/// Whether sequential control flow must be separated by blank lines.
	pub require_blank_line_before_control_flow: bool,
	/// Minimum gap, in lines, between two consecutive control-flow statements.
	pub min_blank_lines_between_control_flow: usize,
	/// Whether a blank line is required before a trailing `return` in a multi-statement body.
	pub require_blank_line_before_return: bool,
	/// Whether logical groups of statements must be separated.
	pub require_group_separation: bool,
	/// Statements in an unbroken run before a blank line is expected.
	///
	/// The rule reports a run only when it is longer than this, so the default of 8 means eight
	/// statements in a row are one group and the ninth is where a break belongs. Raising it is how a
	/// project says its functions are longer than the default taste allows.
	pub max_statements_per_group: usize,
	/// Most consecutive blank lines allowed inside a block before they are reported.
	///
	/// A tool that asks for blank lines has to say when there are too many, or its own advice becomes
	/// the problem: every rule here rewards a gap, so an auto-fixer following them without a ceiling
	/// can grow a gap without limit. One blank line is almost always what the rule meant, so the
	/// default is 1 and a run of two or more is reported.
	pub max_consecutive_blank_lines: usize,

	// --- Structure ---
	/// Maximum nesting depth before a finding is raised.
	pub max_nesting_depth: usize,
	/// Maximum parameters before a call or declaration must be split across lines.
	pub max_parameters_inline: usize,
	/// Maximum indentation width before a line is considered over-indented.
	pub max_indent_width: usize,
	/// Maximum line width, in columns, before a line is considered overlong.
	///
	/// Set this to match the project's formatter rather than leaving the default, so the rule agrees
	/// with the tool that actually wraps the code.
	pub max_line_width: usize,
	/// How far past the limit a line must be before the finding is severe.
	///
	/// Expressed as a multiple of `max-line-width`, so raising the limit for a project that formats
	/// wide does not also move the point at which a line becomes a serious problem.
	pub severe_line_width_ratio: f64,

	// --- Comments ---
	/// Whether a complex unit must carry an explanatory comment.
	pub require_comment_on_complex_units: bool,
	/// Cognitive complexity above which a unit requires a comment.
	pub comment_required_above_cognitive: usize,
	/// Whether comments that narrate code are penalized.
	pub penalize_narrating_comments: bool,
	/// Maximum comment-to-code ratio before comments are considered excessive.
	pub max_comment_ratio: f64,

	// --- Complexity ---
	/// Cyclomatic complexity above which a unit is too complex.
	pub max_cyclomatic_per_unit: usize,
	/// Cognitive complexity above which a unit is too complex.
	pub max_cognitive_per_unit: usize,
	/// Lines above which a unit is too long to hold in your head.
	pub max_unit_lines: usize,
	/// Lines above which a file is too large to navigate.
	pub max_file_lines: usize,
	/// `NPath` count above which a unit has too many execution paths to reason about.
	pub max_npath_per_unit: usize,
	/// Exits above which a unit returns from too many places.
	pub max_exits_per_unit: usize,
	/// Maintainability index below which a unit is reported.
	pub min_maintainability: f64,

	// --- Identifier and literal quality ---
	/// Whether single-character and overly short identifiers are reported.
	pub report_short_identifiers: bool,
	/// Minimum identifier length, excluding conventional loop and coordinate names.
	pub min_identifier_length: usize,
	/// Whether numeric literals outside conventional values are reported as magic numbers.
	pub report_magic_numbers: bool,
	/// Whether exception handlers that swallow errors are reported.
	pub report_empty_handlers: bool,
	/// Whether blocks of commented-out code are reported.
	pub report_commented_out_code: bool,

	// --- Markdown ---
	/// Whether code inside Markdown fences is scored.
	pub score_markdown_fences: bool,
	/// Maximum consecutive prose lines in Markdown before structure is expected.
	pub max_prose_run: usize,

	// --- Ignoring ---
	/// Which paths to skip.
	pub ignore: IgnoreConfig,
}

impl Default for RulesConfig {
	fn default() -> Self {
		Self {
			disabled_rules: Vec::new(),

			require_blank_line_before_control_flow: true,
			min_blank_lines_between_control_flow: 1,
			require_blank_line_before_return: true,
			require_group_separation: true,
			max_statements_per_group: 8,
			max_consecutive_blank_lines: 1,

			max_nesting_depth: 3,
			max_parameters_inline: 3,
			max_indent_width: 24,
			max_line_width: 120,
			severe_line_width_ratio: 1.35,

			require_comment_on_complex_units: true,
			comment_required_above_cognitive: 10,
			penalize_narrating_comments: true,
			max_comment_ratio: 0.6,

			max_cyclomatic_per_unit: 10,
			max_cognitive_per_unit: 15,
			max_unit_lines: 80,
			max_file_lines: 600,
			max_npath_per_unit: 1024,
			max_exits_per_unit: 6,
			min_maintainability: 40.0,

			report_short_identifiers: true,
			min_identifier_length: 3,
			report_magic_numbers: true,
			report_empty_handlers: true,
			report_commented_out_code: true,

			score_markdown_fences: true,
			max_prose_run: 12,

			ignore: IgnoreConfig::default(),
		}
	}
}

impl RulesConfig {
	/// Whether `rule` is enabled.
	#[must_use]
	pub fn is_enabled(&self, rule: &str) -> bool {
		!self.disabled_rules.iter().any(|disabled| disabled == rule)
	}

	/// Width past which a line is a severe problem rather than a minor one.
	#[must_use]
	pub fn severe_line_width(&self) -> usize {
		#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
		let scaled = (self.max_line_width as f64 * self.severe_line_width_ratio).round() as usize;

		scaled.max(self.max_line_width + 1)
	}
}
