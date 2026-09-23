//! End-to-end tests for the rule behaviors shipped in 0.2.0.
//!
//! The rule-level suites in `monostyle_rules` pin each rule's logic against a source string, and those
//! tests cannot catch the failures that matter most in practice: a new rule missing from `monostyle
//! rules`, a name in `disabled-rules` that the registry does not recognize, a config key that parses but
//! is never consulted, or a fix that the `fix` command cannot reach. Those are wiring defects, and only a
//! test that runs the built binary catches them — which is what this file is for.
//!
//! Every test here drives the real binary over files on disk, the way a user runs it.

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

/// Parses the JSON report from a `check` run.
fn report(args: &[&str]) -> serde_json::Value {
	let output = stdout(args);

	serde_json::from_str(&output).expect("the report should be valid JSON")
}

/// The rule names the `rules` listing contains.
fn listed_rules(args: &[&str]) -> Vec<String> {
	let output = stdout(args);
	let decoded: serde_json::Value = serde_json::from_str(&output).expect("valid JSON");

	decoded
		.as_array()
		.expect("an array")
		.iter()
		.map(|rule| {
			rule.get("name")
				.and_then(|name| name.as_str())
				.expect("a name")
				.to_string()
		})
		.collect()
}

/// The rule names a report fired with.
fn fired(args: &[&str]) -> Vec<String> {
	let decoded = report(args);
	let files = decoded.get("files").and_then(|files| files.as_array());

	let findings = files
		.map(|files| {
			files
				.iter()
				.filter_map(|file| file.get("findings").and_then(|f| f.as_array()))
				.flatten()
				.collect::<Vec<_>>()
		})
		.unwrap_or_default();

	findings
		.iter()
		.map(|finding| {
			finding
				.get("rule")
				.and_then(|rule| rule.as_str())
				.expect("a rule name")
				.to_string()
		})
		.collect()
}

/// Writes a file into a fresh temporary directory and returns the directory.
///
/// Pointing `check` at the file rather than the directory keeps config discovery from walking up into
/// the repository's own `monostyle.toml`, which would make the test depend on thresholds this repo
/// chose for itself.
fn workspace(name: &str, contents: &str) -> (tempfile::TempDir, String) {
	let temp = tempfile::tempdir().expect("a temporary directory");
	let path = temp.path().join(name);

	std::fs::write(&path, contents).expect("the fixture should be written");

	let target = path.to_string_lossy().into_owned();

	(temp, target)
}

use std::process::Command;

// ---------------------------------------------------------------------------
// The registry is what `monostyle rules` shows and what `disabled-rules` matches
// ---------------------------------------------------------------------------
#[test]
fn the_rule_listing_names_the_markdown_rules() {
	// Four of these were documented as rules while being emitted from inside another one, so they were
	// invisible here and `disabled-rules` silently ignored them. Listing them is the visible half of
	// being a registry entry.
	let names = listed_rules(&["rules", "--format", "json"]);

	for expected in [
		"markdown/fence-without-language",
		"markdown/fence-language-unknown",
		"markdown/no-title",
		"markdown/skipped-heading-level",
		"readability/excessive-blank-lines",
	] {
		assert!(
			names.iter().any(|name| name == expected),
			"`{expected}` should be listed by `monostyle rules`, got {names:?}"
		);
	}
}

#[test]
fn the_rule_listing_no_longer_names_the_retired_composite() {
	// `markdown/heading-structure` was the aggregate the two heading rules lived inside. It is kept as a
	// configuration alias, and an alias is not a rule: listing it would let a user think disabling a
	// non-entry does something.
	let names = listed_rules(&["rules", "--format", "json"]);

	assert!(
		!names
			.iter()
			.any(|name| name == "markdown/heading-structure"),
		"the retired composite should not be listed as a rule, got {names:?}"
	);
}

#[test]
fn a_disabled_markdown_rule_produces_no_findings() {
	// The defect this release fixes, asserted through the config path a user actually writes: naming
	// `markdown/no-title` used to do nothing, because no registry entry owned the name.
	let (temp, target) = workspace(
		// The document both lacks a title and jumps two levels, so the rule under test and its sibling
		// each have something to report and disabling one leaves the other's finding in place.
		"doc.md",
		"## A document that starts at level two\n\n#### And skips to four\n\nText.\n",
	);

	let config = temp.path().join("monostyle.toml");
	std::fs::write(
		&config,
		"[rules]\ndisabled-rules = [\"markdown/no-title\"]\n",
	)
	.expect("write");

	let findings = fired(&[
		"check",
		&target,
		"--config",
		config.to_str().unwrap(),
		"--format",
		"json",
	]);

	assert!(
		!findings.contains(&"markdown/no-title".to_string()),
		"the disabled rule should be silent: {findings:?}"
	);
	assert!(
		findings.contains(&"markdown/skipped-heading-level".to_string()),
		"the other heading rule should still run: {findings:?}"
	);
}

