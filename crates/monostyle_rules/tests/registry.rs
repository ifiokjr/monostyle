//! Tests for the rule set as a whole.
//!
//! Individual rules have their own suites. These tests cover the contracts the registry must satisfy:
//! every rule is reachable, disableable, and explained, and running the whole set is deterministic.

use monostyle_core::Language;
use monostyle_lexer::lex;
use monostyle_rules::RulesConfig;
use monostyle_rules::all_rules;
use monostyle_rules::run_rules;

/// A source file with a deliberate problem for each class of rule.
const KITCHEN_SINK: &str = r"
// TODO: this is a directive, not a reason
pub fn process(input: &Input, mode: Mode, flags: Flags, cache: &Cache) -> Result<Output, Error> {
    if input.is_empty() {
        return Err(Error::Empty);
    }
    if !input.is_valid_format() {
        return Err(Error::InvalidFormat);
    }
    if input.len() > 86400 {
        return Err(Error::TooLong);
    }
    let ab = input.normalized();
    let cd = ab.expand();
    let ef = cd.hash();
    let gh = cache.lookup(&ef);
    if mode == Mode::Strict {
        if flags.contains(Flags::VERIFY) {
            if !gh.is_empty() {
                if gh.verify().is_ok() {
                    for entry in gh.entries() {
                        if entry.is_stale() {
                            if entry.can_refresh() {
                                entry.refresh();
                            }
                        }
                    }
                }
            }
        }
    }
    // fn old_implementation() {
    //     let stale = compute_something();
    //     process_it(stale);
    //     return stale;
    // }
    match run_it(ab, cd, ef, gh) {
        Err(_) => {
        }
        Ok(value) => use_it(value),
    }
    // Increment the retry counter
    let x = compute(ab, cd, ef, gh, input.scale, input.offset, input.limit, input.target);
    Ok(x)
}
";

#[test]
fn every_rule_has_a_unique_name() {
	let mut names: Vec<&str> = all_rules().iter().map(|rule| rule.name).collect();

	names.sort_unstable();
	let count = names.len();
	names.dedup();

	assert_eq!(names.len(), count, "two rules share a name");
}

#[test]
fn every_rule_has_a_description() {
	for rule in all_rules() {
		assert!(
			!rule.description.is_empty(),
			"{} has no description",
			rule.name
		);
		assert!(
			rule.description.ends_with('.'),
			"{}'s description should be a sentence",
			rule.name
		);
	}
}

#[test]
fn every_rule_name_is_namespaced() {
	// The namespace is what lets a reader tell a layout problem from a complexity one without reading
	// the description.
	for rule in all_rules() {
		assert!(
			rule.name.contains('/'),
			"{} should be namespaced as `category/name`",
			rule.name
		);
	}
}

#[test]
fn every_rule_is_listed_under_a_known_category() {
	let known = ["readability", "complexity", "markdown"];

	for rule in all_rules() {
		let namespace = rule.name.split('/').next().unwrap_or_default();

		assert!(
			known.contains(&namespace),
			"{} has an unknown namespace",
			rule.name
		);
	}
}

#[test]
fn the_rule_set_covers_each_category() {
	let names: Vec<&str> = all_rules().iter().map(|rule| rule.name).collect();

	for expected in [
		"readability/blank-line-before-control-flow",
		"readability/deep-nesting",
		"readability/overlong-line",
		"readability/magic-number",
		"readability/comment-required-on-complex-unit",
		"complexity/cyclomatic-per-unit",
		"complexity/cognitive-per-unit",
		"complexity/npath-per-unit",
		"complexity/low-maintainability",
		"markdown/fence-readability",
	] {
		assert!(
			names.contains(&expected),
			"`{expected}` is missing from the registry"
		);
	}
}

#[test]
fn disabling_a_rule_removes_its_findings() {
	let lexed = lex(KITCHEN_SINK, Language::Rust);

	let enabled = RulesConfig::default();
	let all = run_rules(&lexed, &enabled);

	let rule = "readability/blank-line-before-control-flow";

	assert!(
		all.iter().any(|finding| finding.rule == rule),
		"the rule should fire on this source before it is disabled"
	);

	let disabled = RulesConfig {
		disabled_rules: vec![rule.to_string()],
		..RulesConfig::default()
	};

	let filtered = run_rules(&lexed, &disabled);

	assert!(
		filtered.iter().all(|finding| finding.rule != rule),
		"a disabled rule should produce nothing"
	);
	assert!(
		filtered.len() < all.len(),
		"disabling one rule should reduce the finding count"
	);
}

