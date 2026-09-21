//! Configuration file loading.
//!
//! # Why merging happens at the value level
//!
//! A configuration file should only have to state what it changes. That requires distinguishing "this
//! field was absent" from "this field was set to its default value", which a struct of plain values
//! cannot express: deserialization fills absent fields with defaults, so merging a partial file would
//! silently reset every threshold the file did not mention.
//!
//! The first attempt solved this with a parallel struct of `Option` fields and a macro that copied the
//! present ones across. It worked, and it was a maintenance trap: every new rule threshold had to be
//! added in two places, and the four fields added in one session were all missed, which meant a project
//! setting `max-line-width` had it silently ignored.
//!
//! Merging the parsed TOML over the serialized defaults removes the second list entirely. The defaults
//! are serialized once, the file's values are overlaid onto them, and the result is deserialized back
//! into the typed config. Adding a field to `RulesConfig` needs no change here at all.

use std::path::Path;
use std::path::PathBuf;

use monostyle_rules::RulesConfig;

use crate::analysis::AnalysisOptions;

/// The on-disk configuration file, kept as raw values so a partial file can be merged.
pub type ConfigFile = toml::Value;

/// Reports a key that does not exist in the defaults.
///
/// A typo in a threshold name would otherwise be silently accepted, and the project would score against
/// settings nobody chose — the same failure the overlay type was written to avoid, arriving by a
/// different route. Unknown keys are therefore an error rather than a warning.
fn reject_unknown_keys(
	base: &toml::Value,
	overrides: &toml::Value,
	path: &str,
) -> Result<(), ConfigError> {
	let (Some(base_table), Some(override_table)) = (base.as_table(), overrides.as_table()) else {
		return Ok(());
	};

	for (key, value) in override_table {
		let Some(base_value) = base_table.get(key) else {
			return Err(ConfigError::UnknownKey {
				section: path.to_string(),
				key: key.clone(),
			});
		};

		// A nested table is checked recursively, so `[rules.ignore]` reports an unknown key under its
		// full path rather than only at the top level.
		if value.is_table() {
			reject_unknown_keys(base_value, value, &format!("{path}.{key}"))?;
		}
	}

	Ok(())
}

/// Merges `overrides` onto `base`, recursively for tables.
///
/// A nested table is merged key by key rather than replaced, so a file that sets one value under
/// `[rules.ignore]` does not discard the other keys in that table.
fn merge(base: &mut toml::Value, overrides: &toml::Value) {
	match (base, overrides) {
		(toml::Value::Table(base_table), toml::Value::Table(override_table)) => {
			for (key, value) in override_table {
				match base_table.get_mut(key) {
					Some(existing) => merge(existing, value),
					None => {
						base_table.insert(key.clone(), value.clone());
					}
				}
			}
		}
		(base_value, override_value) => {
			*base_value = override_value.clone();
		}
	}
}

/// Extension trait that folds a parsed configuration file into analysis options.
pub trait ConfigExt: Sized {
	/// Applies the file's settings, leaving every unmentioned value at its default.
	fn apply_config(self, file: &ConfigFile) -> Result<Self, ConfigError>;
}

/// Applies a loaded configuration file to a set of options.
///
/// A free function as well as a method, because `ConfigFile` is a `toml::Value` — an alias for a type
/// this crate does not own, so the method cannot be implemented on it.
///
/// # Errors
///
/// Returns [`ConfigError::Deserialize`] when a value in the file does not match the type of the field
/// it targets, such as a string given to a numeric threshold.
pub fn apply(options: &AnalysisOptions, file: &ConfigFile) -> Result<AnalysisOptions, ConfigError> {
	options.clone().apply_config(file)
}

/// Reads the sections a configuration file declares, resolving each one's rules.
///
/// Sections live at the top level as `[[section]]` entries rather than inside `[rules]`, because a section
/// holds a floor and an ignore flag as well as rule thresholds. Keeping them out of the rules namespace
/// means the merge for `[rules]` stays a simple value overlay.
///
/// Each section's rules are merged onto `base`, the already-resolved repository-wide values, so a section
/// states only what differs. Deserializing a section on its own cannot do this: absent fields would be
/// filled with defaults, and every repository-wide threshold a section did not restate would silently
/// revert for the paths it matched. That is the failure this merge exists to prevent, and it is invisible
/// in the config file because the file reads correctly.
fn read_sections(
	file: &toml::Value,
	base: &RulesConfig,
) -> Result<Vec<crate::section::Section>, ConfigError> {
	let Some(sections) = file.get("section") else {
		return Ok(Vec::new());
	};

	let mut parsed: Vec<crate::section::Section> = sections
		.clone()
		.try_into()
		.map_err(ConfigError::Deserialize)?;

	for (index, section) in parsed.iter_mut().enumerate() {
		let mut merged = toml::Value::try_from(base).map_err(ConfigError::Serialize)?;

		// A section without a `[section.rules]` table inherits the repository-wide rules unchanged, so the
		// overlay is skipped rather than treated as an empty one.
		if let Some(rules) = sections.get(index).and_then(|value| value.get("rules")) {
			merge(&mut merged, rules);
		}

		section.rules = merged.try_into().map_err(ConfigError::Deserialize)?;
	}

	if let Some(problem) = crate::section::validate_sections(&parsed) {
		return Err(ConfigError::InvalidSection(problem));
	}

	Ok(parsed)
}

