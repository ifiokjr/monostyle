//! Per-path configuration sections.
//!
//! A repository is not one uniform standard. Tests are legitimately more nested than production code, a
//! vendored directory may be exempt entirely, and a generated client may deserve a lower bar than the
//! hand-written core. One global threshold forces a project to choose between a rule that is too lax
//! where it matters and one that fires everywhere it does not.
//!
//! A section states a path and what differs at that path:
//!
//! ```toml
//! # The repository-wide standard.
//! [rules]
//! max-nesting-depth = 3
//!
//! # Tests are structurally noisier, so the bar is different rather than the rule being off.
//! [[section]]
//! path = "crates/core/tests"
//! max-nesting-depth = 8
//! fail-under = { readability = 70, complexity = 80 }
//!
//! # Vendored code is not this project's to fix.
//! [[section]]
//! path = "vendor"
//! ignore = true
//! ```
//!
//! # Resolution
//!
//! A file matches the most specific section whose path is a prefix of it, so a section for a directory
//! applies to everything beneath it and a nested section wins over its parent. Sections are matched by path
//! component rather than by string prefix, so `src` matches `src/main.rs` but not `srcgen/main.rs`.

use std::path::Path;

use monostyle_rules::RulesConfig;
use serde::Deserialize;
use serde::Serialize;

/// A score floor for one or both categories.
///
/// Both fields are optional so a section can set a floor for the category it cares about. `complexity` can
/// never be set below [`FLOOR_MINIMUM`], because a floor that permits unmaintainable code is not a floor.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ScoreFloor {
	/// The minimum readability score.
	pub readability: Option<f64>,
	/// The minimum complexity score.
	pub complexity: Option<f64>,
}

impl ScoreFloor {
	/// The lowest score any floor may specify.
	///
	/// A project can be more permissive about layout than this tool's defaults, but a floor that allows
	/// unmaintainable code would defeat the purpose of having one.
	pub const FLOOR_MINIMUM: f64 = 50.0;

	/// Reports a floor below the permitted minimum.
	///
	/// Checked at load time rather than at scoring time so a typo is caught when the file is read, not
	/// silently accepted and then quietly ignored.
	#[must_use]
	pub fn validate(&self, path: &str) -> Option<String> {
		for (name, value) in [
			("readability", self.readability),
			("complexity", self.complexity),
		] {
			if let Some(value) = value
				&& value < Self::FLOOR_MINIMUM
			{
				return Some(format!(
					"the {name} floor for `{path}` is {value}, below the minimum of {}",
					Self::FLOOR_MINIMUM
				));
			}
		}

		None
	}

	/// Whether either floor is set.
	#[must_use]
	pub fn is_set(&self) -> bool {
		self.readability.is_some() || self.complexity.is_some()
	}
}

/// A section of a repository with its own thresholds.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Section {
	/// The directory this section applies to, relative to the configuration file.
	pub path: String,
	/// Rule thresholds that differ here, merged onto the repository-wide values.
	pub rules: RulesConfig,
	/// The score floors that apply here.
	pub fail_under: ScoreFloor,
	/// Whether to skip this path entirely.
	///
	/// Distinct from a floor of zero: an ignored path produces no findings and contributes nothing to any
	/// score, which is what a vendored directory or a generated fixture warrants.
	pub ignore: bool,
	/// A short explanation of why this section differs.
	///
	/// Required in spirit rather than by the type: a section without a reason is a threshold somebody will
	/// be afraid to change. The report prints it beside the section's score so the reason travels with the
	/// number.
	pub reason: Option<String>,
}

impl Section {
	/// Whether this section applies to `path`.
	///
	/// Matched by path component, so `src` does not match `srcgen`.
	#[must_use]
	pub fn matches(&self, path: &Path) -> bool {
		let section = Path::new(self.path.trim_end_matches('/'));

		if section.as_os_str().is_empty() {
			return false;
		}

		path.starts_with(section)
	}

	/// How specific this section is, used to pick a winner among several matches.
	///
	/// The number of path components is what makes a nested section win over its parent.
	#[must_use]
	pub fn specificity(&self) -> usize {
		Path::new(self.path.trim_end_matches('/'))
			.components()
			.count()
	}
}

/// Finds the section that applies to `path`.
///
/// The most specific match wins, so a section for a directory applies to everything beneath it unless a
/// nested section claims part of it.
#[must_use]
pub fn section_for<'config>(sections: &'config [Section], path: &Path) -> Option<&'config Section> {
	sections
		.iter()
		.filter(|section| section.matches(path))
		.max_by_key(|section| section.specificity())
}

/// Reports a configuration problem in a section.
///
/// Checked when the file is read so a mistake is reported at the point it was made, rather than producing
/// scores that look deliberate.
#[must_use]
pub fn validate_sections(sections: &[Section]) -> Option<String> {
	for section in sections {
		if section.path.trim().is_empty() {
			return Some("a section has an empty path, so it would match nothing".to_string());
		}

		if let Some(problem) = section.fail_under.validate(&section.path) {
			return Some(problem);
		}
	}

	None
}