#[test]
fn the_retired_name_still_disables_both_heading_rules() {
	// Configuration written against the old composite must keep the behavior it asked for. Losing it
	// would change a project's scores in a release whose notes never mention it.
	let (temp, target) = workspace(
		"doc.md",
		"## No title\n\n#### Skipped two levels\n\nText.\n",
	);

	let config = temp.path().join("monostyle.toml");
	std::fs::write(
		&config,
		"[rules]\ndisabled-rules = [\"markdown/heading-structure\"]\n",
	)
	.expect("write");

	let findings = fired(&[
		"check",
		&target,
		"--config",
		config.to_str().unwrap(),
		"--format",
		"json",
	]);

	assert!(
		!findings.contains(&"markdown/no-title".to_string()),
		"the alias should disable the title rule: {findings:?}"
	);
	assert!(
		!findings.contains(&"markdown/skipped-heading-level".to_string()),
		"the alias should disable the level rule: {findings:?}"
	);
}

#[test]
fn the_disable_flag_works_for_a_new_rule() {
	// The flag extends the same list the config file feeds, so a rule reachable one way is reachable the
	// other.
	let (_temp, target) = workspace(
		"doc.md",
		"## A document that starts below level one\n\nText.\n",
	);

	assert!(
		fired(&["check", &target, "--format", "json"]).contains(&"markdown/no-title".to_string()),
		"the rule should fire before it is disabled"
	);

	let findings = fired(&[
		"check",
		&target,
		"--disable",
		"markdown/no-title",
		"--format",
		"json",
	]);

	assert!(
		!findings.contains(&"markdown/no-title".to_string()),
		"the flag should silence the rule: {findings:?}"
	);
}

// ---------------------------------------------------------------------------
// group-separation through the real pipeline
// ---------------------------------------------------------------------------
fn crowded_function(count: usize) -> String {
	let mut source = String::from("fn work() {\n");

	push_statements(&mut source, count, "    ");

	source.push_str("}\n");

	source
}

/// Appends `count` numbered statements at `indent`.
///
/// Writing into the buffer rather than appending a formatted string is the shape the workspace's clippy
/// configuration asks for, and it names the fixture's shape more plainly than the loop it replaces.
fn push_statements(text: &mut String, count: usize, indent: &str) {
	use std::fmt::Write;

	for index in 1..=count {
		let _ = writeln!(text, "{indent}step_{index}();");
	}
}

#[test]
fn a_run_inside_a_function_body_is_reported() {
	// The blind spot: the rule used to see nothing inside any function, which is where every real run
	// lives. This is asserted at the CLI level because the failure it guards against is a rule that is
	// wired into the registry but never reaches a report.
	let (_temp, target) = workspace("work.rs", &crowded_function(10));

	let findings = fired(&["check", &target, "--format", "json"]);

	assert!(
		findings.contains(&"readability/group-separation".to_string()),
		"a ten-statement function body should be reported: {findings:?}"
	);
}

#[test]
fn the_finding_names_the_run_and_the_limit() {
	// A reader has to be able to split the run until the finding clears, which needs both numbers.
	let (_temp, target) = workspace("work.rs", &crowded_function(10));

	let decoded = report(&["check", &target, "--format", "json"]);
	let message = decoded
		.get("files")
		.and_then(|files| files.get(0))
		.and_then(|file| file.get("findings"))
		.and_then(|findings| findings.get(0))
		.and_then(|finding| finding.get("message"))
		.and_then(|message| message.as_str())
		.expect("the report should name the first finding");

	assert!(
		message.contains("10 statements") && message.contains("(limit 8)"),
		"the message should name the run and the limit: {message}"
	);
}

#[test]
fn a_run_at_the_limit_is_not_reported() {
	// The boundary the configurable limit defines: a group may hold exactly `max-statements-per-group`
	// statements, and the ninth is where a break belongs. The old rule flagged the eighth.
	let (_temp, target) = workspace("work.rs", &crowded_function(8));

	let findings = fired(&["check", &target, "--format", "json"]);

	assert!(
		!findings.contains(&"readability/group-separation".to_string()),
		"eight statements are within the limit: {findings:?}"
	);
}

