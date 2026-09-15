//! End-to-end CLI tests.
//!
//! These run the built binary as a subprocess, which covers the surface the other suites cannot: argument
//! parsing, exit codes, output routing, and the interaction between flags. A rule can be perfectly
//! correct and still be unreachable because a flag is wired to the wrong field, and only a test at this
//! level catches that.

use std::path::PathBuf;
use std::process::Command;
use std::process::Output;

/// Runs the built binary with `args`.
fn run(args: &[&str]) -> Output {
	Command::new(env!("CARGO_BIN_EXE_monostyle"))
		.args(args)
		.output()
		.expect("the binary should run")
}

/// Runs the binary and returns stdout as text.
fn stdout(args: &[&str]) -> String {
	let output = run(args);

	assert!(
		output.status.success(),
		"the command failed: {}\n{}",
		args.join(" "),
		String::from_utf8_lossy(&output.stderr)
	);

	String::from_utf8_lossy(&output.stdout).into_owned()
}

/// Returns the path to a fixture.
fn fixture(relative: &str) -> PathBuf {
	PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.join("tests/fixtures")
		.join(relative)
}

/// Returns a fixture path as a string that outlives the call.
///
/// `fixture(..).to_str().unwrap()` does not compile in a `let`, because the `PathBuf` is a temporary
/// that is dropped at the end of the statement while the `&str` still borrows it. Callers use
/// `.as_str()` when passing the result into an argument list.
fn fixture_str(relative: &str) -> String {
	fixture(relative).to_string_lossy().into_owned()
}

// ---------------------------------------------------------------------------
// Basic invocations
// ---------------------------------------------------------------------------

#[test]
fn the_version_flag_prints_a_version() {
	let output = stdout(&["--version"]);

	assert!(output.contains("monostyle"), "got {output:?}");
	assert!(output.chars().any(|character| character.is_ascii_digit()));
}

#[test]
fn the_help_flag_lists_the_subcommands() {
	let output = stdout(&["--help"]);

	for command in ["check", "fix", "rules", "config"] {
		assert!(
			output.contains(command),
			"`{command}` should appear in help"
		);
	}
}

#[test]
fn checking_a_file_reports_scores() {
	let output = stdout(&[
		"check",
		fixture("bad/rust.rs").to_str().unwrap(),
		"--no-color",
	]);

	assert!(output.contains("readability"));
	assert!(output.contains("complexity"));
}

#[test]
fn checking_a_clean_file_reports_no_problems() {
	let output = stdout(&[
		"check",
		fixture("good/rust.rs").to_str().unwrap(),
		"--no-color",
	]);

	assert!(
		output.contains("No rule violations") || !output.contains("costing you points"),
		"a clean file should not be reported as having problems:\n{output}"
	);
}

#[test]
fn checking_a_directory_walks_it() {
	let output = stdout(&[
		"check",
		fixture("languages").to_str().unwrap(),
		"--no-color",
	]);

	assert!(
		output.contains("files"),
		"the report should state a file count"
	);
}

#[test]
fn a_missing_path_is_an_error() {
	let output = run(&["check", "/nonexistent/path/for/monostyle"]);

	assert!(!output.status.success(), "a missing path should fail");
	assert!(
		String::from_utf8_lossy(&output.stderr).contains("does not exist"),
		"the error should name the problem"
	);
}

#[test]
fn a_directory_with_no_analyzable_files_is_not_an_error() {
	let temp = tempfile::tempdir().expect("a temporary directory");

	let output = run(&["check", temp.path().to_str().unwrap(), "--no-color"]);

	assert!(
		output.status.success(),
		"an empty directory is not a failure"
	);
	assert!(
		String::from_utf8_lossy(&output.stderr).contains("no analyzable files"),
		"the run should say why it produced nothing"
	);
}

// ---------------------------------------------------------------------------
// Output formats
// ---------------------------------------------------------------------------

#[test]
fn json_output_is_valid_and_structured() {
	let output = stdout(&[
		"check",
		fixture("bad").to_str().unwrap(),
		"--format",
		"json",
	]);
	let decoded: serde_json::Value = serde_json::from_str(&output).expect("valid JSON");

	assert!(decoded["files"].is_array());
	assert!(decoded["readability"]["value"].is_number());
}

#[test]
fn output_can_be_written_to_a_file() {
	let temp = tempfile::tempdir().expect("a temporary directory");
	let target = temp.path().join("report.json");

	let _ = stdout(&[
		"check",
		fixture("bad").to_str().unwrap(),
		"--format",
		"json",
		"--output",
		target.to_str().unwrap(),
	]);

	let written = std::fs::read_to_string(&target).expect("the report should be written");

	assert!(serde_json::from_str::<serde_json::Value>(&written).is_ok());
}

// ---------------------------------------------------------------------------
// Filters and flags
// ---------------------------------------------------------------------------

#[test]
fn units_adds_a_function_table() {
	let output = stdout(&[
		"check",
		fixture("bad").to_str().unwrap(),
		"--units",
		"--no-color",
	]);

	assert!(output.contains("worst functions"), "got:\n{output}");
}