#[test]
fn a_kitchen_sink_source_exercises_many_rules() {
	// A source with one problem per class should trip most of the registry; if it stops doing so, a
	// rule has silently stopped firing.
	let lexed = lex(KITCHEN_SINK, Language::Rust);
	let findings = run_rules(&lexed, &RulesConfig::default());

	let mut rules: Vec<&str> = findings
		.iter()
		.map(|finding| finding.rule.as_str())
		.collect();
	rules.sort_unstable();
	rules.dedup();

	assert!(
		rules.len() >= 8,
		"expected at least eight rules to fire, got {}: {rules:?}",
		rules.len()
	);
}

#[test]
fn running_the_rules_twice_gives_the_same_answer() {
	let lexed = lex(KITCHEN_SINK, Language::Rust);

	let first = run_rules(&lexed, &RulesConfig::default());
	let second = run_rules(&lexed, &RulesConfig::default());

	assert_eq!(first.len(), second.len());

	for (left, right) in first.iter().zip(second.iter()) {
		assert_eq!(left.rule, right.rule);
		assert_eq!(left.message, right.message);
		assert_eq!(left.span.start_line, right.span.start_line);
	}
}

#[test]
fn every_finding_from_the_whole_set_is_explained() {
	let lexed = lex(KITCHEN_SINK, Language::Rust);

	for finding in run_rules(&lexed, &RulesConfig::default()) {
		assert!(
			!finding.message.is_empty(),
			"{} has no message",
			finding.rule
		);
		assert_ne!(
			finding.message, "rule violated",
			"{} used the default message",
			finding.rule
		);
		assert!(
			!finding.suggestion.is_empty(),
			"{} has no suggestion",
			finding.rule
		);
	}
}

#[test]
fn every_finding_carries_a_category_matching_its_namespace() {
	// When these disagree, a finding is scored against the wrong score, which is invisible in a report
	// and changes the number a reader trusts.
	for rule in all_rules() {
		let lexed = lex(KITCHEN_SINK, Language::Rust);
		let findings = (rule.run)(&lexed, &RulesConfig::default());

		let namespace = rule.name.split('/').next().unwrap_or_default();

		for finding in findings {
			let expected = match namespace {
				"complexity" => monostyle_core::Category::Complexity,
				_ => monostyle_core::Category::Readability,
			};

			assert_eq!(
				finding.category, expected,
				"{} reports category {:?} under the `{namespace}` namespace",
				rule.name, finding.category
			);
			assert_eq!(
				finding.rule, rule.name,
				"a rule emitted a finding under another name"
			);
		}
	}
}

#[test]
fn every_rule_survives_an_empty_file() {
	for rule in all_rules() {
		for language in [Language::Rust, Language::Python, Language::Markdown] {
			let lexed = lex("", language);

			// The assertion is that the rule terminates without panicking on no input, which is the
			// case a malformed file produces.
			let _ = (rule.run)(&lexed, &RulesConfig::default());
		}
	}
}

#[test]
fn every_rule_survives_one_line_of_garbage() {
	for rule in all_rules() {
		let lexed = lex("}}}]]](((:::;;;", Language::Rust);

		let _ = (rule.run)(&lexed, &RulesConfig::default());
	}
}

#[test]
fn only_one_rule_produces_a_fix() {
	// Fixes are restricted to the blank-line insertion because it is the only edit guaranteed to
	// survive a formatter. A second fixable rule would need the same argument made for it.
	let lexed = lex(KITCHEN_SINK, Language::Rust);
	let findings = run_rules(&lexed, &RulesConfig::default());

	let fixable: Vec<&str> = findings
		.iter()
		.filter(|finding| finding.fix.is_some())
		.map(|finding| finding.rule.as_str())
		.collect();

	let mut unique = fixable.clone();
	unique.sort_unstable();
	unique.dedup();

	assert_eq!(
		unique,
		vec!["readability/blank-line-before-control-flow"],
		"only the blank-line rule should be auto-fixable"
	);
}

#[test]
fn the_fixable_rule_produces_a_fix_for_every_finding() {
	// A finding from the fixable rule with no fix attached would mean the rule attached one
	// conditionally, which makes `monostyle fix` unreliable.
	let lexed = lex(KITCHEN_SINK, Language::Rust);
	let findings = run_rules(&lexed, &RulesConfig::default());

	for finding in findings
		.iter()
		.filter(|finding| finding.rule == "readability/blank-line-before-control-flow")
	{
		let fix = finding
			.fix
			.as_ref()
			.expect("a fixable finding must carry a fix");

		assert_eq!(fix.replacement, "\n", "the fix should insert a line break");
		assert_eq!(
			fix.span.start_byte, finding.span.start_byte,
			"the fix should target the finding"
		);
	}
}