#[test]
fn the_statement_limit_is_configurable_through_a_config_file() {
	// A config key that parses but is never consulted produces scores that describe a rule set nobody
	// chose. This is the check that the value a project writes is the value the rule measures against.
	let mut source = String::new();

	push_statements(&mut source, 12, "");

	let (temp, target) = workspace("work.rs", &source);

	// At the default limit of 8, twelve top-level statements are a finding.
	assert!(
		fired(&["check", &target, "--format", "json"])
			.contains(&"readability/group-separation".to_string()),
		"twelve statements should be reported at the default limit"
	);

	let config = temp.path().join("monostyle.toml");
	std::fs::write(&config, "[rules]\nmax-statements-per-group = 20\n").expect("write");

	let findings = fired(&[
		"check",
		&target,
		"--config",
		config.to_str().unwrap(),
		"--format",
		"json",
	]);

	assert!(
		!findings.contains(&"readability/group-separation".to_string()),
		"the raised limit should silence the run: {findings:?}"
	);
}

#[test]
fn a_dart_switch_expression_of_many_arms_is_not_reported() {
	// A total function over an enum lists one arm per variant, so a nine-arm switch is the shape every
	// such function has. Counting the arms as statements made the rule fire on it.
	let source = "\
SectionColour fromCode(int code) => switch (code) {
  0 => SectionColour.acid,
  1 => SectionColour.coral,
  2 => SectionColour.cyan,
  3 => SectionColour.paper,
  4 => SectionColour.violet,
  5 => SectionColour.amber,
  6 => SectionColour.blue,
  7 => SectionColour.pink,
  _ => throw StateError('unknown colour'),
};
";
	let (_temp, target) = workspace("colour.dart", source);

	let findings = fired(&["check", &target, "--format", "json"]);

	assert!(
		!findings.contains(&"readability/group-separation".to_string()),
		"switch arms are alternatives, not a statement run: {findings:?}"
	);
}

#[test]
fn a_struct_literal_is_not_counted_as_a_statement_run() {
	// A literal's fields are data. Counting them reported every large constructor in a repository as a
	// wall of statements, which is the false positive that made the rule noise downstream.
	let source = "\
fn build() -> Point {
    let point = Point {
        x: 1,
        y: 2,
        z: 3,
        w: 4,
        a: 5,
        b: 6,
        c: 7,
        d: 8,
        e: 9,
        f: 10,
    };

    point
}
";
	let (_temp, target) = workspace("point.rs", source);

	let findings = fired(&["check", &target, "--format", "json"]);

	assert!(
		!findings.contains(&"readability/group-separation".to_string()),
		"a literal's fields are data, not statements: {findings:?}"
	);
}

// ---------------------------------------------------------------------------
// The blank-line ceiling and its fix
// ---------------------------------------------------------------------------
#[test]
fn a_stacked_blank_run_is_reported_and_named() {
	// The ceiling exists because every other layout rule asks for a gap without bounding one. A gap
	// grown by a fixer that only inserts blank lines is exactly what this rule reports.
	let (_temp, target) = workspace(
		"split.rs",
		"fn split() {\n    let a = 1;\n\n\n\n\n    let b = 2;\n}\n",
	);

	let findings = fired(&["check", &target, "--format", "json"]);

	assert!(
		findings.contains(&"readability/excessive-blank-lines".to_string()),
		"five consecutive blank lines should be reported: {findings:?}"
	);
}

#[test]
fn two_blank_lines_in_python_are_not_reported() {
	// The allowance is per language: PEP 8 asks for two blank lines before a top-level definition, so
	// the limit of one would report the documented layout of a whole language.
	let (_temp, target) = workspace(
		"layout.py",
		"def first():\n    pass\n\n\ndef second():\n    pass\n",
	);

	let findings = fired(&["check", &target, "--format", "json"]);

	assert!(
		!findings.contains(&"readability/excessive-blank-lines".to_string()),
		"the Python convention should not be reported: {findings:?}"
	);
}

#[test]
fn fix_collapses_a_stacked_run_on_disk() {
	// The property that made the fix worth writing: the run is brought back to the allowance, and the
	// edit survives because it deletes rather than rewrites.
	let (_temp, target) = workspace(
		"split.rs",
		"fn split() {\n    let a = 1;\n\n\n\n\n    let b = 2;\n}\n",
	);

	let _ = stdout(&["fix", &target, "--no-color"]);

	let fixed = std::fs::read_to_string(&target).expect("the fixed file");

	assert_eq!(
		fixed, "fn split() {\n    let a = 1;\n\n    let b = 2;\n}\n",
		"exactly one blank line should remain"
	);

	// A second run finds nothing, which is what makes the fixer safe to run on every commit.
	let findings = fired(&["check", &target, "--format", "json"]);

	assert!(
		!findings.contains(&"readability/excessive-blank-lines".to_string()),
		"the fixed file should be clean: {findings:?}"
	);
}

