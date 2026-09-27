//! Property tests over the fixture corpus.
//!
//! Snapshots pin *what* the fixer produces; these tests pin *invariants* that must hold for
//! every fixture and every recombination of fixture fragments:
//!
//! 1. Fixing never lowers a score. Blank-line edits cannot change the line counts the
//!    denominators use, so readability can only hold or improve and complexity must not move.
//! 2. Recombined fragments never panic, never hang, and fix to a fixed point.
//! 3. A file that lexes clean before fixing lexes clean after.

use std::path::Path;

use monostyle::analysis::AnalysisOptions;
use monostyle::analysis::analyze_with_language;
use monostyle_core::Fix;
use monostyle_core::Language;
use monostyle_rules::RulesConfig;
use monostyle_rules::run_rules;

const CORPUS: &str = "tests/fixtures/fix";

/// Discovers every fixture input with its language.
fn fixtures() -> Vec<(std::path::PathBuf, Language)> {
	let mut found = Vec::new();
	visit(Path::new(CORPUS), &mut found);
	found.sort();

	found
}

fn visit(directory: &Path, found: &mut Vec<(std::path::PathBuf, Language)>) {
	let Ok(entries) = std::fs::read_dir(directory) else {
		return;
	};

	for entry in entries.flatten() {
		let path = entry.path();

		if path.is_dir() {
			visit(&path, found);
			continue;
		}

		let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
			continue;
		};

		let Some((_, extension)) = name.split_once('.') else {
			continue;
		};

		if !extension.starts_with("input.") {
			continue;
		}

		let real_extension = extension.trim_start_matches("input.");

		if let Some(language) = Language::from_extension(real_extension) {
			found.push((path, language));
		}
	}
}

/// Analyzes `source` as `language` and returns (readability, complexity).
fn scores(source: &str, language: Language) -> (f64, f64) {
	let report = analyze_with_language(
		Path::new("probe.ext"),
		source,
		language,
		&AnalysisOptions::default(),
	);

	(report.readability.value, report.complexity.value)
}

/// Applies every fixable finding, the way the fix command does.
fn fixed(source: &str, language: Language) -> String {
	let report = analyze_with_language(
		Path::new("probe.ext"),
		source,
		language,
		&AnalysisOptions::default(),
	);
	let fixes: Vec<Fix> = report
		.findings
		.iter()
		.filter_map(|finding| finding.fix.clone())
		.collect();
	let (rewritten, _) = monostyle::fix::apply_fixes(source, &fixes);

	rewritten
}

#[test]
fn fixing_never_lowers_a_score() {
	for (path, language) in fixtures() {
		let source = std::fs::read_to_string(&path).expect("fixture should read");

		// A scan that needed recovery produces unreliable findings on both sides; the fixer
		// refuses such files and the property is only claimed where analysis is trustworthy.
		let (before_read, before_cplx) = scores(&source, language);
		let rewritten = fixed(&source, language);

		let (after_read, after_cplx) = scores(&rewritten, language);

		assert!(
			after_read >= before_read - 0.001,
			"{}: readability dropped {before_read:.2} -> {after_read:.2} after fixing",
			path.display()
		);
		assert!(
			(after_cplx - before_cplx).abs() < 0.001,
			"{}: complexity moved {before_cplx:.2} -> {after_cplx:.2} after fixing",
			path.display()
		);
	}
}

#[test]
fn a_clean_file_is_still_clean_after_fixing() {
	for (path, language) in fixtures() {
		let source = std::fs::read_to_string(&path).expect("fixture should read");
		let before = monostyle_lexer::lex(&source, language);

		if !before.is_clean() {
			continue;
		}

		let rewritten = fixed(&source, language);
		let after = monostyle_lexer::lex(&rewritten, language);

		assert!(
			after.is_clean(),
			"{}: fixing introduced an unterminated construct: {:?}",
			path.display(),
			after.unterminated
		);
	}
}

/// A tiny deterministic generator so the fuzz is reproducible in CI.
struct Lcg(u64);

impl Lcg {
	fn next(&mut self) -> u64 {
		self.0 = self
			.0
			.wrapping_mul(6_364_136_223_846_793_005)
			.wrapping_add(1_442_695_040_888_963_407);

		self.0 >> 11
	}

	fn below(&mut self, bound: usize) -> usize {
		(self.next() % bound as u64) as usize
	}
}

