//! A scoped function keeps both decision metrics and its actual argument-list finding.

use monostyle_core::Language;
use monostyle_lexer::lex;
use monostyle_metrics::complexity_of_lines;
use monostyle_metrics::find_units;
use monostyle_rules::RulesConfig;
use monostyle_rules::structure;

const SCOPED_FUNCTION: &str = "\
mod functions {
    pub(super) fn choose(first: bool, second: bool, third: bool, fourth: bool, fifth: bool, sixth: bool) -> bool
    {
        if first {
            return second;
        }

        if third {
            return fourth;
        }

        fifth || sixth
    }
}
";

#[test]
fn scoped_functions_keep_real_decisions_and_argument_findings() {
	const FUNCTION_START_LINE: usize = 2;
	const FUNCTION_END_LINE: usize = 13;

	let lexed = lex(SCOPED_FUNCTION, Language::Rust);
	let units = find_units(&lexed);

	assert_eq!(units.len(), 1);

	let unit = units.first().expect("the choose function is present");

	assert_eq!(unit.name, "choose");
	assert_eq!(unit.start_line, FUNCTION_START_LINE);
	assert_eq!(unit.end_line, FUNCTION_END_LINE);

	let lines = lexed
		.lines
		.get(FUNCTION_START_LINE - 1..FUNCTION_END_LINE)
		.expect("the function range is inside the fixture");
	let decisions = complexity_of_lines(lines);

	assert_eq!(decisions.breakdown.branches, 2);
	assert_eq!(decisions.breakdown.logical, 1);
	assert_eq!(decisions.total, 4);

	let findings = structure::long_parameter_list(&lexed, &RulesConfig::default());

	assert_eq!(findings.len(), 1);

	let finding = findings
		.first()
		.expect("the six-argument header is measured");

	assert_eq!(finding.span.start_line, FUNCTION_START_LINE);
	assert!(finding.message.contains("takes 6 arguments"));
}
