//! Tests for unit detection and file scoring.
//!
//! Unit detection is approximate by design — it is structural rather than grammatical — so these tests
//! pin the cases it must get right and the cases it must not claim: a call is not a function, and a
//! nested block does not end its enclosing function.

use monostyle_core::Language;
use monostyle_lexer::lex;
use monostyle_metrics::find_units;

/// Returns the names of the units detected in `source`.
fn names(source: &str, language: Language) -> Vec<String> {
	let lexed = lex(source, language);

	find_units(&lexed)
		.into_iter()
		.map(|unit| unit.name)
		.collect()
}

/// Returns the first unit detected in `source`.
fn first_unit(source: &str, language: Language) -> monostyle_metrics::CodeUnit {
	let lexed = lex(source, language);

	find_units(&lexed)
		.into_iter()
		.next()
		.expect("a unit should be detected")
}

// ---------------------------------------------------------------------------
// Brace languages
// ---------------------------------------------------------------------------

#[test]
fn a_rust_function_is_detected() {
	assert!(names("fn compute() -> i32 { 1 }\n", Language::Rust).contains(&"compute".to_string()));
}

#[test]
fn a_rust_function_span_covers_its_whole_body() {
	let source = "fn outer() {\n    let a = 1;\n    fn inner() { work(); }\n    let b = 2;\n}\n";

	let unit = first_unit(source, Language::Rust);

	assert_eq!(unit.name, "outer");
	assert_eq!(unit.start_line, 1);
	assert_eq!(
		unit.end_line, 5,
		"a nested function must not end its parent early"
	);
}

#[test]
fn a_one_line_function_is_detected() {
	let unit = first_unit("fn short() { work(); }\n", Language::Rust);

	assert_eq!(unit.start_line, 1);
	assert_eq!(unit.end_line, 1);
}

#[test]
fn calls_are_not_mistaken_for_declarations() {
	// A declaration detector that accepts any identifier before a parenthesis reports `compute` here
	// as a function of its own, which then appears in the per-function table as noise.
	let source = "\
fn outer() {
    let a = compute(1, 2);
    let b = transform(a);
    if ready() { work(); }
    Ok(())
}
";
	let detected = names(source, Language::Rust);

	assert!(detected.contains(&"outer".to_string()));
	assert!(
		!detected.contains(&"compute".to_string()),
		"a call is not a declaration: {detected:?}"
	);
	assert!(
		!detected.contains(&"transform".to_string()),
		"a call is not a declaration: {detected:?}"
	);
	assert!(
		!detected.contains(&"ready".to_string()),
		"a call is not a declaration: {detected:?}"
	);
	assert!(
		!detected.contains(&"Ok".to_string()),
		"a call is not a declaration: {detected:?}"
	);
}

#[test]
fn several_functions_are_all_detected() {
	let source = "fn a() {}\n\nfn b() {}\n\nfn c() {}\n";
	let detected = names(source, Language::Rust);

	assert_eq!(detected, vec!["a", "b", "c"]);
}

#[test]
fn a_go_function_is_detected() {
	assert!(
		names("func Compute() int { return 1 }\n", Language::Go).contains(&"Compute".to_string())
	);
}

#[test]
fn a_typescript_function_is_detected() {
	assert!(
		names(
			"function compute(): number { return 1; }\n",
			Language::TypeScript
		)
		.contains(&"compute".to_string())
	);
}

#[test]
fn a_typescript_arrow_function_assignment_is_detected_by_its_parameter_list() {
	// An arrow function has no declaration keyword, so its name comes from the assignment target.
	let detected = names(
		"const compute = (a: number) => a + 1;\n",
		Language::TypeScript,
	);

	assert!(
		detected.contains(&"compute".to_string()),
		"got {detected:?}"
	);
}

// ---------------------------------------------------------------------------
// Indentation languages
// ---------------------------------------------------------------------------

#[test]
fn a_python_function_is_detected_with_its_indented_body() {
	let source = "def compute(a, b):\n    total = a + b\n    return total\n\nx = 1\n";

	let unit = first_unit(source, Language::Python);

	assert_eq!(unit.name, "compute");
	assert_eq!(unit.start_line, 1);
	assert_eq!(
		unit.end_line, 3,
		"the body ends where the indentation returns"
	);
}

#[test]
fn a_python_body_ends_at_a_dedent() {
	let source = "def a():\n    work()\n\ndef b():\n    work()\n";

	let detected = names(source, Language::Python);

	assert_eq!(
		detected,
		vec!["a", "b"],
		"the first function must not swallow the second"
	);
}

// ---------------------------------------------------------------------------
// End-keyword languages
// ---------------------------------------------------------------------------

#[test]
fn a_ruby_method_is_detected_with_its_end() {
	let source = "def compute(a)\n  a + 1\nend\n";

	let unit = first_unit(source, Language::Ruby);

	assert_eq!(unit.name, "compute");
	assert_eq!(unit.end_line, 3, "the method ends at `end`");
}

#[test]
fn a_shell_function_is_detected() {
	let source = "run() {\n  echo hi\n}\n";

	let detected = names(source, Language::Shell);

	assert!(!detected.is_empty(), "a shell function should be detected");
}

// ---------------------------------------------------------------------------
// Span properties
// ---------------------------------------------------------------------------

#[test]
fn a_unit_reports_its_line_count() {
	let source = "fn a() {\n    work();\n    work();\n}\n";
	let unit = first_unit(source, Language::Rust);

	assert_eq!(unit.line_count(), 4);
}

#[test]
fn contains_reports_membership_inclusively() {
	let source = "fn a() {\n    work();\n}\n";
	let unit = first_unit(source, Language::Rust);

	assert!(unit.contains(1), "the first line is inside");
	assert!(unit.contains(3), "the last line is inside");
	assert!(!unit.contains(0), "line zero does not exist");
	assert!(!unit.contains(4), "the line after the unit is outside");
}

#[test]
fn every_language_detects_units_without_panicking() {
	// The detector has a different code path per block style, and an unhandled style would panic or
	// silently return nothing. This exercises all three.
	for language in Language::ALL {
		if language == Language::Markdown {
			continue;
		}

		let lexed = lex("function compute(a) {\n    return a\n}\n", language);

		// The assertion is that detection terminates and produces a coherent result, not that this
		// pseudo-source is recognized in every language.
		let _ = find_units(&lexed);
	}
}

#[test]
fn a_file_with_no_functions_detects_nothing() {
	let detected = names("const A: i32 = 1;\nconst B: i32 = 2;\n", Language::Rust);

	assert!(
		detected.is_empty(),
		"no declaration is present: {detected:?}"
	);
}

#[test]
fn an_unterminated_unit_still_reports_its_lines() {
	// A file that ends mid-function should still attribute those lines, or the complexity disappears
	// from the report entirely.
	let unit = first_unit("fn a() {\n    work();\n", Language::Rust);

	assert_eq!(unit.name, "a");
	assert!(unit.end_line >= 2);
}
