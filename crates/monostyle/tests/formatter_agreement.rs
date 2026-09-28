//! Formatter-agreement tests for the fixer.
//!
//! The fixture corpus proves the fixer is byte-safe; these tests prove the stronger property the
//! downstream rollout depended on: on input a formatter already accepts, the fixer's output must
//! still be accepted. A blank line the formatter removes is a disagreement about the same file,
//! and enough of them made real repositories' lint gates fail. The inputs are kept
//! formatter-clean by hand; if a tool is not installed the test reports a skip rather than
//! failing, so the suite still runs where the toolchain is thin.

use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use monostyle::analysis::AnalysisOptions;
use monostyle::analysis::analyze_paths;
use monostyle::fix;
use monostyle_core::Fix;

/// Returns the path to a formatter-clean fixture.
fn fixture(name: &str) -> PathBuf {
	PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.join("tests/fixtures/formatter_clean")
		.join(name)
}

/// Runs `monostyle fix` over a copy of `source` and returns the fixed bytes.
fn fixed_bytes(source: &Path) -> Vec<u8> {
	let original = std::fs::read(source).expect("the fixture");
	let temp = tempfile::tempdir().expect("a temporary directory");
	let target = temp.path().join(source.file_name().expect("a file name"));
	std::fs::write(&target, &original).expect("write the copy");

	let report = analyze_paths(
		std::slice::from_ref(&target),
		&AnalysisOptions {
			cache: false,
			..AnalysisOptions::default()
		},
	);

	for file in &report.files {
		let fixes: Vec<Fix> = file
			.findings
			.iter()
			.filter_map(|finding| finding.fix.clone())
			.collect();

		fix::fix_file(&file.path, &fixes, false).expect("apply the fixes");
	}

	std::fs::read(&target).expect("the fixed copy")
}

/// Whether a formatter binary is on PATH.
fn tool_available(name: &str) -> bool {
	Command::new(name)
		.arg("--version")
		.stdout(std::process::Stdio::null())
		.stderr(std::process::Stdio::null())
		.status()
		.is_ok_and(|status| status.success())
}

/// Asserts `tool` leaves `content` unchanged, printing its output on disagreement.
fn assert_unchanged(tool: &str, arguments: &[&str], working_dir: &Path, content: &[u8]) {
	let temp = tempfile::tempdir().expect("a temporary directory");
	let file = temp.path().join("probe");
	std::fs::write(&file, content).expect("write the probe");

	let status = Command::new(tool)
		.args(arguments)
		.arg(file.file_name().expect("a file name"))
		.current_dir(temp.path())
		.status()
		.expect("run the formatter");

	assert!(
		status.success(),
		"{tool} rejected the fixed output in {}; the fixer and {tool} disagree",
		working_dir.display()
	);
}

#[test]
fn the_fixer_never_disagrees_with_rustfmt() {
	if !tool_available("rustfmt") {
		eprintln!("skipping: rustfmt is not installed");

		return;
	}

	for entry in std::fs::read_dir(fixture(".")).expect("the fixture directory") {
		let path = entry.expect("an entry").path();

		if path.extension().is_none_or(|extension| extension != "rs") {
			continue;
		}

		let fixed = fixed_bytes(&path);

		assert_unchanged("rustfmt", &["--check"], &path, &fixed);
	}
}

#[test]
fn the_fixer_never_disagrees_with_dart_format() {
	if !tool_available("dart") {
		eprintln!("skipping: dart is not installed");

		return;
	}

	for entry in std::fs::read_dir(fixture(".")).expect("the fixture directory") {
		let path = entry.expect("an entry").path();

		if path.extension().is_none_or(|extension| extension != "dart") {
			continue;
		}

		let fixed = fixed_bytes(&path);

		assert_unchanged(
			"dart",
			&["format", "--output=none", "--set-exit-if-changed"],
			&path,
			&fixed,
		);
	}
}