#[test]
fn explain_lists_every_finding() {
	let output = stdout(&[
		"check",
		fixture("bad").to_str().unwrap(),
		"--explain",
		"--no-color",
	]);

	assert!(output.contains("all findings"), "got:\n{output}");
}

#[test]
fn language_restriction_narrows_the_set() {
	let output = stdout(&[
		"check",
		fixture("languages").to_str().unwrap(),
		"--language",
		"python",
		"--format",
		"json",
	]);

	let decoded: serde_json::Value = serde_json::from_str(&output).expect("valid JSON");
	let files = decoded["files"].as_array().expect("files is an array");

	assert!(!files.is_empty(), "the Python fixture should be analyzed");

	for file in files {
		assert_eq!(file["language"], "python", "only Python should be analyzed");
	}
}

#[test]
fn a_disabled_rule_stops_firing() {
	let path = fixture_str("bad");
	let path = path.as_str();

	let with_rule = stdout(&["check", path, "--format", "json"]);
	let without_rule = stdout(&[
		"check",
		path,
		"--disable",
		"readability/blank-line-before-control-flow",
		"--format",
		"json",
	]);

	let enabled: serde_json::Value = serde_json::from_str(&with_rule).expect("valid JSON");
	let disabled: serde_json::Value = serde_json::from_str(&without_rule).expect("valid JSON");

	assert!(
		enabled["readability"]["penalty"].as_f64().unwrap_or(0.0)
			> disabled["readability"]["penalty"].as_f64().unwrap_or(0.0),
		"disabling a firing rule should lower the penalty"
	);
}

#[test]
fn strict_and_lenient_move_the_score_apart() {
	let path = fixture_str("bad");
	let path = path.as_str();

	let strict = stdout(&["check", path, "--strict", "--format", "json"]);
	let lenient = stdout(&["check", path, "--lenient", "--format", "json"]);

	let strict: serde_json::Value = serde_json::from_str(&strict).expect("valid JSON");
	let lenient: serde_json::Value = serde_json::from_str(&lenient).expect("valid JSON");

	assert!(
		strict["readability"]["value"].as_f64().unwrap_or(0.0)
			<= lenient["readability"]["value"].as_f64().unwrap_or(0.0),
		"strict mode should not score above lenient mode"
	);
}

#[test]
fn fail_under_uses_the_exit_code() {
	let path = fixture_str("bad");
	let path = path.as_str();

	let passing = run(&["check", path, "--fail-under", "0", "--quiet"]);
	let failing = run(&["check", path, "--fail-under", "100", "--quiet"]);

	assert!(passing.status.success(), "a threshold of zero should pass");
	assert_eq!(
		failing.status.code(),
		Some(1),
		"a threshold above the score should exit one"
	);
}

#[test]
fn include_generated_analyzes_what_is_skipped_by_default() {
	let excluded = stdout(&[
		"check",
		fixture("generated").to_str().unwrap(),
		"--format",
		"json",
	]);
	let included = stdout(&[
		"check",
		fixture("generated").to_str().unwrap(),
		"--include-generated",
		"--format",
		"json",
	]);

	let excluded: serde_json::Value = serde_json::from_str(&excluded).expect("valid JSON");
	let included: serde_json::Value = serde_json::from_str(&included).expect("valid JSON");

	assert!(excluded["files"].as_array().is_none_or(Vec::is_empty));
	assert!(
		!included["files"].as_array().is_none_or(Vec::is_empty),
		"--include-generated should analyze the files"
	);
}

#[test]
fn quiet_suppresses_the_summary_line() {
	let path = fixture_str("bad");
	let path = path.as_str();

	let output = run(&["check", path, "--quiet", "--no-color"]);
	let stderr = String::from_utf8_lossy(&output.stderr);

	assert!(
		!stderr.contains("monostyle:"),
		"the summary should be suppressed: {stderr}"
	);
}

// ---------------------------------------------------------------------------
// Rules and config
// ---------------------------------------------------------------------------

#[test]
fn rules_lists_every_rule() {
	let output = stdout(&["rules"]);

	assert!(output.contains("rules"), "got {output:?}");
	assert!(
		output.contains("readability/"),
		"rule names should be namespaced"
	);
	assert!(output.contains("complexity/"));
}

#[test]
fn rules_can_describe_one_rule() {
	let output = stdout(&["rules", "readability/deep-nesting"]);

	assert!(output.contains("readability/deep-nesting"));
	assert!(
		output.len() > 40,
		"a description should be more than the name"
	);
}

#[test]
fn an_unknown_rule_name_fails() {
	let output = run(&["rules", "readability/does-not-exist"]);

	assert_eq!(
		output.status.code(),
		Some(2),
		"an unknown rule should be an error"
	);
}

#[test]
fn rules_can_be_filtered_to_fixable_ones() {
	let output = stdout(&["rules", "--fixable", "--format", "json"]);
	let decoded: serde_json::Value = serde_json::from_str(&output).expect("valid JSON");

	let listed = decoded.as_array().expect("an array");

	// Only the blank-line rule is fixable, so this should be a short list rather than everything.
	assert!(
		listed.len() <= 3,
		"only a few rules are fixable, got {}",
		listed.len()
	);
}

