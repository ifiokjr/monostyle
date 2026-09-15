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
//! into the typed config. Adding a field to [`RulesConfig`] needs no change here at all.

use std::path::Path;
use std::path::PathBuf;

use crate::analysis::AnalysisOptions;

/// The on-disk configuration file, kept as raw values so a partial file can be merged.
pub type ConfigFile = toml::Value;

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

impl ConfigExt for AnalysisOptions {
	fn apply_config(self, file: &ConfigFile) -> Result<Self, ConfigError> {
		// The defaults are serialized so the overlay has something total to merge into. A failure here
		// would mean the config types cannot round-trip, which is a bug rather than a user error.
		let mut merged = toml::Value::try_from(&self.rules).map_err(ConfigError::Serialize)?;

		if let Some(rules) = file.get("rules") {
			merge(&mut merged, rules);
		}

		// `[scoring]` is a sibling of `[rules]` on the options rather than inside it, so it is applied
		// separately and keeps its own defaults.
		let mut options = Self {
			rules: merged.try_into().map_err(ConfigError::Deserialize)?,
			..self
		};

		if let Some(scoring) = file.get("scoring") {
			let mut serialized =
				toml::Value::try_from(options.scoring).map_err(ConfigError::Serialize)?;

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
		}
	}
}

impl std::error::Error for ConfigError {}
