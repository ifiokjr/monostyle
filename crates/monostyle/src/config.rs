//! Configuration file loading.
//!
//! # Why the file format has its own type
//!
//! A configuration file should only have to state what it changes. That requires
//! distinguishing "this field was absent" from "this field was set to the default value",
//! which a struct of plain values cannot do — deserialization fills absent fields with
//! defaults, so merging a partial file would silently reset every threshold the file did not
//! mention.
//!
//! [`RulesOverlay`] solves this by making every field optional. Its [`RulesOverlay::apply`]
//! then copies only the fields that were actually present, using a macro so the intent stays
//! readable instead of becoming a long chain of near-identical comparisons.

use std::path::Path;
use std::path::PathBuf;

use monostyle_core::ScoringConfig;
use monostyle_rules::RulesConfig;
use serde::Deserialize;

use crate::analysis::AnalysisOptions;

/// Applies each named field from `$overlay` onto `$base` when it is present.
///
/// Written as a macro because the operation is mechanical: every rule threshold merges the
/// same way, and spelling that out sixteen times would bury the one line that matters — the
/// `if let` — under repetition.
macro_rules! apply_overlay {
	($base:expr, $overlay:expr, $($field:ident),+ $(,)?) => {
		$(
			if let Some(value) = $overlay.$field {
				$base.$field = value;
			}
		)+
	};
}

/// The on-disk configuration file.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ConfigFile {
	/// Scoring curve settings.
	pub scoring: ScoringSection,
	/// Rule thresholds to override.
	pub rules: RulesOverlay,
}

/// The `[scoring]` section.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ScoringSection {
	/// Penalty density per 100 lines that yields a score of 50.
	pub half_life: Option<f64>,
	/// Lines used as the denominator floor.
	pub min_normalization_lines: Option<f64>,
}

/// Rule thresholds as written in a configuration file.
///
/// Every field is optional, and an absent field leaves the corresponding default in place.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct RulesOverlay {
	/// Rules to turn off entirely.
	pub disabled_rules: Option<Vec<String>>,

	/// Whether sequential control flow must be separated by blank lines.
	pub require_blank_line_before_control_flow: Option<bool>,
	/// Minimum gap, in lines, between two consecutive control-flow statements.
	pub min_blank_lines_between_control_flow: Option<usize>,
	/// Whether a blank line is required before a trailing `return`.
	pub require_blank_line_before_return: Option<bool>,
	/// Whether logical groups of statements must be separated.
	pub require_group_separation: Option<bool>,

	/// Maximum nesting depth before a finding is raised.
	pub max_nesting_depth: Option<usize>,
	/// Maximum parameters before a call must be split across lines.
	pub max_parameters_inline: Option<usize>,
	/// Maximum indentation width before a line is over-indented.
	pub max_indent_width: Option<usize>,

	/// Whether a complex unit must carry an explanatory comment.
	pub require_comment_on_complex_units: Option<bool>,
	/// Cognitive complexity above which a unit requires a comment.
	pub comment_required_above_cognitive: Option<usize>,
	/// Whether comments that narrate code are penalized.
	pub penalize_narrating_comments: Option<bool>,
	/// Maximum comment-to-code ratio before comments are excessive.
	pub max_comment_ratio: Option<f64>,

	/// Cyclomatic complexity above which a unit is too complex.
	pub max_cyclomatic_per_unit: Option<usize>,
	/// Cognitive complexity above which a unit is too complex.
	pub max_cognitive_per_unit: Option<usize>,
	/// Lines above which a unit is too long.
	pub max_unit_lines: Option<usize>,
	/// Lines above which a file is too large.
	pub max_file_lines: Option<usize>,

	/// Whether code inside Markdown fences is scored.
	pub score_markdown_fences: Option<bool>,
	/// Maximum consecutive prose lines before structure is expected.
	pub max_prose_run: Option<usize>,
}

