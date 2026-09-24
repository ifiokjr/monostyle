//! Command-line interface definition.

use std::path::PathBuf;

use clap::Parser;
use clap::Subcommand;
use clap::ValueEnum;

/// Measure the complexity and readability of a codebase, a file, or a function.
///
/// Every point lost is traced to a named rule with an explanation, so a score is always
/// actionable rather than merely informative.
///
/// The global flags below are independent switches rather than a state machine, so they stay
/// flat.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Parser)]
#[command(name = "monostyle", version, about, long_about = None)]
pub struct Cli {
	/// What to analyze.
	#[command(subcommand)]
	pub command: Command,

	/// Path to a configuration file.
	#[arg(long, short, global = true, value_name = "FILE")]
	pub config: Option<PathBuf>,

	/// Turn a rule off. May be repeated.
	#[arg(long = "disable", global = true, value_name = "RULE")]
	pub disable: Vec<String>,

	/// Score strictly, lowering every threshold's tolerance.
	#[arg(long, global = true, conflicts_with = "lenient")]
	pub strict: bool,

	/// Score leniently, raising every threshold's tolerance.
	#[arg(long, global = true)]
	pub lenient: bool,

	/// Force coloured output even when stdout is not a terminal.
	#[arg(long, global = true, conflicts_with = "no_color")]
	pub color: bool,

	/// Disable coloured output.
	#[arg(long, global = true)]
	pub no_color: bool,
}

/// The available subcommands.
#[derive(Debug, Subcommand)]
pub enum Command {
	/// Analyze a directory, file, or list of paths.
	Check(CheckArgs),

	/// Apply every fixable finding to the files that have one.
	Fix(FixArgs),

	/// List every rule, or show details for one.
	Rules(RulesArgs),

	/// Print the effective configuration.
	Config {
		/// The output format.
		#[arg(long, short, value_enum, default_value_t = OutputFormat::Toml)]
		format: OutputFormat,
	},
}

/// Arguments to `monostyle check`.
///
/// The boolean fields are independent clap flags rather than a state machine, so they stay flat.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Parser)]
pub struct CheckArgs {
	/// Paths to analyze. Defaults to the current directory.
	#[arg(value_name = "PATH", default_value = ".")]
	pub paths: Vec<PathBuf>,

	/// Restrict analysis to these languages. May be repeated.
	#[arg(long, short, value_name = "LANGUAGE")]
	pub language: Vec<String>,

	/// The output format.
	#[arg(long, short, value_enum, default_value_t = OutputFormat::Text)]
	pub format: OutputFormat,

	/// Write the report to a file instead of standard output.
	#[arg(long, short, value_name = "FILE")]
	pub output: Option<PathBuf>,

	/// Only report functions scoring below this value.
	#[arg(long, value_name = "SCORE")]
	pub max_unit_score: Option<f64>,

	/// Show only the worst N functions.
	#[arg(long, value_name = "COUNT")]
	pub top: Option<usize>,

	/// Include per-function detail.
	#[arg(long)]
	pub units: bool,

	/// List every finding with its explanation.
	#[arg(long)]
	pub explain: bool,

	/// Do not read ignore files.
	#[arg(long)]
	pub no_ignore: bool,

	/// Analyze generated files as well, overriding the default exclusion.
	#[arg(long)]
	pub include_generated: bool,

	/// Exit non-zero when any category scores below this value.
	#[arg(long, value_name = "SCORE")]
	pub fail_under: Option<f64>,

	/// Suppress all output except errors.
	#[arg(long, short)]
	pub quiet: bool,
}

/// Arguments to `monostyle fix`.
///
/// The boolean fields are independent clap flags rather than a state machine, so they stay flat.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Parser)]
pub struct FixArgs {
	/// Paths to fix. Defaults to the current directory.
	#[arg(value_name = "PATH", default_value = ".")]
	pub paths: Vec<PathBuf>,

	/// Show what would change without writing anything.
	#[arg(long)]
	pub dry_run: bool,

	/// Only fix findings from this rule. May be repeated.
	#[arg(long = "rule", value_name = "RULE")]
	pub rules: Vec<String>,

	/// Do not read ignore files.
	#[arg(long)]
	pub no_ignore: bool,

	/// Analyze generated files as well, overriding the default exclusion.
	#[arg(long)]
	pub include_generated: bool,

	/// Suppress per-file output, printing only the summary.
	#[arg(long, short)]
	pub quiet: bool,
}

/// Arguments to `monostyle rules`.
#[derive(Debug, Parser)]
pub struct RulesArgs {
	/// Show details for a single rule.
	#[arg(value_name = "RULE")]
	pub rule: Option<String>,

	/// The output format.
	#[arg(long, short, value_enum, default_value_t = OutputFormat::Text)]
	pub format: OutputFormat,

	/// Only list rules that can be fixed automatically.
	#[arg(long)]
	pub fixable: bool,
}

/// How to render a report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
	/// Human-readable tables and prose.
	Text,
	/// Machine-readable JSON.
	Json,
	/// TOML, used for configuration output.
	Toml,
	/// GitHub Actions workflow commands, which render as inline annotations on the pull request diff.
	Github,
}
