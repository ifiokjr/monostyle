//! Allocation ceilings for the scanner.
//!
//! # Why count allocations rather than time
//!
//! Wall-clock benchmarks run on shared machines where only the minimum of many runs can be
//! trusted, and a regression that adds one heap allocation per character is invisible in the
//! timing of a small file. Counting allocations is exact, deterministic, and meaningful in a
//! debug build, so a per-character or per-line allocation creeping back into the scanner fails
//! this test instead of the next profiling session.
//!
//! The ceilings sit a little above the measured figures so an incidental change does not trip
//! them, and far below the implementation they replaced — which made 64 to 72 allocations per
//! line, one per character — so none of its costs can return unnoticed.
//!
//! The counter is a global allocator, so every measurement in this file runs inside one test
//! function: the harness runs tests in parallel threads, and a second test allocating at the
//! same time would be counted against the first.

// Implementing the allocator trait is the one unsafe thing in this workspace. The implementation
// counts calls and forwards every one to the system allocator unchanged, which is as small as an
// allocator can be.
#![allow(unsafe_code)]

use std::alloc::GlobalAlloc;
use std::alloc::Layout;
use std::alloc::System;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::Relaxed;

use monostyle_core::Language;
use monostyle_lexer::lex;

/// Counts allocations and forwards them to the system allocator.
struct Counting;

/// Allocations made since the process started.
static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
	unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
		ALLOCATIONS.fetch_add(1, Relaxed);

		// SAFETY: the layout is passed through unchanged to the system allocator, which upholds
		// the trait's contract for it.
		unsafe { System.alloc(layout) }
	}

	unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
		// SAFETY: every pointer this allocator hands out came from `System.alloc` with this same
		// layout, so returning it to `System.dealloc` is exactly what the contract requires.
		unsafe { System.dealloc(pointer, layout) }
	}
}

#[global_allocator]
static GLOBAL: Counting = Counting;

/// Lines each synthetic sample is built from, enough for the per-file costs to wash out.
const LINES: usize = 1_000;

/// Counts the allocations `lex` makes over `source`, per line.
fn allocations_per_line(source: &str, language: Language) -> f64 {
	let lines = source.lines().count().max(1);
	let before = ALLOCATIONS.load(Relaxed);
	let lexed = lex(source, language);
	let during = ALLOCATIONS.load(Relaxed) - before;

	drop(lexed);

	during as f64 / lines as f64
}

/// A source of `LINES` copies of `line`.
fn repeated(line: &str) -> String {
	let mut source = String::with_capacity((line.len() + 1) * LINES);

	for _ in 0..LINES {
		source.push_str(line);
		source.push('\n');
	}

	source
}

/// Asserts that lexing `line` repeated stays under `ceiling` allocations per line.
fn assert_ceiling(name: &str, line: &str, language: Language, ceiling: f64) {
	let source = repeated(line);
	let measured = allocations_per_line(&source, language);

	assert!(
		measured <= ceiling,
		"{name}: {measured:.2} allocations per line, over the ceiling of {ceiling}"
	);
}

#[test]
fn the_scanner_allocates_per_line_not_per_character() {
	// A blank line builds nothing: no words, no mask, no comment.
	assert_ceiling("blank", "", Language::Rust, 0.05);

	// A plain code line allocates once, for its list of words. The ceiling allows one more and no
	// further: the costs this guards against are one allocation per character.
	assert_ceiling(
		"plain code",
		"    let value = other + 1;",
		Language::Rust,
		2.0,
	);
	assert_ceiling("plain code", "    value = other + 1", Language::Python, 2.0);
	assert_ceiling(
		"plain code",
		"    value=$((other + 1))",
		Language::Shell,
		2.0,
	);
	assert_ceiling(
		"call with arguments",
		"    compute(first, second, third);",
		Language::Rust,
		2.0,
	);

	// A decision adds the keyword lists.
	assert_ceiling(
		"control flow",
		"    if value > other { return; }",
		Language::Rust,
		4.0,
	);

	// Literal content forces the mask apart from the text, and the opener reads the rest of the
	// line once; neither happens per character inside the string.
	assert_ceiling(
		"string literal",
		"    let text = \"hello world\";",
		Language::Rust,
		8.0,
	);

	// A comment owns its body and is classified once per line and once per block.
	assert_ceiling(
		"line comment",
		"    // a comment explaining why",
		Language::Rust,
		7.0,
	);
	assert_ceiling(
		"trailing comment",
		"    let a = 1; // trailing",
		Language::Rust,
		7.0,
	);
	assert_ceiling(
		"doc comment",
		"    /// Documents the item below.",
		Language::Rust,
		5.0,
	);

	// The scanner's own source: long, comment-heavy, full of literals and keywords, and the file
	// that measured 72 allocations per line before the per-character costs were removed.
	let source = include_str!("../src/lexer.rs");
	let measured = allocations_per_line(source, Language::Rust);

	assert!(
		measured <= 4.0,
		"lexing lexer.rs made {measured:.2} allocations per line, over the ceiling of 4"
	);
}