impl RulesOverlay {
	/// Copies every present field onto `base`.
	pub fn apply(self, base: &mut RulesConfig) {
		apply_overlay!(
			base,
			self,
			disabled_rules,
			require_blank_line_before_control_flow,
			min_blank_lines_between_control_flow,
			require_blank_line_before_return,
			require_group_separation,
			max_nesting_depth,
			max_parameters_inline,
			max_indent_width,
			require_comment_on_complex_units,
			comment_required_above_cognitive,
			penalize_narrating_comments,
			max_comment_ratio,
			max_cyclomatic_per_unit,
			max_cognitive_per_unit,
			max_unit_lines,
			max_file_lines,
			score_markdown_fences,
			max_prose_run,
		);
	}
}

impl ConfigFile {
	/// Folds the file's settings into analysis options.
	#[must_use]
	pub fn apply(self, base: &AnalysisOptions) -> AnalysisOptions {
		let mut options = base.clone();

		if let Some(half_life) = self.scoring.half_life {
			options.scoring.half_life = half_life;
		}

		if let Some(minimum) = self.scoring.min_normalization_lines {
			options.scoring.min_normalization_lines = minimum;
		}

		self.rules.apply(&mut options.rules);
		options
	}
}

/// Finds the nearest configuration file, searching upward from `start`.
#[must_use]
pub fn find_config(start: &Path) -> Option<PathBuf> {
	/// Configuration file names, in priority order.
	const NAMES: &[&str] = &["monostyle.toml", ".monostyle.toml"];

	let mut current = Some(start);

	while let Some(directory) = current {
		for name in NAMES {
			let candidate = directory.join(name);

			if candidate.is_file() {
				return Some(candidate);
			}
		}

		current = directory.parent();
	}

	None
}

/// Loads configuration from `path`.
///
/// A malformed configuration file is a hard error rather than a silent fallback: quietly
/// ignoring a threshold a project asked for would produce scores nobody can explain.
pub fn load_config(path: &Path) -> Result<ConfigFile, ConfigError> {
	let contents = std::fs::read_to_string(path).map_err(|source| {
		ConfigError::Read {
			path: path.to_path_buf(),
			source,
		}
	})?;

	toml::from_str(&contents).map_err(|source| {
		ConfigError::Parse {
			path: path.to_path_buf(),
			source,
		}
	})
}

/// Applies `--strict` or `--lenient` to the scoring curve.
#[must_use]
pub fn apply_tolerance(
	mut options: AnalysisOptions,
	strict: bool,
	lenient: bool,
) -> AnalysisOptions {
	if strict {
		options.scoring = ScoringConfig::strict();
		options.rules.max_cyclomatic_per_unit /= 2;
		options.rules.max_cognitive_per_unit /= 2;
		options.rules.max_nesting_depth = options.rules.max_nesting_depth.saturating_sub(1).max(1);
	}

	if lenient {
		options.scoring = ScoringConfig::lenient();
		options.rules.max_cyclomatic_per_unit *= 2;
		options.rules.max_cognitive_per_unit *= 2;
	}

	options
}

/// Errors raised while reading configuration.
#[derive(Debug)]
pub enum ConfigError {
	/// The file could not be read.
	Read {
		/// The path that failed.
		path: PathBuf,
		/// The underlying I/O error.
		source: std::io::Error,
	},
	/// The file could not be parsed.
	Parse {
		/// The path that failed.
		path: PathBuf,
		/// The underlying parse error.
		source: toml::de::Error,
	},
}

impl std::fmt::Display for ConfigError {
	fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Read { path, source } => {
				write!(
					formatter,
					"could not read config at {}: {source}",
					path.display()
				)
			}
			Self::Parse { path, source } => {
				write!(
					formatter,
					"could not parse config at {}: {source}",
					path.display()
				)
			}
		}
	}
}

impl std::error::Error for ConfigError {}
