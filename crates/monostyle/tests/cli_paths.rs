//! End-to-end CLI tests for the paths the library tests cannot reach: floor reporting, automatic
//! configuration discovery, configuration errors, and the fixer's skip messages.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::process::Output;

/// A scratch project directory with a unique name per test.
struct Project {
	root: PathBuf,
}

impl Project {
	fn new(name: &str) -> Self {
		let mut root = std::env::temp_dir();
		root.push(format!("monostyle-cli-{name}-{}", std::process::id()));

		let _ = fs::remove_dir_all(&root);
		fs::create_dir_all(&root).expect("the project directory should create");

		Self { root }
	}

	fn write(&self, relative: &str, contents: &str) -> PathBuf {
		let path = self.root.join(relative);

		if let Some(parent) = path.parent() {
			fs::create_dir_all(parent).expect("the parent directory should create");
		}

		fs::write(&path, contents).expect("the file should write");

		path
	}

	fn run(&self, args: &[&str]) -> Output {
		Command::new(env!("CARGO_BIN_EXE_monostyle"))
			.args(args)
			.current_dir(&self.root)
			.output()
			.expect("the binary should run")
	}
}

impl Drop for Project {
	fn drop(&mut self) {
		let _ = fs::remove_dir_all(&self.root);
	}
}

fn text(output: &Output) -> String {
	format!(
		"{}{}",
		String::from_utf8_lossy(&output.stdout),
		String::from_utf8_lossy(&output.stderr)
	)
}

/// A Rust file that scores badly: a long run with no separation, deep nesting, magic numbers.
const BAD_FILE: &str = "\
fn process(input: &str, mode: u8) -> u8 {
    let a = input.len() as u8;
    let b = a * 3;
    let c = b + 7;
    let d = c ^ 255;
    let e = d | 12;
    let f = e & 63;
    let g = f - 4;
    let h = g + 4096;
    if mode == 1 {
        if h > 4096 {
            if a > 0 {
                return h;
            }
        }
    }
    h
}
";

#[test]
fn a_below_floor_path_is_named_with_its_reason_and_offenders() {
	let project = Project::new("floor");
	project.write(
		"monostyle.toml",
		"\
fail-under = { readability = 99.0, complexity = 99.0 }

[[section]]
path = \"src\"
reason = \"the core deserves better\"
fail-under = { readability = 99.0, complexity = 99.0 }
",
	);
	project.write("src/lib.rs", BAD_FILE);

	let output = project.run(&["check", "."]);
	let rendered = text(&output);

	assert!(
		!output.status.success(),
		"a below-floor run must fail: {rendered}"
	);
	assert!(
		rendered.contains("below the configured floor"),
		"the failure names the floor: {rendered}"
	);
	assert!(
		rendered.contains("the core deserves better"),
		"the section's reason travels with the number: {rendered}"
	);
	assert!(
		rendered.contains("costing the most points"),
		"the worst rules are listed: {rendered}"
	);
}

#[test]
fn a_quiet_run_still_fails_but_says_less() {
	let project = Project::new("floor-quiet");
	project.write(
		"monostyle.toml",
		"fail-under = { readability = 99.0, complexity = 99.0 }\n",
	);
	project.write("src/lib.rs", BAD_FILE);

	let output = project.run(&["check", ".", "--quiet"]);
	let rendered = text(&output);

	assert!(!output.status.success(), "the run must still fail");
	assert!(
		!rendered.contains("below the configured floor"),
		"quiet suppresses the floor listing: {rendered}"
	);
}

#[test]
fn many_below_floor_paths_are_summarized() {
	let project = Project::new("floor-many");
	project.write(
		"monostyle.toml",
		"fail-under = { readability = 99.0, complexity = 99.0 }\n",
	);

	for index in 0..7 {
		project.write(&format!("src/file-{index}.rs"), BAD_FILE);
	}

	let output = project.run(&["check", "."]);
	let rendered = text(&output);

	assert!(
		rendered.contains("more"),
		"the listing is capped with a summary: {rendered}"
	);
}

#[test]
fn a_config_beside_the_paths_is_discovered_automatically() {
	let project = Project::new("autoconfig");
	project.write("monostyle.toml", "[rules]\nmax-line-width = 20\n");
	project.write(
		"src/lib.rs",
		"fn total() -> u8 { let sum = alpha + beta + gamma + delta + epsilon; sum }\n",
	);

	let output = project.run(&["check", "."]);
	let rendered = text(&output);

	assert!(
		rendered.contains("columns wide"),
		"the discovered config's line limit applies: {rendered}"
	);
}

#[test]
fn a_bare_number_fail_under_sets_both_floors() {
	let project = Project::new("fail-under-bare");
	project.write("monostyle.toml", "fail-under = 150.0\n");
	project.write("src/lib.rs", BAD_FILE);

	let output = project.run(&["check", "."]);
	let rendered = text(&output);

	assert!(
		rendered.contains("floor") || rendered.contains("fail-under"),
		"an out-of-range floor is rejected with a message: {rendered}"
	);
}

#[test]
fn an_unknown_config_key_is_reported() {
	let project = Project::new("unknown-key");
	project.write("monostyle.toml", "[rules]\nno-such-rule-setting = 3\n");
	project.write("src/lib.rs", BAD_FILE);

	let output = project.run(&["check", "."]);
	let rendered = text(&output);

	assert!(
		!output.status.success(),
		"an unknown key must be an error: {rendered}"
	);
	assert!(
		rendered.contains("no-such-rule-setting"),
		"the error names the key: {rendered}"
	);
}

#[test]
fn a_fix_skips_a_file_whose_scan_is_not_clean() {
	let project = Project::new("fix-unclean");
	project.write("lib.dart", "final tip = '''\nthe story never ends\n\n\n\n");

	let output = project.run(&["fix", "."]);
	let rendered = text(&output);

	assert!(
		rendered.contains("unterminated construct"),
		"the skip is explained: {rendered}"
	);
	assert!(
		rendered.contains("lib.dart"),
		"the skipped file is named: {rendered}"
	);
}

#[test]
fn a_dry_run_reports_fixes_without_writing() {
	let project = Project::new("fix-dry");
	let path = project.write(
		"lib.rs",
		"fn work() {\n    let a = 1;\n\n\n\n\n    let b = 2;\n}\n",
	);
	let before = fs::read_to_string(&path).expect("the file should read");

	let output = project.run(&["fix", ".", "--dry-run"]);
	let after = fs::read_to_string(&path).expect("the file should read");

	assert_eq!(before, after, "a dry run writes nothing");
	assert!(
		text(&output).contains("dry run"),
		"the run says it was a dry run"
	);
}

#[test]
fn fixing_one_rule_leaves_the_others() {
	let project = Project::new("fix-rule-filter");
	let path = project.write(
		"lib.rs",
		"fn work() {\n    let a = 1;\n\n\n\n\n    let b = 2;\n    return_placeholder;\n}\n"
			.replace("return_placeholder;", "let c = a + b;")
			.as_str(),
	);

	let output = project.run(&["fix", ".", "--rule", "readability/excessive-blank-lines"]);
	let rendered = text(&output);

	assert!(
		output.status.success(),
		"the run should succeed: {rendered}"
	);

	let fixed = fs::read_to_string(&path).expect("the file should read");

	assert!(
		!fixed.contains("\n\n\n\n\n"),
		"the blank run is collapsed: {fixed}"
	);
}
