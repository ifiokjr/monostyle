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
use monostyle::cli::FixArgs;
use monostyle::cli::OutputFormat;
use monostyle::cli::RulesArgs;
use monostyle::config;
use monostyle::config::ConfigExt;
use monostyle::fix;
use monostyle::report;
use monostyle::style;

/// Exit code used when the analysis completed but scores were below the threshold.
const EXIT_BELOW_THRESHOLD: u8 = 1;

/// Source used to discover which rules can produce a fix.
///
/// A rule is fixable when it emits an edit for at least one input, so a source with one problem of each
/// shape is enough to find them all.
const SAMPLE_FOR_FIXABILITY: &str = "fn a() {\n    work();\n    if x {\n        work();\n    }\n    match run() {\n        Err(_) => {}\n        Ok(v) => use_it(v),\n    }\n}\n";

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
	style::init_color(cli.color, cli.no_color);

	let options = resolve_options(&cli)?;

	match cli.command {
		Command::Check(args) => run_check(&args, options),
		Command::Fix(args) => run_fix(&args, options),
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
		options = options.apply_config(&config::load_config(path)?)?;
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
		options = options.apply_config(&config::load_config(&path)?)?;
	}

	// The generated-file exclusion is a rule default, so overriding it means turning it off before
	// the walk rather than filtering afterwards.
	if args.include_generated {
		options.rules.ignore.generated = false;
	}

	let ignore_config = options.rules.ignore.clone();
	let mut paths = Vec::new();
	let mut skipped = Vec::new();

	for path in &args.paths {
		if path.is_dir() {
			let mut found = collect_paths(path, !args.no_ignore, &ignore_config);

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

		// A caller asking for JSON still gets JSON. Emitting nothing would make a consumer's parse fail
		// for a reason that has nothing to do with the repository's code, which is the wrong failure to
		// hand back.
		if args.format == OutputFormat::Json {
			let empty = analyze_paths(&paths, &options);
			let rendered = report::render_project_json(&empty)?;

			write_output(&rendered, args.output.as_deref())?;
		}

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

/// Runs the `fix` command.
///
/// Collects every fixable finding, applies the edits back to front, and reports what changed. A dry
/// run performs every step except the write, so the counts it prints are the ones a real run would
/// produce.
fn run_fix(
	args: &FixArgs,
	options: AnalysisOptions,
) -> Result<ExitCode, Box<dyn std::error::Error>> {
	use rayon::prelude::*;

	let mut options = options;

	if args.include_generated {
		options.rules.ignore.generated = false;
	}

	let ignore_config = options.rules.ignore.clone();
	let mut paths = Vec::new();

	for path in &args.paths {
		if path.is_dir() {
			paths.append(&mut collect_paths(path, !args.no_ignore, &ignore_config));
		} else if path.is_file() {
			paths.push(path.clone());
		} else {
			return Err(format!("path does not exist: {}", path.display()).into());
		}
	}

	if paths.is_empty() {
		eprintln!("monostyle: no analyzable files found");

		return Ok(ExitCode::SUCCESS);
	}

	let outcomes: Vec<(
		fix::AppliedFixes,
		Vec<monostyle_core::Finding>,
		Vec<monostyle_core::Finding>,
	)> = paths
		.par_iter()
		.filter_map(|path| {
			let language = analysis::language_for_path(path)?;
			let source = std::fs::read_to_string(path).ok()?;
			let report = analysis::analyze_with_language(path, &source, language, &options);

			// Only fixes from the requested rules are applied, so a caller can address one class of
			// problem at a time.
			let selected: Vec<monostyle_core::Finding> = report
				.findings
				.iter()
				.filter(|finding| finding.fix.is_some())
				.filter(|finding| args.rules.is_empty() || args.rules.contains(&finding.rule))
				.cloned()
				.collect();

			if selected.is_empty() {
				return None;
			}

			let fixes: Vec<monostyle_core::Fix> = selected
				.iter()
				.filter_map(|finding| finding.fix.clone())
				.collect();

			match fix::fix_file(path, &fixes, args.dry_run) {
				Ok(outcome) if outcome.applied > 0 => {
					Some((outcome, selected, report.findings.clone()))
				}
				_ => None,
			}
		})
		.collect();

	let mut total_applied = 0;
	let mut total_conflicts = 0;
	let mut files_changed = 0;

	for (outcome, applied, all_findings) in &outcomes {
		total_applied += outcome.applied;
		total_conflicts += outcome.conflicts;

		if outcome.written {
			files_changed += 1;
		}

		if args.quiet {
			continue;
		}

		let verb = if args.dry_run { "would fix" } else { "fixed" };

		println!(
			"\n{} {} {verb} {} finding{}:",
			style::cyan(&outcome.path.display().to_string()),
			style::dim("—"),
			outcome.applied,
			if outcome.applied == 1 { "" } else { "s" }
		);

		for finding in applied {
			// Showing the suggestion beside the edit is what makes the output a record of the change
			// rather than a count. A reader can see what was wrong and why the edit is correct.
			println!(
				"  {} {}:{}  {}",
				style::dim("+"),
				outcome.path.display(),
				finding.span.start_line,
				style::magenta(&finding.rule)
			);
			println!("      {}", wrap_text(&finding.message, 74, "      "));
		}

		// Findings without a fix are listed separately, because they are the reader's work rather
		// than the tool's.
		let remaining: Vec<&monostyle_core::Finding> = all_findings
			.iter()
			.filter(|finding| finding.fix.is_none() && finding.penalty() > 0.0)
			.collect();

		if !remaining.is_empty() {
			println!(
				"\n  {} {} finding{} need a decision:",
				style::yellow("!"),
				remaining.len(),
				if remaining.len() == 1 { "" } else { "s" }
			);

			for finding in remaining.iter().take(5) {
				println!(
					"    {}:{}  {}",
					outcome.path.display(),
					finding.span.start_line,
					style::magenta(&finding.rule)
				);
				println!("        {}", wrap_text(&finding.message, 70, "        "));
				println!(
					"        {} {}",
					style::dim("->"),
					style::dim(&wrap_text(&finding.suggestion, 70, "        "))
				);
			}

			if remaining.len() > 5 {
				println!("    {} and {} more", style::dim("…"), remaining.len() - 5);
			}
		}
	}

	if args.dry_run {
		println!(
			"\n{total_applied} fixable finding{} across {} file{} (dry run; nothing written)",
			if total_applied == 1 { "" } else { "s" },
			outcomes.len(),
			if outcomes.len() == 1 { "" } else { "s" }
		);
	} else {
		println!(
			"\n{total_applied} finding{} fixed in {files_changed} file{}",
			if total_applied == 1 { "" } else { "s" },
			if files_changed == 1 { "" } else { "s" }
		);
	}

	if total_conflicts > 0 {
		eprintln!(
			"monostyle: {total_conflicts} fix{} skipped because they overlapped another edit",
			if total_conflicts == 1 { "" } else { "es" }
		);
	}

	Ok(ExitCode::SUCCESS)
}

/// Runs the `rules` command.
fn run_rules(args: &RulesArgs) -> Result<ExitCode, Box<dyn std::error::Error>> {
	let mut rules = monostyle_rules::all_rules();

	// A rule is fixable when running it can produce an edit, which is discovered by running it rather
	// than declared alongside the rule. Declaring it separately would be a second place to forget.
	if args.fixable {
		let lexed = monostyle_lexer::lex(SAMPLE_FOR_FIXABILITY, monostyle_core::Language::Rust);

		rules.retain(|rule| {
			let config = monostyle_rules::RulesConfig::default();

			(rule.run)(&lexed, &config)
				.iter()
				.any(|finding| finding.fix.is_some())
		});
	}

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
		// The nested tables are lifted out and printed under their full path. TOML's serializer emits a
		// nested table before its parent's scalar keys, so `ignore` would otherwise appear as a top-level
		// `[ignore]` header — output that cannot be pasted back into a config file, where the same values
		// belong under `[rules.ignore]`.
		let payload = toml::Value::try_from(&options.rules)
			.map(|mut value| {
				// Removing the table before serializing is what keeps it out of the top-level output.
				let nested = value
					.as_table_mut()
					.and_then(|table| table.remove("ignore"));
				let mut text = toml::to_string_pretty(&value).unwrap_or_default();

				if let Some(table) = nested {
					text.push_str("\n[rules.ignore]\n");

					if let Ok(rendered) = toml::to_string_pretty(&table) {
						text.push_str(&rendered);
					}
				}

				text
			})
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

/// Wraps `text` to `width` columns with `indent` on continuation lines.
///
/// Duplicated from the report renderer rather than shared, because the two live in different
/// crates: this one is the binary and the renderer is the library, and a one-function dependency
/// between them would be more coupling than the duplication costs.
fn wrap_text(text: &str, width: usize, indent: &str) -> String {
	let mut lines: Vec<String> = Vec::new();
	let mut current = String::new();

	for word in text.split_whitespace() {
		if !current.is_empty() && current.len() + word.len() + 1 > width {
			lines.push(std::mem::take(&mut current));
		}

		if !current.is_empty() {
			current.push(' ');
		}

		current.push_str(word);
	}

	if !current.is_empty() {
		lines.push(current);
	}

	lines.join(&format!("\n{indent}"))
}
