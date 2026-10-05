//! The auto-fix fixture corpus.
//!
//! Every directory under `fixtures/fix` holds a pair: `<name>.input.<ext>` and
//! `<name>.expected.<ext>`. The harness runs the real fix pipeline over the input and requires
//! the result to match the expected snapshot byte for byte, then runs the fixer again and
//! requires the second pass to be a no-op.
//!
//! The corpus exists to pin the fixer's behavior against the weird corners of every supported
//! language: interpolation with nested braces, raw strings, heredocs, macros, and the traps that
//! once led the fixer into editing the bytes of string literals. A fixture whose expected file
//! is missing fails the suite, so a half-added fixture cannot merge silently.

use std::path::Path;
use std::path::PathBuf;

use monostyle::analysis::AnalysisOptions;
use monostyle::fix::fix_file;
use monostyle_core::Fix;
use monostyle_core::Language;

/// The directory holding the corpus.
const CORPUS: &str = "tests/fixtures/fix";

/// The lowest number of fixtures the corpus must hold.
///
/// A floor rather than an exact count so adding fixtures never breaks the suite, while deleting
/// them in bulk does.
const MIN_FIXTURES: usize = 220;

/// One discovered fixture: its input path and the language implied by its extension.
struct Fixture {
	input: PathBuf,
	expected: PathBuf,
	language: Option<Language>,
	name: String,
}

/// Collects every `<name>.input.<ext>` under the corpus, recursively.
fn discover(root: &Path) -> Vec<Fixture> {
	let mut fixtures = Vec::new();
	visit(root, &mut fixtures);
	fixtures.sort_by(|left, right| left.name.cmp(&right.name));

	fixtures
}

fn visit(directory: &Path, fixtures: &mut Vec<Fixture>) {
	let Ok(entries) = std::fs::read_dir(directory) else {
		return;
	};

	for entry in entries.flatten() {
		let path = entry.path();

		if path.is_dir() {
			visit(&path, fixtures);
			continue;
		}

		let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
			continue;
		};

		let Some((stem, extension)) = file_name.split_once('.') else {
			continue;
		};

		if !extension.starts_with("input.") {
			continue;
		}

		let real_extension = extension.trim_start_matches("input.");
		let expected_name = format!("{stem}.expected.{real_extension}");
		let expected = path.with_file_name(expected_name);

		fixtures.push(Fixture {
			expected,
			language: Language::from_extension(real_extension),
			name: path
				.strip_prefix(CORPUS)
				.unwrap_or(&path)
				.display()
				.to_string(),
			input: path,
		});
	}
}

/// Applies every fixable finding to `source`, the same way the CLI's `fix` command does.
fn fixed_text(
	path: &Path,
	source: &str,
	language: Language,
) -> (String, monostyle::fix::AppliedFixes) {
	let report = monostyle::analysis::analyze_with_language(
		path,
		source,
		language,
		&AnalysisOptions::default(),
	);
	let fixes: Vec<Fix> = report
		.findings
		.iter()
		.filter(|finding| finding.fix.is_some())
		.filter_map(|finding| finding.fix.clone())
		.collect();

	let outcome = fix_file(path, &fixes, false).expect("the fixture should be fixable");
	let rewritten = std::fs::read_to_string(path).expect("the fixture should read back");
	(rewritten, outcome)
}

#[test]
fn the_corpus_meets_its_size_floor() {
	let fixtures = discover(Path::new(CORPUS));

	assert!(
		fixtures.len() >= MIN_FIXTURES,
		"the corpus holds {} fixtures but the floor is {MIN_FIXTURES}",
		fixtures.len()
	);
}

#[test]
fn every_input_has_an_expected_snapshot() {
	let fixtures = discover(Path::new(CORPUS));

	assert!(!fixtures.is_empty(), "the corpus should not be empty");

	let orphans: Vec<&String> = fixtures
		.iter()
		.filter(|fixture| !fixture.expected.exists())
		.map(|fixture| &fixture.name)
		.collect();

	assert!(
		orphans.is_empty(),
		"fixtures without an expected snapshot: {orphans:?}"
	);
}

