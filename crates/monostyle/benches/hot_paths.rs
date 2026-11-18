//! Micro-benchmarks for the per-file hot paths: lexing, unit measurement, the rule set, and the
//! whole analysis of one file.
//!
//! # Why these four
//!
//! Every file an analysis touches goes through exactly this sequence, so a regression in any of
//! them is paid once per file across a whole repository. The benchmark workflow times the binary
//! over real trees and catches the total; these isolate each stage, so a change that slows the
//! lexer by a third cannot hide behind a rule set that got faster.
//!
//! # Why the corpus is repeated
//!
//! Each sample is grown to a few thousand lines by repetition. Per-file fixed costs — building
//! the profile, allocating the line vector — then disappear into the measurement, and the lexer's
//! figure is reported as bytes per second, which is the number to compare across languages.

use std::hint::black_box;
use std::path::Path;

use criterion::BenchmarkId;
use criterion::Criterion;
use criterion::Throughput;
use criterion::criterion_group;
use criterion::criterion_main;
use monostyle::analysis::AnalysisOptions;
use monostyle::analysis::analyze_with_language;
use monostyle_core::Language;
use monostyle_lexer::lex;
use monostyle_rules::RuleContext;
use monostyle_rules::RulesConfig;
use monostyle_rules::run_rules;
use monostyle_rules::unit_measures::UnitSet;

/// One corpus file and the language it is written in.
struct Sample {
	/// The name the benchmark reports under.
	name: &'static str,
	/// The language the scanner is told to use.
	language: Language,
	/// The file name analysis resolves the language from.
	path: &'static str,
	/// The sample's source.
	source: &'static str,
}

/// The corpus, one file per language, each written to exercise that language's hardest scanning:
/// nested interpolation, heredocs, raw strings, regex against division, nested comments.
const SAMPLES: &[Sample] = &[
	Sample {
		name: "rust",
		language: Language::Rust,
		path: "sample.rs",
		source: include_str!("corpus/rust.rs"),
	},
	Sample {
		name: "typescript",
		language: Language::TypeScript,
		path: "sample.ts",
		source: include_str!("corpus/typescript.ts"),
	},
	Sample {
		name: "python",
		language: Language::Python,
		path: "sample.py",
		source: include_str!("corpus/python.py"),
	},
	Sample {
		name: "dart",
		language: Language::Dart,
		path: "sample.dart",
		source: include_str!("corpus/dart.dart"),
	},
	Sample {
		name: "shell",
		language: Language::Shell,
		path: "sample.sh",
		source: include_str!("corpus/shell.sh"),
	},
	Sample {
		name: "ruby",
		language: Language::Ruby,
		path: "sample.rb",
		source: include_str!("corpus/ruby.rb"),
	},
	Sample {
		name: "go",
		language: Language::Go,
		path: "sample.go",
		source: include_str!("corpus/go.go"),
	},
	Sample {
		name: "markdown",
		language: Language::Markdown,
		path: "sample.md",
		source: include_str!("corpus/markdown.md"),
	},
	Sample {
		name: "nix",
		language: Language::Nix,
		path: "sample.nix",
		source: include_str!("corpus/nix.nix"),
	},
	Sample {
		name: "php",
		language: Language::Php,
		path: "sample.php",
		source: include_str!("corpus/php.php"),
	},
];

/// Lines a sample is grown to before it is measured.
const TARGET_LINES: usize = 3_000;

/// Repeats a sample until it reaches [`TARGET_LINES`].
fn grown(sample: &Sample) -> String {
	let lines = sample.source.lines().count().max(1);

	sample.source.repeat(TARGET_LINES.div_ceil(lines))
}

/// The scanner alone, in every language, reported as bytes per second.
fn lexing(criterion: &mut Criterion) {
	let mut group = criterion.benchmark_group("lex");

	for sample in SAMPLES {
		let source = grown(sample);

		group.throughput(Throughput::Bytes(source.len() as u64));
		group.bench_with_input(
			BenchmarkId::from_parameter(sample.name),
			&source,
			|bencher, source| {
				bencher.iter(|| black_box(lex(black_box(source), sample.language)));
			},
		);
	}

	group.finish();
}

/// Unit detection and measurement, once per block style the detector knows.
fn units(criterion: &mut Criterion) {
	let mut group = criterion.benchmark_group("units");

	// Braces, indentation, and end keywords are three different detectors, so one sample each.
	for name in ["rust", "python", "ruby"] {
		let sample = SAMPLES
			.iter()
			.find(|sample| sample.name == name)
			.expect("the corpus covers every block style");
		let source = grown(sample);
		let lexed = lex(&source, sample.language);

		group.bench_with_input(
			BenchmarkId::from_parameter(name),
			&lexed,
			|bencher, lexed| {
				bencher.iter(|| black_box(UnitSet::new(black_box(lexed))));
			},
		);
	}

	group.finish();
}

/// The whole rule set over an already-lexed file.
fn rules(criterion: &mut Criterion) {
	let mut group = criterion.benchmark_group("rules");
	let config = RulesConfig::default();

	for sample in SAMPLES {
		let source = grown(sample);
		let lexed = lex(&source, sample.language);

		group.bench_with_input(
			BenchmarkId::from_parameter(sample.name),
			&lexed,
			|bencher, lexed| {
				bencher.iter(|| {
					let context = RuleContext::new(black_box(lexed), &config);

					black_box(run_rules(&context))
				});
			},
		);
	}

	group.finish();
}

/// Everything a file costs: lexing, rules, metrics, and the report, as the CLI runs it.
fn analysis(criterion: &mut Criterion) {
	let mut group = criterion.benchmark_group("analyze");
	let options = AnalysisOptions::default();

	for sample in SAMPLES {
		let source = grown(sample);
		let path = Path::new(sample.path);

		group.throughput(Throughput::Bytes(source.len() as u64));
		group.bench_with_input(
			BenchmarkId::from_parameter(sample.name),
			&source,
			|bencher, source| {
				bencher.iter(|| {
					black_box(analyze_with_language(
						path,
						black_box(source),
						sample.language,
						&options,
					))
				});
			},
		);
	}

	group.finish();
}

criterion_group!(hot_paths, lexing, units, rules, analysis);
criterion_main!(hot_paths);
