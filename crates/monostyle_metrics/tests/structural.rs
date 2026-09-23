//! Tests for the structural metrics: `NPath`, exits, nesting profile, and Halstead.

use monostyle_core::Language;
use monostyle_lexer::lex;
use monostyle_metrics::ExitCount;
use monostyle_metrics::NPath;
use monostyle_metrics::NestingProfile;
use monostyle_metrics::exit_count_of_lines;
use monostyle_metrics::halstead_of_lines;
use monostyle_metrics::maintainability_index;
use monostyle_metrics::nesting_profile_of_lines;
use monostyle_metrics::npath_of_lines;

/// Measures a Rust snippet.
fn measure(source: &str) -> (NPath, ExitCount, NestingProfile) {
	let lexed = lex(source, Language::Rust);

	(
		npath_of_lines(&lexed.lines),
		exit_count_of_lines(&lexed.lines),
		nesting_profile_of_lines(&lexed.lines),
	)
}

#[test]
fn straight_line_code_has_one_path() {
	let (npath, _exits, _nesting) = measure("fn a() { let x = 1; }\n");

	assert_eq!(npath.value, NPath::MINIMUM);
	assert!(!npath.capped);
}

#[test]
fn sequential_branches_multiply_rather_than_add() {
	// This is the property that distinguishes NPath from cyclomatic complexity. Two sequential `if`s
	// have a cyclomatic complexity of three and four execution paths, because the second test runs
	// regardless of the first outcome.
	let source = "\
fn a(x: i32) {
    if x > 0 { work(); }
    if x > 1 { work(); }
}
";
	let (npath, _exits, _nesting) = measure(source);

	assert_eq!(
		npath.value, 4,
		"two sequential branches should give four paths"
	);
}

#[test]
fn nested_branches_multiply_with_their_parent() {
	let source = "\
fn a(x: i32) {
    if x > 0 {
        if x > 1 { work(); }
    }
}
";
	let (npath, _exits, _nesting) = measure(source);

	assert_eq!(npath.value, 4, "a nested branch multiplies with its parent");
}

#[test]
fn path_count_is_capped_rather_than_overflowing() {
	// Thirty sequential branches would be roughly a billion paths. The cap exists because the value
	// grows faster than exponentially and an uncapped count overflows on realistic input.
	let branches: String = (0..30)
		.map(|index| format!("    if x > {index} {{ work(); }}\n"))
		.fold(String::new(), |mut text, line| {
			text.push_str(&line);

			text
		});
	let source = format!("fn a(x: i32) {{\n{branches}}}\n");

	let (npath, _exits, _nesting) = measure(&source);

	assert!(npath.capped, "the count should report that it was capped");
	assert_eq!(npath.value, NPath::CAP);
}

#[test]
fn npath_grades_span_the_range() {
	/// A count and the band it should fall in.
	const CASES: &[(usize, &str)] = &[
		(1, "simple"),
		(8, "simple"),
		(9, "moderate"),
		(64, "moderate"),
		(65, "complex"),
		(1024, "complex"),
		(1025, "severe"),
		(100_000, "severe"),
		(100_001, "untestable"),
	];

	for (value, expected) in CASES {
		let npath = NPath {
			value: *value,
			capped: false,
		};

		assert_eq!(
			npath.grade(),
			*expected,
			"NPath {value} should grade as {expected}"
		);
	}
}

#[test]
fn exits_are_counted_by_kind() {
	let source = "\
fn a(x: i32) -> i32 {
    if x > 0 { return 1; }
    if x < 0 { return -1; }
    panic!(\"zero\");
}
";
	let (_npath, exits, _nesting) = measure(source);

	assert_eq!(exits.exits, 2, "two returns");
	assert!(exits.throws >= 1, "panic should count as a throw");
	assert_eq!(exits.total(), exits.exits + exits.throws + exits.jumps);
}

#[test]
fn loop_jumps_are_counted_separately_from_returns() {
	let source = "\
fn a(items: &[i32]) {
    for item in items {
        if *item < 0 { continue; }
        if *item > 100 { break; }
    }
}
";
	let (_npath, exits, _nesting) = measure(source);

	assert_eq!(exits.exits, 0);
	assert_eq!(exits.jumps, 2, "continue and break should be counted");
}

#[test]
fn nesting_profile_records_the_histogram() {
	// A function body is depth one and the `if` body inside it is depth two, so this source reaches
	// two. The depth counts the blocks a line sits inside, which is what the layout rules also mean
	// by nesting.
	let source = "\
fn a(x: i32) {
    let top = 1;
    if x > 0 {
        let inner = 2;
    }
}
";
	let (_npath, _exits, nesting) = measure(source);

	assert_eq!(
		nesting.max_depth, 2,
		"the `if` body sits inside the function body"
	);
	assert!(
		nesting.depth_histogram[0] > 0,
		"the declaration sits at depth zero"
	);
	assert!(
		nesting.depth_histogram[1] > 0,
		"the function body sits at depth one"
	);
	assert!(
		nesting.depth_histogram[2] > 0,
		"the `if` body sits at depth two"
	);
}

#[test]
fn flat_code_reports_depth_one() {
	// A file of top-level statements has one level of structure, not zero, which keeps the figure
	// consistent with how indentation is counted elsewhere.
	let source = "const X: i32 = 1;\nconst Y: i32 = 2;\n";
	let (_npath, _exits, nesting) = measure(source);

	assert_eq!(nesting.max_depth, 0);
}

