//! The monostyle command-line tool.

use std::path::Path;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use monostyle::analysis;
use monostyle::analysis::AnalysisOptions;
use monostyle::analysis::analyze_paths;
use monostyle::analysis::collect_paths;
use monostyle::cli::CheckArgs;
use monostyle::cli::Cli;
use monostyle::cli::Command;
use monostyle::cli::OutputFormat;
use monostyle::cli::RulesArgs;
use monostyle::config;
use monostyle::report;

/// Exit code used when the analysis completed but scores were below the threshold.
const EXIT_BELOW_THRESHOLD: u8 = 1;

/// Exit code used when the run itself failed.
const EXIT_FAILURE: u8 = 2;

fn main() -> ExitCode {
	let cli = Cli::parse();

	match run(cli) {
		Ok(code) => code,
		Err(error) => {
			eprintln!("monostyle: {error}");

			ExitCode::from(EXIT_FAILURE)
		}
	}
}

/// Runs the requested command.
fn run(cli: Cli) -> Result<ExitCode, Box<dyn std::error::Error>> {
	monostyle::style::init_color(cli.color, cli.no_color);

	let options = resolve_options(&cli)?;

	match cli.command {
		Command::Check(args) => run_check(&args, options),
		Command::Rules(args) => run_rules(&args),
		Command::Config { format } => {
			print_config(&options, format);

			Ok(ExitCode::SUCCESS)
		}
	}
}

/// Builds analysis options from the configuration file and command-line flags.
fn resolve_options(cli: &Cli) -> Result<AnalysisOptions, Box<dyn std::error::Error>> {
	let mut options = AnalysisOptions::default();

	if let Some(path) = &cli.config {
		options = config::load_config(path)?.apply(&options);
	}

	options
		.rules
		.disabled_rules
		.extend(cli.disable.iter().cloned());
	options = config::apply_tolerance(options, cli.strict, cli.lenient);

	Ok(options)
}

/// Runs the `check` command.
fn run_check(
	args: &CheckArgs,
	mut options: AnalysisOptions,
) -> Result<ExitCode, Box<dyn std::error::Error>> {
	// A config file beside the analyzed paths is picked up automatically, so a project does
	// not have to pass `--config` on every invocation.
	if options.rules == monostyle_rules::RulesConfig::default()
		&& let Some(path) = args
			.paths
			.first()
			.and_then(|path| config::find_config(path))
	{
		options = config::load_config(&path)?.apply(&options);
	}

	let mut paths = Vec::new();
	let mut skipped = Vec::new();

	for path in &args.paths {
		if path.is_dir() {
			let mut found = collect_paths(path, !args.no_ignore);

			paths.append(&mut found);
		} else if path.is_file() {
			if analysis::language_for_path(path).is_some() {
				paths.push(path.clone());
			} else {
				skipped.push(path.clone());
			}
		} else {
			return Err(format!("path does not exist: {}", path.display()).into());
		}
	}

	// An explicit language filter narrows the set after collection, so directory walks stay
	// cheap and the filter reads as a postcondition rather than a walk parameter.
	if !args.language.is_empty() {
		let wanted: Vec<monostyle_core::Language> = args
			.language
			.iter()
			.filter_map(|name| monostyle_core::Language::from_name(name))
			.collect();

		paths.retain(|path| {
			analysis::language_for_path(path).is_some_and(|language| wanted.contains(&language))
		});
	}

	if paths.is_empty() {
		eprintln!("monostyle: no analyzable files found");

		return Ok(ExitCode::SUCCESS);
	}

	let mut report = analyze_paths(&paths, &options);
	report.skipped.extend(skipped);
	apply_unit_filters(&mut report, args);

	let rendered = match args.format {
		OutputFormat::Json => report::render_project_json(&report)?,
		OutputFormat::Text | OutputFormat::Toml => {
			report::render_project(&report, args.explain, args.units)
		}
	};

	write_output(&rendered, args.output.as_deref())?;

	if !args.quiet {
		eprintln!(
			"\nmonostyle: readability {:.1}, complexity {:.1}, overall {:.1}",
			report.readability.value,
			report.complexity.value,
			report.overall()
		);
	}

	Ok(exit_code_for(&report, args))
}