/// Reads the repository-wide score floors.
fn read_floor(file: &toml::Value) -> Result<crate::section::ScoreFloor, ConfigError> {
	// `fail-under` may be a bare number, which sets both categories to the same floor.
	if let Some(value) = file.get("fail-under") {
		if let Some(number) = value
			.as_float()
			.or_else(|| value.as_integer().map(|i| i as f64))
		{
			let floor = crate::section::ScoreFloor {
				readability: Some(number),
				complexity: Some(number),
			};

			if let Some(problem) = floor.validate("fail-under") {
				return Err(ConfigError::InvalidSection(problem));
			}

			return Ok(floor);
		}

		let floor: crate::section::ScoreFloor =
			value.clone().try_into().map_err(ConfigError::Deserialize)?;

		if let Some(problem) = floor.validate("fail-under") {
			return Err(ConfigError::InvalidSection(problem));
		}

		return Ok(floor);
	}

	Ok(crate::section::ScoreFloor::default())
}

impl ConfigExt for AnalysisOptions {
	fn apply_config(self, file: &ConfigFile) -> Result<Self, ConfigError> {
		// The defaults are serialized so the overlay has something total to merge into. A failure here
		// would mean the config types cannot round-trip, which is a bug rather than a user error.
		let mut merged = toml::Value::try_from(&self.rules).map_err(ConfigError::Serialize)?;

		if let Some(rules) = file.get("rules") {
			reject_unknown_keys(&merged, rules, "rules")?;
			merge(&mut merged, rules);
		}

		// A section's rules are checked against the same defaults, so a typo inside a section is caught
		// the same way one at the top level is.
		for section in file
			.get("section")
			.and_then(|value| value.as_array())
			.into_iter()
			.flatten()
		{
			if let Some(rules) = section.get("rules") {
				let name = section
					.get("path")
					.and_then(|path| path.as_str())
					.unwrap_or("<unnamed>");
				let label = format!("{name}.rules");

				reject_unknown_keys(&merged, rules, &label)?;
			}
		}

		// `[scoring]` is a sibling of `[rules]` on the options rather than inside it, so it is applied
		// separately and keeps its own defaults.
		let resolved: RulesConfig = merged.try_into().map_err(ConfigError::Deserialize)?;

		let mut options = Self {
			// Sections are resolved against the repository-wide rules rather than the defaults, so a
			// section inherits every threshold it does not restate.
			sections: read_sections(file, &resolved)?,
			rules: resolved,
			fail_under: read_floor(file)?,
			..self
		};

		if let Some(scoring) = file.get("scoring") {
			let mut serialized =
				toml::Value::try_from(options.scoring).map_err(ConfigError::Serialize)?;

			reject_unknown_keys(&serialized, scoring, "scoring")?;
			merge(&mut serialized, scoring);

			options.scoring = serialized.try_into().map_err(ConfigError::Deserialize)?;
		}

		Ok(options)
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
/// A malformed configuration file is a hard error rather than a silent fallback: quietly ignoring a
/// threshold a project asked for would produce scores nobody can explain.
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
		options.scoring = monostyle_core::ScoringConfig::strict();
		options.rules.max_cyclomatic_per_unit /= 2;
		options.rules.max_cognitive_per_unit /= 2;
		options.rules.max_nesting_depth = options.rules.max_nesting_depth.saturating_sub(1).max(1);
		options.rules.max_npath_per_unit /= 2;
	}

	if lenient {
		options.scoring = monostyle_core::ScoringConfig::lenient();
		options.rules.max_cyclomatic_per_unit *= 2;
		options.rules.max_cognitive_per_unit *= 2;
		options.rules.max_npath_per_unit *= 2;
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
	/// The defaults could not be serialized for merging.
	Serialize(toml::ser::Error),
	/// A merged configuration could not be read back into the typed form.
	Deserialize(toml::de::Error),
	/// A section is malformed or states an impossible floor.
	InvalidSection(String),
	/// The file names a key that does not exist.
	UnknownKey {
		/// The section the key was found in, as a dotted path.
		section: String,
		/// The unrecognized key.
		key: String,
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
			Self::Serialize(source) => write!(formatter, "could not serialize defaults: {source}"),
			Self::Deserialize(source) => {
				write!(
					formatter,
					"could not read the merged configuration: {source}"
				)
			}
			Self::InvalidSection(problem) => write!(formatter, "{problem}"),
			Self::UnknownKey { section, key } => {
				write!(
					formatter,
					"unknown configuration key `{key}` in [{section}]; run `monostyle config` to list the \
				 available keys"
				)
			}
		}
	}
}

impl std::error::Error for ConfigError {}