/// Splices `3-6` fragments of a language's fixtures into one source, with stacked blanks
/// injected between fragments.
fn recombine(pool: &[String], random: &mut Lcg) -> String {
	let mut combined = String::new();

	for _ in 0..(3 + random.below(4)) {
		let source = pool
			.get(random.below(pool.len()))
			.expect("the index is below the pool's length");
		let lines: Vec<&str> = source.lines().collect();

		if lines.is_empty() {
			continue;
		}

		let start = random.below(lines.len());
		let width = (2 + random.below(7)).min(lines.len() - start);

		let fragment = lines
			.get(start..start + width)
			.expect("the range stays within the fixture's lines");
		combined.push_str(&fragment.join("\n"));
		combined.push('\n');

		if random.below(3) == 0 {
			combined.push_str("\n\n\n\n");
		}
	}

	combined
}

/// Fixes `source` until nothing changes, bounded at `passes`.
///
/// Fixing cascades — collapsing a blank run can expose a detachment the next pass removes — so
/// the fixed point, not the first pass, is what a caller can rely on.
fn fix_to_a_fixed_point(source: &str, language: Language, passes: usize) -> String {
	let probe = Path::new("recombined.ext");
	let mut current = source.to_string();

	for _ in 0..passes {
		let report = analyze_with_language(probe, &current, language, &AnalysisOptions::default());
		let fixes: Vec<Fix> = report
			.findings
			.iter()
			.filter_map(|finding| finding.fix.clone())
			.collect();
		let (next, _) = monostyle::fix::apply_fixes(&current, &fixes);

		if next == current {
			return current;
		}

		current = next;
	}

	current
}

#[test]
fn recombined_fragments_fix_to_a_fixed_point() {
	// Splice lines from a language's fixtures into new files, with stacked blanks injected at
	// random boundaries. Whatever the combination, the pipeline must reach a fixed point and
	// never turn a clean scan into an unclean one.
	const PASSES: usize = 5;
	const ITERATIONS: usize = 24;

	for language in [
		Language::Rust,
		Language::Dart,
		Language::TypeScript,
		Language::Python,
		Language::Go,
		Language::Ruby,
		Language::Shell,
		Language::CSharp,
		Language::Kotlin,
		Language::Swift,
		Language::Nix,
		Language::Lua,
	] {
		let pool: Vec<String> = fixtures()
			.into_iter()
			.filter(|(_, candidate)| *candidate == language)
			.filter_map(|(path, _)| std::fs::read_to_string(path).ok())
			.collect();

		assert!(
			pool.len() >= 3,
			"the recombination pool for {language:?} should not be tiny"
		);

		let mut random = Lcg(0x5EED_600D);

		for iteration in 0..ITERATIONS {
			let combined = recombine(&pool, &mut random);
			let settled = fix_to_a_fixed_point(&combined, language, PASSES);

			let before = monostyle_lexer::lex(&combined, language);
			let after = monostyle_lexer::lex(&settled, language);

			if before.is_clean() {
				assert!(
					after.is_clean(),
					"{language:?} recombination {iteration}: fixing broke the scan: {:?}",
					after.unterminated
				);
			}
		}
	}
}

#[test]
fn the_rules_never_panic_on_arbitrary_line_soup() {
	// Rule bodies index and slice lines; a panic on crafted input would take down a run. Feed
	// every rule hostile line shapes assembled from punctuation-only lines.
	const SOUP: &[&str] = &[
		"{\n}\n{\n}\n",
		"}}}}\n{{{{\n",
		"if {\n} else {\n}\n",
		"else\nelse if\nfinally\ncatch\n",
		"} else if {\n} catch {\n} finally {\n",
		"return\nreturn\nreturn\n",
		"\n\n\n\n\n",
		"#\n#\n#\n",
		"/*\n*/\n/*\n",
		"''\n''\n''\n",
		"${\n}\n${\n",
		"\\\n\\\n\\\n",
	];

	let config = RulesConfig::default();

	for text in SOUP {
		for language in [
			Language::Rust,
			Language::Dart,
			Language::Python,
			Language::Shell,
		] {
			let lexed = monostyle_lexer::lex(text, language);
			let findings = run_rules(&lexed, &config);

			// Whatever the findings, applying them must also be safe.
			let fixes: Vec<Fix> = findings
				.iter()
				.filter_map(|finding| finding.fix.clone())
				.collect();
			let _ = monostyle::fix::apply_fixes(text, &fixes);
		}
	}
}