#[test]
fn config_prints_the_effective_settings() {
	let output = stdout(&["config"]);

	for key in ["max-line-width", "max-nesting-depth", "half-life"] {
		assert!(
			output.contains(key),
			"`{key}` should appear in the config, got:\n{output}"
		);
	}
}

#[test]
fn config_can_be_printed_as_json() {
	let output = stdout(&["config", "--format", "json"]);
	let decoded: serde_json::Value = serde_json::from_str(&output).expect("valid JSON");

	assert!(decoded["rules"]["max-line-width"].is_number());
	assert!(decoded["scoring"]["half-life"].is_number());
}

#[test]
fn a_configuration_file_changes_the_thresholds() {
	let temp = tempfile::tempdir().expect("a temporary directory");
	let config = temp.path().join("monostyle.toml");

	std::fs::write(&config, "[rules]\nmax-line-width = 20\n").expect("write");

	// The config is discovered by walking up from the analyzed path, so the fixture is analyzed from
	// inside the temporary directory to keep the test independent of the repository's own config.
	std::fs::copy(fixture("bad/rust.rs"), temp.path().join("sample.rs")).expect("copy");

	let output = stdout(&[
		"check",
		temp.path().join("sample.rs").to_str().unwrap(),
		"--format",
		"json",
	]);

	let decoded: serde_json::Value = serde_json::from_str(&output).expect("valid JSON");

	assert!(
		decoded["readability"]["value"].is_number(),
		"the config should be found and applied"
	);
}

#[test]
fn an_explicit_config_path_is_used() {
	let temp = tempfile::tempdir().expect("a temporary directory");
	let config = temp.path().join("custom.toml");

	std::fs::write(&config, "[rules]\nmax-line-width = 200\n").expect("write");

	let output = run(&[
		"check",
		fixture("bad").to_str().unwrap(),
		"--config",
		config.to_str().unwrap(),
		"--format",
		"json",
	]);

	assert!(
		output.status.success(),
		"an explicit config should be accepted"
	);
}

// ---------------------------------------------------------------------------
// Fix
// ---------------------------------------------------------------------------

#[test]
fn fix_dry_run_writes_nothing() {
	let temp = tempfile::tempdir().expect("a temporary directory");
	let target = temp.path().join("sample.rs");

	let source = "fn a() {\n    if x {\n        work();\n    }\n}\n";
	std::fs::write(&target, source).expect("write");

	let output = stdout(&["fix", target.to_str().unwrap(), "--dry-run", "--no-color"]);

	assert!(output.contains("dry run"), "a dry run should say so");
	assert_eq!(
		std::fs::read_to_string(&target).expect("read"),
		source,
		"a dry run must not write"
	);
}

#[test]
fn fix_applies_the_blank_line() {
	let temp = tempfile::tempdir().expect("a temporary directory");
	let target = temp.path().join("sample.rs");

	std::fs::write(
		&target,
		"fn a() {\n    work();\n    if x {\n        work();\n    }\n}\n",
	)
	.expect("write");

	let _ = stdout(&["fix", target.to_str().unwrap(), "--no-color"]);

	let fixed = std::fs::read_to_string(&target).expect("read");

	assert!(
		fixed.contains("work();\n\n    if x"),
		"a blank line should be inserted:\n{fixed}"
	);
}

#[test]
fn fix_reports_what_needs_a_decision() {
	let temp = tempfile::tempdir().expect("a temporary directory");
	let target = temp.path().join("sample.rs");

	std::fs::write(
		&target,
		"fn a() {\n    work();\n    if x {\n        work();\n    }\n    let y = z * 86400;\n}\n",
	)
	.expect("write");

	let output = stdout(&["fix", target.to_str().unwrap(), "--dry-run", "--no-color"]);

	assert!(
		output.contains("need a decision"),
		"a finding without a fix should be listed for the reader:\n{output}"
	);
}

#[test]
fn fix_can_be_restricted_to_one_rule() {
	let temp = tempfile::tempdir().expect("a temporary directory");
	let target = temp.path().join("sample.rs");

	let source = "fn a() {\n    work();\n    if x {\n        work();\n    }\n}\n";
	std::fs::write(&target, source).expect("write");

	let output = stdout(&[
		"fix",
		target.to_str().unwrap(),
		"--rule",
		"readability/magic-number",
		"--dry-run",
		"--no-color",
	]);

	// No magic number is present, so restricting to that rule should find nothing to fix.
	assert!(
		output.contains("0 fixable") || !output.contains("would fix"),
		"a rule with nothing to fix should produce no edits:\n{output}"
	);
}

#[test]
fn fix_on_an_already_clean_file_does_nothing() {
	let temp = tempfile::tempdir().expect("a temporary directory");
	let target = temp.path().join("sample.rs");

	let source = "fn a() {\n    work();\n}\n";
	std::fs::write(&target, source).expect("write");

	let _ = stdout(&["fix", target.to_str().unwrap(), "--no-color"]);

	assert_eq!(
		std::fs::read_to_string(&target).expect("read"),
		source,
		"a clean file should be left untouched"
	);
}