/// Removes files and units excluded by the reporting filters.
fn apply_unit_filters(report: &mut analysis::ProjectReport, args: &CheckArgs) {
	if let Some(threshold) = args.max_unit_score {
		for file in &mut report.files {
			file.units.retain(|unit| unit.score() < threshold);
		}
	}

	if let Some(limit) = args.top {
		for file in &mut report.files {
			file.units.sort_by(|left, right| {
				left.score()
					.partial_cmp(&right.score())
					.unwrap_or(std::cmp::Ordering::Equal)
			});

			file.units.truncate(limit);
		}
	}
}

/// Writes the rendered report to a file or standard output.
fn write_output(rendered: &str, output: Option<&Path>) -> Result<(), Box<dyn std::error::Error>> {
	match output {
		Some(path) => {
			std::fs::write(path, rendered)?;
		}
		None => {
			print!("{rendered}");
		}
	}

	Ok(())
}

/// Decides the exit code from the scores.
fn exit_code_for(report: &analysis::ProjectReport, args: &CheckArgs) -> ExitCode {
	let Some(threshold) = args.fail_under else {
		return ExitCode::SUCCESS;
	};

	let below = report.readability.value < threshold || report.complexity.value < threshold;

	if below && !args.quiet {
		eprintln!(
			"monostyle: scores are below the required threshold of {threshold:.1} \
			 (readability {:.1}, complexity {:.1})",
			report.readability.value, report.complexity.value
		);
	}

	if below {
		ExitCode::from(EXIT_BELOW_THRESHOLD)
	} else {
		ExitCode::SUCCESS
	}
}

/// Runs the `rules` command.
fn run_rules(args: &RulesArgs) -> Result<ExitCode, Box<dyn std::error::Error>> {
	let rules = monostyle_rules::all_rules();

	if let Some(name) = &args.rule {
		let Some(rule) = rules.iter().find(|rule| rule.name == name) else {
			eprintln!("monostyle: unknown rule `{name}`");

			return Ok(ExitCode::from(EXIT_FAILURE));
		};

		match args.format {
			OutputFormat::Json => {
				println!(
					"{}",
					serde_json::json!({ "name": rule.name, "description": rule.description })
				);
			}
			_ => {
				println!("{}\n\n{}", rule.name, rule.description);
			}
		}

		return Ok(ExitCode::SUCCESS);
	}

	if args.format == OutputFormat::Json {
		let listed: Vec<serde_json::Value> = rules
			.iter()
			.map(|rule| serde_json::json!({ "name": rule.name, "description": rule.description }))
			.collect();

		println!("{}", serde_json::to_string_pretty(&listed)?);
	} else {
		println!("{} rules\n", rules.len());

		for rule in &rules {
			println!("  {}\n      {}", rule.name, rule.description);
		}
	}

	Ok(ExitCode::SUCCESS)
}

/// Prints the effective configuration.
fn print_config(options: &AnalysisOptions, format: OutputFormat) {
	if format == OutputFormat::Json {
		let payload = serde_json::json!({
			"scoring": {
				"half-life": options.scoring.half_life,
				"min-normalization-lines": options.scoring.min_normalization_lines,
			},
			"rules": serde_json::to_value(&options.rules).unwrap_or(serde_json::Value::Null),
		});

		println!(
			"{}",
			serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string())
		);
	} else {
		let payload = toml::Value::try_from(&options.rules)
			.map(|value| toml::to_string_pretty(&value).unwrap_or_default())
			.unwrap_or_default();

		println!(
			"[scoring]\nhalf-life = {}\nmin-normalization-lines = {}\n\n[rules]\n{payload}",
			options.scoring.half_life, options.scoring.min_normalization_lines
		);
	}
}

/// Returns the paths exactly as given, for callers that need them.
#[must_use]
pub fn as_paths(values: &[String]) -> Vec<PathBuf> {
	values.iter().map(PathBuf::from).collect()
}