#[test]
fn fix_targets_the_collapse_by_rule_name() {
	// `--rule` is how a project applies one fix in CI without taking the others. The collapse has to be
	// reachable that way, or the ceiling cannot be applied on its own.
	let (_temp, target) = workspace(
		"split.py",
		"def first():\n    pass\n\n\n\n\ndef second():\n    pass\n",
	);

	let _ = stdout(&[
		"fix",
		&target,
		"--rule",
		"readability/excessive-blank-lines",
		"--no-color",
	]);

	let fixed = std::fs::read_to_string(&target).expect("the fixed file");

	assert_eq!(
		fixed, "def first():\n    pass\n\n\ndef second():\n    pass\n",
		"the Python allowance of two blank lines should survive the fix"
	);
}

#[test]
fn only_the_formatter_safe_rules_are_listed_as_fixable() {
	// Every fixable rule is an edit a formatter leaves alone: inserting a blank line, deleting the ones
	// past the allowance, padding after a block, and re-attaching a comment. A new fixable rule needs
	// the same argument made for it, so the list is pinned.
	let mut names = listed_rules(&["rules", "--fixable", "--format", "json"]);
	names.sort();

	assert_eq!(
		names,
		vec![
			"readability/blank-line-after-control-flow",
			"readability/blank-line-before-control-flow",
			"readability/blank-line-before-return",
			"readability/detached-comment",
			"readability/excessive-blank-lines"
		],
		"only the formatter-safe rules should be fixable"
	);
}

// ---------------------------------------------------------------------------
// Markdown prose versus the code inside it
// ---------------------------------------------------------------------------
#[test]
fn prose_is_never_scored_but_fence_code_is() {
	// Both halves in one document: the numbered list is prose, so a code rule must not read it, while the
	// Rust example inside the fence is code people copy and must be scored. The fence finding is
	// attributed to the example, which is what tells a reader where to look.
	let source = "\
# Guide

1. First step of the process.
2. Second step of the process.
3. Third step of the process.
4. Fourth step of the process.
5. Fifth step of the process.
6. Sixth step of the process.

An example:

```rust
fn demo() {
    let a = 1;
    let b = 2;
    let c = 3;
    let d = 4;
    let e = 5;
    let f = 6;
    let g = 7;
    let h = 8;
    let i = 9;
}
```
";
	let (_temp, target) = workspace("guide.md", source);

	let findings = fired(&["check", &target, "--format", "json"]);

	assert!(
		!findings.contains(&"readability/magic-number".to_string()),
		"a list marker is not a numeric literal: {findings:?}"
	);
	assert!(
		findings.contains(&"readability/group-separation".to_string()),
		"the run inside the fence should be scored: {findings:?}"
	);
}

#[test]
fn fence_findings_are_attributed_to_the_example() {
	// A fence's code starts partway down the document, so a finding that named the fence's own line
	// number would send a reader to the wrong place.
	let (_temp, target) = workspace(
		"guide.md",
		"# Example\n\nSome prose.\n\n```rust\nfn demo() {\n    let a = 1;\n    if a > 0 {\n        work();\n    }\n}\n```\n",
	);

	let decoded = report(&["check", &target, "--format", "json"]);
	let message = decoded
		.get("files")
		.and_then(|files| files.get(0))
		.and_then(|file| file.get("findings"))
		.and_then(|findings| findings.get(0))
		.and_then(|finding| finding.get("message"))
		.and_then(|message| message.as_str())
		.expect("the report should name the first finding");

	assert!(
		message.starts_with("in the rust example:"),
		"the finding should name the example it came from: {message}"
	);
}

// ---------------------------------------------------------------------------
// A checked-in bundle is a dependency, not the project
// ---------------------------------------------------------------------------
#[test]
fn a_bundled_dependency_is_skipped_and_can_be_included() {
	// The two halves of the ignore decision: a bundle banner means the file is somebody else's code, and
	// `--include-generated` is the override a reviewer needs when they do want to look inside it.
	//
	// The path is a directory because ignore rules apply while collecting files; a file named on the
	// command line is analyzed whatever the ignore rules say, which is how a user insists.
	let temp = tempfile::tempdir().expect("a temporary directory");

	std::fs::write(
		temp.path().join("app.js"),
		"/* esm.sh - esbuild bundle(@wallet-standard/app@1.1.0) es2022 development */\n\
		 var wallets = void 0;\n\
		 var registered = new Set();\n",
	)
	.expect("the fixture should be written");

	let path = temp.path().to_str().expect("a path");

	// Skipped by default: nothing is analyzed, so the report has no files at all.
	let decoded = report(&["check", path, "--format", "json"]);

	assert_eq!(
		decoded["files"].as_array().map(Vec::len),
		Some(0),
		"a bundle should be skipped by default: {decoded}"
	);

	// The override brings it back, whether or not its contents trip a rule.
	let included = report(&["check", path, "--include-generated", "--format", "json"]);

	assert_eq!(
		included["files"].as_array().map(Vec::len),
		Some(1),
		"the override should analyze the bundle: {included}"
	);
}