#[test]
fn every_language_is_recognized() {
	// A fixture with an unknown extension would run through the fixer with no guards, because
	// the language drives the protected ranges.
	let fixtures = discover(Path::new(CORPUS));
	let unknown: Vec<&String> = fixtures
		.iter()
		.filter(|fixture| fixture.language.is_none())
		.map(|fixture| &fixture.name)
		.collect();

	assert!(
		unknown.is_empty(),
		"fixtures in unknown languages: {unknown:?}"
	);
}

#[test]
fn fixtures_fix_to_their_snapshots_and_are_idempotent() {
	let failures: Vec<String> = discover(Path::new(CORPUS))
		.into_iter()
		.filter_map(|fixture| {
			let language = fixture.language.expect("languages are checked separately");
			let input = std::fs::read_to_string(&fixture.input).expect("input should read");
			let expected =
				std::fs::read_to_string(&fixture.expected).expect("expected should read");

			if !lex(&input, language) {
				return Some(format!("{}: input does not lex cleanly", fixture.name));
			}

			let work = temp_copy(&fixture.name, &fixture.input, language);
			let (once, outcome) = fixed_text(&work, &input, language);

			let mut problem = None;

			if outcome.reverted {
				problem = Some(format!(
					"{}: rewrite reverted by the structural check",
					fixture.name
				));
			} else if once != expected {
				problem = Some(format!(
					"{}: fixed text differs from the snapshot\n--- expected ---\n{}\n--- actual ---\n{}",
					fixture.name, expected, once
				));
			}

			let (twice, _) = fixed_text(&work, &once, language);

			if twice != once {
				problem = Some(format!(
					"{}: the second pass is not a no-op\n--- after first ---\n{}\n--- after second ---\n{}",
					fixture.name, once, twice
				));
			}

			if !lex(&once, language) {
				problem = Some(format!("{}: fixed text does not lex cleanly", fixture.name));
			}

			problem
		})
		.collect();

	assert!(
		failures.is_empty(),
		"{} fixture(s) failed:\n{}",
		failures.len(),
		failures.join("\n\n")
	);
}

/// Lexes `source` and reports whether the scan completed without recovery.
fn lex(source: &str, language: Language) -> bool {
	monostyle_lexer::lex(source, language).is_clean()
}

/// Copies the input to a temporary file with the right extension so `fix_file` can detect the
/// language.
fn temp_copy(name: &str, input: &Path, language: Language) -> PathBuf {
	let extension = extension_of(language);
	let safe_name: String = name
		.chars()
		.map(|character| {
			if character.is_alphanumeric() {
				character
			} else {
				'-'
			}
		})
		.collect();
	let mut work = std::env::temp_dir();
	work.push(format!("monostyle-fixture-{safe_name}.{extension}"));
	std::fs::copy(input, &work).expect("the fixture should copy");

	work
}

fn extension_of(language: Language) -> String {
	match language {
		Language::Rust => "rs",
		Language::Dart => "dart",
		Language::TypeScript => "ts",
		Language::JavaScript | Language::Mozjs => "js",
		Language::Python => "py",
		Language::Go => "go",
		Language::C => "c",
		Language::Cpp => "cpp",
		Language::CSharp => "cs",
		Language::Kotlin => "kt",
		Language::Swift => "swift",
		Language::Ruby => "rb",
		Language::Php => "php",
		Language::Scala => "scala",
		Language::Nix => "nix",
		Language::Shell => "sh",
		Language::Lua => "lua",
		Language::Elixir => "ex",
		Language::Haskell => "hs",
		Language::Java => "java",
		Language::Tsx => "tsx",
		Language::Markdown => "md",
	}
	.to_string()
}
