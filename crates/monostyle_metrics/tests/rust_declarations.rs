//! Rust visibility and qualifiers must not turn data declarations into functions.

use monostyle_core::Language;
use monostyle_lexer::lex;
use monostyle_metrics::CodeUnit;
use monostyle_metrics::find_units;

const VISIBILITIES: &[&str] = &[
	"",
	"pub",
	"pub(super)",
	"pub(crate)",
	"pub(self)",
	"pub(in crate)",
	"pub ( super )",
];

const TABLE_ROWS: usize = 64;

/// Exercises the public lexer-to-metrics boundary rather than a private recognizer.
fn rust_units(source: &str) -> Vec<CodeUnit> {
	find_units(&lex(source, Language::Rust))
}

#[test]
fn visibility_does_not_absorb_constant_tables_into_following_functions() {
	let first_function_line = TABLE_ROWS + 5;
	let last_function_line = TABLE_ROWS + 7;

	for visibility in VISIBILITIES {
		let rows = "        0,\n".repeat(TABLE_ROWS);
		let source = format!(
			"mod data {{\n    {visibility} const ROWS: &[u8] = &[\n{rows}    ];\n\n\
    {visibility} fn rows() -> usize {{\n        ROWS.len()\n    }}\n}}\n"
		);
		let units = rust_units(&source);

		assert_eq!(
			units.len(),
			1,
			"visibility: {visibility:?}; units: {units:?}"
		);

		let unit = units.first().expect("the rows function is present");

		assert_eq!(unit.name, "rows", "visibility: {visibility:?}");
		assert_eq!(unit.start_line, first_function_line);
		assert_eq!(unit.end_line, last_function_line);
	}
}

#[test]
fn real_functions_keep_their_names_with_scoped_visibility_and_qualifiers() {
	let declarations = [
		("fn private() {}", "private"),
		("pub fn public() {}", "public"),
		("pub(super) const fn constant() {}", "constant"),
		("pub(crate) async fn asynchronous() {}", "asynchronous"),
		("pub(self) unsafe fn unchecked() {}", "unchecked"),
		("pub(in crate) extern \"C\" fn foreign() {}", "foreign"),
		(
			"pub(super) unsafe extern \"C\" fn callback() {}",
			"callback",
		),
		("pub(super) fn generic<F: Fn()>(f: F) { f(); }", "generic"),
	];

	for (declaration, expected_name) in declarations {
		let source = format!("mod functions {{\n    {declaration}\n}}\n");
		let units = rust_units(&source);

		assert_eq!(
			units.len(),
			1,
			"declaration: {declaration}; units: {units:?}"
		);

		let unit = units.first().expect("the function is present");

		assert_eq!(unit.name, expected_name, "declaration: {declaration}");
		assert_eq!(unit.start_line, 2);
		assert_eq!(unit.end_line, 2);
	}
}

#[test]
fn function_pointer_data_and_types_are_not_function_declarations() {
	let declarations = [
		"pub(super) const CALLBACK: fn() = other::callback;",
		"pub(crate) static CALLBACK: fn() = other::callback;",
		"pub(self) const CALLBACKS: &[fn()] = &[other::callback];",
		"pub(in crate) type Callback = fn();",
		"pub(super) type Callback = unsafe extern \"C\" fn();",
		"pub(super) struct Handler { callback: fn() }",
		"pub(super) const ROWS: &[u8] = &[other::value()];",
	];

	for declaration in declarations {
		let source = format!("mod data {{\n    {declaration}\n}}\n");
		let units = rust_units(&source);

		assert!(
			units.is_empty(),
			"declaration: {declaration}; units: {units:?}"
		);
	}
}

#[test]
fn scoped_multiline_parameters_and_bodyless_methods_keep_their_boundaries() {
	let source = "\
mod functions {
    pub(super) fn multiline(
        value: usize,
    ) -> usize {
        value
    }

    unsafe extern \"C\" {
        pub(super) safe fn callback();
    }
}
";
	let units = rust_units(source);
	let actual: Vec<_> = units
		.iter()
		.map(|unit| (unit.name.as_str(), unit.start_line, unit.end_line))
		.collect();

	assert_eq!(actual, [("multiline", 2, 6), ("callback", 9, 9)]);
}

#[test]
fn pub_remains_a_valid_function_name_in_other_languages() {
	let declarations = [
		(Language::C, "int pub(int scope) { return scope; }"),
		(
			Language::JavaScript,
			"function pub(scope) { return scope; }",
		),
		(Language::Shell, "pub() { printf scope; }"),
	];

	for (language, declaration) in declarations {
		let units = find_units(&lex(declaration, language));
		let names: Vec<_> = units.iter().map(|unit| unit.name.as_str()).collect();

		assert_eq!(names, ["pub"], "language: {language}");
	}
}

#[test]
fn raw_function_identifiers_keep_their_names_and_boundaries() {
	const PRIVATE_LINE: usize = 2;
	const SCOPED_START: usize = 4;
	const SCOPED_END: usize = 6;
	const ASYNC_START: usize = 8;
	const ASYNC_END: usize = 10;
	const BODYLESS_LINE: usize = 13;
	const FINAL_LINE: usize = 16;

	let source = "\
mod functions {
    fn r#type() {}

    pub(crate) fn r#match(value: bool) -> bool {
        value
    }

    pub(super) async fn r#async<Value: Fn()>(callback: Value) {
        callback();
    }

    trait Callback {
        fn r#move();
    }

    fn ordinary() {}
}
";
	let units = rust_units(source);
	let actual: Vec<_> = units
		.iter()
		.map(|unit| (unit.name.as_str(), unit.start_line, unit.end_line))
		.collect();

	assert_eq!(
		actual,
		[
			("type", PRIVATE_LINE, PRIVATE_LINE),
			("match", SCOPED_START, SCOPED_END),
			("async", ASYNC_START, ASYNC_END),
			("move", BODYLESS_LINE, BODYLESS_LINE),
			("ordinary", FINAL_LINE, FINAL_LINE),
		]
	);
}
