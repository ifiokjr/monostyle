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
	assert!(!options.rules.ignore.generated);
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

// ---------------------------------------------------------------------------
// Error paths and edge cases
// ---------------------------------------------------------------------------

#[test]
fn a_value_of_the_wrong_type_is_an_error() {
	// A string where a number belongs should fail loudly rather than being ignored, because the score
	// would then describe a threshold nobody chose.
	let result = apply_config(
		&AnalysisOptions::default(),
		&load("[rules]\nmax-nesting-depth = \"deep\"\n"),
	);

	assert!(result.is_err(), "a mistyped value should be reported");
}

#[test]
fn a_nested_scoring_value_is_merged() {
	let options = options("[scoring]\nhalf-life = 30.0\n");

	assert_eq!(options.scoring.half_life, 30.0);
}

#[test]
fn a_nested_ignore_value_does_not_discard_its_siblings() {
	// The merge is recursive, so setting one key under `[rules.ignore]` must leave the others alone.
	let options = options("[rules.ignore]\npatterns = [\"**/*.spec.ts\"]\n");

	assert_eq!(options.rules.ignore.patterns, vec!["**/*.spec.ts"]);
	assert!(
		options.rules.ignore.skip_generated(),
		"the other keys should keep their defaults"
	);
	assert!(options.rules.ignore.skip_defaults());
}

#[test]
fn several_nested_values_are_all_applied() {
	let options = options(
		"[rules.ignore]\npatterns = [\"a.ts\"]\ngenerated = false\ninclude = [\"b.g.dart\"]\n",
	);

	assert_eq!(options.rules.ignore.patterns, vec!["a.ts"]);
	assert!(!options.rules.ignore.generated);
	assert_eq!(options.rules.ignore.include, vec!["b.g.dart"]);
}

#[test]
fn a_file_with_no_rules_section_leaves_the_rules_alone() {
	let options = options("[scoring]\nhalf-life = 20.0\n");

	assert_eq!(options.rules, RulesConfig::default());
}

#[test]
fn a_file_with_no_scoring_section_leaves_the_curve_alone() {
	let options = options("[rules]\nmax-nesting-depth = 4\n");

	assert_eq!(options.scoring, AnalysisOptions::default().scoring);
}

#[test]
fn a_comment_only_file_changes_nothing() {
	let options = options("# just a note\n");

	assert_eq!(options.rules, RulesConfig::default());
}

#[test]
fn an_unknown_key_is_an_error() {
	// A typo in a threshold name would otherwise be silently ignored, which is how a project ends up
	// scoring against settings nobody chose.
	let result = apply_config(
		&AnalysisOptions::default(),
		&load("[rules]\nmax-nesting-depthh = 4\n"),
	);

	assert!(result.is_err(), "an unknown key should be reported");
}

#[test]
fn tolerance_flags_also_move_the_path_limit() {
	// Every threshold that describes complexity should respond to the tolerance flags, or strict mode
	// would be strict about some rules and not others.
	let strict = apply_tolerance(AnalysisOptions::default(), true, false);

	let lenient = apply_tolerance(AnalysisOptions::default(), false, true);

	let defaults = AnalysisOptions::default();

	assert!(strict.rules.max_npath_per_unit < defaults.rules.max_npath_per_unit);
	assert!(lenient.rules.max_npath_per_unit > defaults.rules.max_npath_per_unit);
}

// ---------------------------------------------------------------------------
// Unknown-key reporting
// ---------------------------------------------------------------------------

#[test]
fn an_unknown_nested_key_is_reported_with_its_full_path() {
	let result = apply_config(
		&AnalysisOptions::default(),
		&load("[rules.ignore]\ngeneratedd = false\n"),
	);

	let error = result.expect_err("a nested typo should be reported");

	assert!(
		error.to_string().contains("rules.ignore"),
		"the message should name the full path, got {error}"
	);
}

#[test]
fn an_unknown_scoring_key_is_reported() {
	let result = apply_config(
		&AnalysisOptions::default(),
		&load("[scoring]\nhalf-lif = 10\n"),
	);

	assert!(
		result.is_err(),
		"a typo in the scoring section should be reported"
	);
}

#[test]
fn the_error_message_names_the_command_that_lists_the_keys() {
	// A reader who made a typo needs the list of valid names, so the message points at it.
	let result = apply_config(
		&AnalysisOptions::default(),
		&load("[rules]\nmax-depth = 1\n"),
	);
	let error = result.expect_err("a typo should be reported");

	assert!(
		error.to_string().contains("monostyle config"),
		"the message should say how to list the keys, got {error}"
	);
}

#[test]
fn an_unknown_top_level_section_is_ignored() {
	// Only the sections the tool owns are validated; an editor's own table in the same file is not an
	// error, because a config file may be shared with other tooling.
	let options = options("[editor]\ntab-width = 4\n");

	assert_eq!(options.rules, RulesConfig::default());
}

#[test]
fn a_scalar_where_a_table_belongs_is_an_error() {
	let result = apply_config(&AnalysisOptions::default(), &load("rules = 1\n"));

	assert!(
		result.is_err(),
		"a scalar in place of the rules table should be reported"
	);
}

#[test]
fn an_array_where_a_scalar_belongs_is_an_error() {
	let result = apply_config(
		&AnalysisOptions::default(),
		&load("[rules]\nmax-nesting-depth = [1, 2]\n"),
	);

	assert!(
		result.is_err(),
		"an array in place of a number should be reported"
	);
}

#[test]
fn every_documented_key_is_accepted() {
	// The `config` command prints a starting point for a config file, so every key it prints must be
	// accepted when read back. This asserts the invariant with the real output rather than a
	// reconstruction of it.
	let printed = std::process::Command::new(env!("CARGO_BIN_EXE_monostyle"))
		.arg("config")
		.output()
		.expect("the binary should run");

	let text = String::from_utf8_lossy(&printed.stdout);
	let parsed: monostyle::config::ConfigFile =
		toml::from_str(&text).expect("the printed config should parse");

	let result = apply_config(&AnalysisOptions::default(), &parsed);

	assert!(
		result.is_ok(),
		"every printed key should be accepted: {:?}",
		result.err()
	);
}
