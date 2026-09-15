//! Rule configuration.
//!
//! Thresholds live in configuration rather than in rule bodies so that a project can
//! tighten or relax a rule without forking the tool, and so that a report can always
//! state the threshold it measured against.

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

	// --- Structure ---
	/// Maximum nesting depth before a finding is raised.
	pub max_nesting_depth: usize,
	/// Maximum parameters before a call or declaration must be split across lines.
	pub max_parameters_inline: usize,
	/// Maximum indentation width before a line is considered over-indented.
	pub max_indent_width: usize,

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

	// --- Markdown ---
	/// Whether code inside Markdown fences is scored.
	pub score_markdown_fences: bool,
	/// Maximum consecutive prose lines in Markdown before structure is expected.
	pub max_prose_run: usize,
}

impl Default for RulesConfig {
	fn default() -> Self {
		Self {
			disabled_rules: Vec::new(),

			require_blank_line_before_control_flow: true,
			min_blank_lines_between_control_flow: 1,
			require_blank_line_before_return: true,
			require_group_separation: true,

			max_nesting_depth: 3,
			max_parameters_inline: 3,
			max_indent_width: 24,

			require_comment_on_complex_units: true,
			comment_required_above_cognitive: 10,
			penalize_narrating_comments: true,
			max_comment_ratio: 0.6,

			max_cyclomatic_per_unit: 10,
			max_cognitive_per_unit: 15,
			max_unit_lines: 80,
			max_file_lines: 600,

			score_markdown_fences: true,
			max_prose_run: 12,
		}
	}
}

impl RulesConfig {
	/// Whether `rule` is enabled.
	#[must_use]
	pub fn is_enabled(&self, rule: &str) -> bool {
		!self.disabled_rules.iter().any(|disabled| disabled == rule)
	}
}