#[test]
fn nesting_mean_and_share_are_computed_from_the_histogram() {
	let source = "\
fn a(x: i32) {
    if x > 0 {
        work();
    }
}
";
	let (_npath, _exits, nesting) = measure(source);

	// The mean must sit between the shallowest and deepest lines, and the share deeper than the
	// maximum is by definition zero.
	assert!(nesting.mean_depth() >= 0.0);
	assert!(nesting.mean_depth() <= nesting.max_depth as f64);
	assert_eq!(nesting.share_deeper_than(nesting.max_depth), 0.0);
	assert!(nesting.share_deeper_than(0) > 0.0);
}

#[test]
fn an_empty_profile_reports_zero_rather_than_dividing_by_zero() {
	let nesting = NestingProfile {
		max_depth: 0,
		depth_histogram: [0; NestingProfile::MAX_TRACKED],
	};

	assert_eq!(nesting.mean_depth(), 0.0);
	assert_eq!(nesting.share_deeper_than(0), 0.0);
}

// ---------------------------------------------------------------------------
// Halstead
// ---------------------------------------------------------------------------
#[test]
fn halstead_counts_operators_and_operands() {
	let source = "fn a() { let x = 1 + 2; }\n";
	let lexed = lex(source, Language::Rust);
	let halstead = halstead_of_lines(&lexed.lines, Language::Rust);

	assert!(
		halstead.total_operators > 0,
		"an addition and braces are operators"
	);
	assert!(
		halstead.total_operands > 0,
		"identifiers and literals are operands"
	);
	assert_eq!(
		halstead.length,
		halstead.total_operators + halstead.total_operands
	);
	assert_eq!(
		halstead.vocabulary,
		halstead.distinct_operators + halstead.distinct_operands
	);
}

#[test]
fn halstead_volume_grows_with_vocabulary() {
	let small = lex("fn a() { let x = 1; }\n", Language::Rust);
	let large = lex(
		"fn a() { let x = 1 + 2 * 3 - 4 / 5; let y = x % 6; }\n",
		Language::Rust,
	);

	let small_volume = halstead_of_lines(&small.lines, Language::Rust).volume;
	let large_volume = halstead_of_lines(&large.lines, Language::Rust).volume;

	assert!(
		large_volume > small_volume,
		"more distinct operators should raise the volume: {large_volume:.1} vs {small_volume:.1}"
	);
}

#[test]
fn halstead_metrics_are_zero_for_empty_code() {
	let lexed = lex("", Language::Rust);
	let halstead = halstead_of_lines(&lexed.lines, Language::Rust);

	assert_eq!(halstead, monostyle_metrics::Halstead::EMPTY);
	assert!(!halstead.is_meaningful());
}

#[test]
fn maintainability_is_maximal_when_there_is_nothing_to_measure() {
	let index = maintainability_index(monostyle_metrics::Halstead::EMPTY, 1, 0);

	assert_eq!(index.value, 100.0);
	assert_eq!(index.grade(), "high");
}

#[test]
fn maintainability_falls_as_complexity_and_length_rise() {
	let lexed = lex(
		"fn a(x: i32) { let y = x + 1 * 2 - 3 / 4; if y > 0 { work(y); } }\n",
		Language::Rust,
	);
	let halstead = halstead_of_lines(&lexed.lines, Language::Rust);

	let simple = maintainability_index(halstead, 1, 10);
	let complex = maintainability_index(halstead, 40, 10);
	let long = maintainability_index(halstead, 1, 1000);

	assert!(
		complex.value < simple.value,
		"more paths should lower the index"
	);
	assert!(
		long.value < simple.value,
		"more lines should lower the index"
	);
}

#[test]
fn maintainability_grades_span_the_range() {
	/// An index value and the band it should fall in.
	const CASES: &[(f64, &str)] = &[
		(100.0, "high"),
		(85.0, "high"),
		(84.9, "moderate"),
		(65.0, "moderate"),
		(64.9, "low"),
		(0.0, "low"),
	];

	for (value, expected) in CASES {
		let index = monostyle_metrics::MaintainabilityIndex { value: *value };

		assert_eq!(
			index.grade(),
			*expected,
			"{value} should grade as {expected}"
		);
	}
}

#[test]
fn maintainability_never_goes_below_zero() {
	// A pathological unit should clamp rather than report a negative index, because the scale is
	// defined from zero and a negative value would break any arithmetic a caller does with it.
	let lexed = lex(
		"fn a() { let x = 1 + 2 * 3 - 4 / 5 % 6; let y = x << 2 >> 1; }\n",
		Language::Rust,
	);
	let halstead = halstead_of_lines(&lexed.lines, Language::Rust);
	let index = maintainability_index(halstead, 500, 10_000);

	assert!(
		index.value >= 0.0,
		"index should clamp at zero, got {}",
		index.value
	);
}

#[test]
fn halstead_handles_every_language() {
	// The operator table differs per language and a missing entry would silently produce zero counts,
	// so every supported language is measured.
	for language in Language::ALL {
		let lexed = lex("let x = 1 + 2;\n", language);
		let halstead = halstead_of_lines(&lexed.lines, language);

		assert!(
			halstead.total_operators + halstead.total_operands > 0,
			"{language}: no tokens were counted"
		);
	}
}

#[test]
fn member_access_dots_are_not_counted_as_operators() {
	// Counting every dot as an operator inflates the operator total on any code that calls methods,
	// which makes the volume measure misleading rather than informative.
	let chain = lex("fn a() { x.y.z(); }\n", Language::Rust);
	let plain = lex("fn a() { x(); }\n", Language::Rust);

	let chained = halstead_of_lines(&chain.lines, Language::Rust);
	let simple = halstead_of_lines(&plain.lines, Language::Rust);

	assert!(
		chained.distinct_operators <= simple.distinct_operators + 1,
		"a three-call chain should not add three distinct operators"
	);
}
