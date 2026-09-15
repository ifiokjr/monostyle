//! Tests for configuration loading and merging.
//!
//! A configuration file that is silently ignored is worse than one that fails to parse, because the
//! scores it produces describe a rule set nobody chose. These tests cover the merge semantics that
//! make a partial file behave the way a reader expects.

use std::io::Write;

use monostyle::analysis::AnalysisOptions;
use monostyle::config::apply as apply_config;
use monostyle::config::apply_tolerance;
use monostyle::config::load_config;
use monostyle_rules::RulesConfig;

/// Loads a configuration file and applies it to the default options.
///
/// The merge is fallible because a file can name a value of the wrong type; every test here uses a
/// well-typed file, so a failure is a bug in the test rather than a case under test.
fn options(contents: &str) -> AnalysisOptions {
	apply_config(&AnalysisOptions::default(), &load(contents)).expect("the merge should succeed")
}

/// Writes a configuration file and loads it.
fn load(contents: &str) -> monostyle::config::ConfigFile {
	let mut file = tempfile::NamedTempFile::new().expect("a temporary file");

	file.write_all(contents.as_bytes()).expect("write");
	file.flush().expect("flush");

	load_config(file.path()).expect("the config should parse")
}

#[test]
fn an_empty_file_changes_nothing() {
	let options = options("");
	let defaults = AnalysisOptions::default();

	assert_eq!(options.rules, defaults.rules);
	assert_eq!(options.scoring.half_life, defaults.scoring.half_life);
}

#[test]
fn a_single_rule_value_overrides_only_that_rule() {
	// This is the property the overlay type exists for. Replacing the whole rule set would silently
	// reset every threshold the file did not mention, which is the bug that makes a config file
	// untrustworthy.
	let options = options("[rules]\nmax-nesting-depth = 7\n");
	let defaults = RulesConfig::default();

	assert_eq!(options.rules.max_nesting_depth, 7);
	assert_eq!(
		options.rules.max_cyclomatic_per_unit,
		defaults.max_cyclomatic_per_unit
	);
	assert_eq!(
		options.rules.max_cognitive_per_unit,
		defaults.max_cognitive_per_unit
	);
	assert_eq!(options.rules.max_line_width, defaults.max_line_width);
}

#[test]
fn several_rule_values_are_all_applied() {
	let options = options(
		"[rules]\nmax-nesting-depth = 5\nmax-line-width = 80\nmax-cyclomatic-per-unit = 20\n",
	);

	assert_eq!(options.rules.max_nesting_depth, 5);
	assert_eq!(options.rules.max_line_width, 80);
	assert_eq!(options.rules.max_cyclomatic_per_unit, 20);
}

#[test]
fn booleans_can_be_turned_off() {
	let options = options(
		"[rules]\nrequire-blank-line-before-control-flow = false\nreport-magic-numbers = false\n",
	);

	assert!(!options.rules.require_blank_line_before_control_flow);
	assert!(!options.rules.report_magic_numbers);
}

#[test]
fn a_disabled_rule_list_replaces_the_default() {
	let options = options(
		"[rules]\ndisabled-rules = [\"readability/overlong-line\", \"readability/magic-number\"]\n",
	);

	assert!(options.rules.is_enabled("readability/deep-nesting"));
	assert!(!options.rules.is_enabled("readability/overlong-line"));
	assert!(!options.rules.is_enabled("readability/magic-number"));
}

#[test]
fn the_scoring_curve_is_configurable() {
	let options = options("[scoring]\nhalf-life = 25.0\nmin-normalization-lines = 50.0\n");

	assert_eq!(options.scoring.half_life, 25.0);
	assert_eq!(options.scoring.min_normalization_lines, 50.0);
}

#[test]
fn ignore_patterns_are_read_from_configuration() {
	let options = options(
		"[rules.ignore]\npatterns = [\"**/*.spec.ts\"]\ngenerated = false\ninclude = [\"keep.g.dart\"]\n",
	);

	assert_eq!(options.rules.ignore.patterns, vec!["**/*.spec.ts"]);
	assert_eq!(options.rules.ignore.generated, Some(false));
	assert_eq!(options.rules.ignore.include, vec!["keep.g.dart"]);
}

#[test]
fn generated_exclusion_defaults_to_on() {
	let options = options("[rules]\nmax-nesting-depth = 2\n");

	assert!(options.rules.ignore.skip_generated());
	assert!(options.rules.ignore.skip_defaults());
}

#[test]
fn a_malformed_file_is_an_error_rather_than_a_silent_fallback() {
	// Quietly ignoring a threshold a project asked for would produce scores nobody can explain.
	let mut file = tempfile::NamedTempFile::new().expect("a temporary file");
	file.write_all(b"this is not [ valid toml").expect("write");
	file.flush().expect("flush");

	let result = load_config(file.path());

	assert!(result.is_err(), "a malformed config should fail loudly");
}

#[test]
fn a_missing_file_is_an_error() {
	let result = load_config(std::path::Path::new("/nonexistent/monostyle.toml"));

	assert!(result.is_err());
}

#[test]
fn strict_lowers_the_tolerance() {
	let strict = apply_tolerance(AnalysisOptions::default(), true, false);
	let defaults = AnalysisOptions::default();

	assert!(strict.scoring.half_life < defaults.scoring.half_life);
	assert!(strict.rules.max_cyclomatic_per_unit < defaults.rules.max_cyclomatic_per_unit);
	assert!(strict.rules.max_cognitive_per_unit < defaults.rules.max_cognitive_per_unit);
	assert!(strict.rules.max_nesting_depth <= defaults.rules.max_nesting_depth);
}

#[test]
fn lenient_raises_the_tolerance() {
	let lenient = apply_tolerance(AnalysisOptions::default(), false, true);
	let defaults = AnalysisOptions::default();

	assert!(lenient.scoring.half_life > defaults.scoring.half_life);
	assert!(lenient.rules.max_cyclomatic_per_unit > defaults.rules.max_cyclomatic_per_unit);
	assert!(lenient.rules.max_cognitive_per_unit > defaults.rules.max_cognitive_per_unit);
}

#[test]
fn strict_nesting_never_falls_below_one() {
	// A zero nesting limit would report every block, including a function body, which is not a
	// meaningful review and would make the tool unusable in strict mode.
	let mut options = AnalysisOptions::default();
	options.rules.max_nesting_depth = 1;

	let strict = apply_tolerance(options, true, false);

	assert!(strict.rules.max_nesting_depth >= 1);
}

#[test]
fn neither_tolerance_flag_leaves_the_defaults_alone() {
	let untouched = apply_tolerance(AnalysisOptions::default(), false, false);
	let defaults = AnalysisOptions::default();

	assert_eq!(untouched.rules, defaults.rules);
	assert_eq!(untouched.scoring, defaults.scoring);
}

#[test]
fn the_default_options_enable_the_cache() {
	// The cache was silently disabled once because a derived `Default` made every boolean false. This
	// pins the intent so it cannot regress the same way.
	assert!(AnalysisOptions::default().cache);
}

#[test]
fn the_severe_line_width_tracks_the_configured_limit() {
	let options = options("[rules]\nmax-line-width = 200\n");

	assert!(options.rules.severe_line_width() > 200);
}
