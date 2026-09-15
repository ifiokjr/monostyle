//! Terminal styling.
//!
//! Colour is applied through small helpers rather than scattered escape codes so that the decision
//! to colour lives in one place. Every helper degrades to plain text when colour is disabled, which
//! is what makes `--no-color`, `NO_COLOR`, and non-TTY output work without special cases at each
//! call site.

use std::io::IsTerminal;
use std::sync::OnceLock;

/// Whether colour should be emitted.
static COLOR_ENABLED: OnceLock<bool> = OnceLock::new();

/// Decides once whether colour is appropriate for this process.
///
/// The environment is consulted before the terminal check because `NO_COLOR` is an explicit
/// instruction from the user, while a non-TTY is merely a hint.
pub fn init_color(force: bool, disable: bool) {
	let enabled = if disable {
		false
	} else if force {
		true
	} else if std::env::var_os("NO_COLOR").is_some()
		|| std::env::var("TERM").is_ok_and(|term| term == "dumb")
	{
		false
	} else {
		std::io::stdout().is_terminal()
	};

	let _ = COLOR_ENABLED.set(enabled);
}

/// Whether colour is enabled.
#[must_use]
pub fn color_enabled() -> bool {
	*COLOR_ENABLED.get().unwrap_or(&false)
}

/// Wraps `text` in an ANSI colour when colour is enabled.
fn paint(text: &str, code: &str) -> String {
	if color_enabled() {
		format!("\x1b[{code}m{text}\x1b[0m")
	} else {
		text.to_string()
	}
}

/// Bold text, for headings and labels.
#[must_use]
pub fn bold(text: &str) -> String {
	paint(text, "1")
}

/// Dimmed text, for secondary detail.
#[must_use]
pub fn dim(text: &str) -> String {
	paint(text, "2")
}

/// Green, for healthy values.
#[must_use]
pub fn green(text: &str) -> String {
	paint(text, "32")
}

/// Yellow, for values worth attention.
#[must_use]
pub fn yellow(text: &str) -> String {
	paint(text, "33")
}

/// Red, for values that are a problem.
#[must_use]
pub fn red(text: &str) -> String {
	paint(text, "31")
}

/// Cyan, for paths and identifiers.
#[must_use]
pub fn cyan(text: &str) -> String {
	paint(text, "36")
}

/// Magenta, for rule names.
#[must_use]
pub fn magenta(text: &str) -> String {
	paint(text, "35")
}

/// Colours a score by how good it is.
///
/// The bands match [`Score::grade`](monostyle_core::Score::grade) so the colour and the word beside
/// it never disagree.
#[must_use]
pub fn score_color(value: f64) -> fn(&str) -> String {
	match value {
		value if value >= 90.0 => green,
		value if value >= 75.0 => green,
		value if value >= 60.0 => yellow,
		_ => red,
	}
}

/// Renders a score with its colour applied.
#[must_use]
pub fn score(value: f64) -> String {
	let text = format!("{value:.1}");
	score_color(value)(&text)
}

/// Draws a proportional bar for a score out of 100.
///
/// Uses block characters that are wide in every common terminal font, so the bar's length reads
/// correctly rather than appearing ragged.
#[must_use]
pub fn bar(value: f64, width: usize) -> String {
	let filled = ((value.clamp(0.0, 100.0) / 100.0) * width as f64).round() as usize;
	let empty = width.saturating_sub(filled);

	let filled_text = "█".repeat(filled);
	let empty_text = "░".repeat(empty);
	let colour = score_color(value);

	if color_enabled() {
		format!("{}{}", colour(&filled_text), dim(&empty_text))
	} else {
		format!("{filled_text}{empty_text}")
	}
}

/// Draws a proportional bar for a relative share from 0 to 1.
#[must_use]
pub fn share_bar(share: f64, width: usize) -> String {
	bar(share.clamp(0.0, 1.0) * 100.0, width)
}

/// A horizontal rule sized for a terminal.
#[must_use]
pub fn rule(width: usize) -> String {
	dim(&"─".repeat(width))
}

/// The width available for output, capped so lines stay readable on wide terminals.
#[must_use]
pub fn width() -> usize {
	std::env::var("COLUMNS")
		.ok()
		.and_then(|columns| columns.parse::<usize>().ok())
		.unwrap_or(80)
		.clamp(60, 100)
}
