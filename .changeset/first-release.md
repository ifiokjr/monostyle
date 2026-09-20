---
"monostyle": major
"monostyle_core": major
"monostyle_languages": major
"monostyle_lexer": major
"monostyle_markdown": major
"monostyle_metrics": major
"monostyle_rules": major
"@monostyle_rs/cli": major
"@monostyle_rs/cli_darwin_arm64": major
"@monostyle_rs/cli_darwin_x64": major
"@monostyle_rs/cli_linux_arm64_gnu": major
"@monostyle_rs/cli_linux_arm64_musl": major
"@monostyle_rs/cli_linux_x64_gnu": major
"@monostyle_rs/cli_linux_x64_musl": major
"@monostyle_rs/cli_win32_arm64_msvc": major
"@monostyle_rs/cli_win32_x64_msvc": major
"@monostyle_rs/skill": major
---

# First release

monostyle scores the complexity and readability of a codebase, a file, or a function, out of 100.
Every point lost is traced to a named rule with an explanation and a suggested fix, so a report is a
worklist rather than a grade.

**Two scores.** Readability measures how the code looks: whether complex sections have room to
breathe, whether sequential control flow is separated, how deeply logic nests, and whether comments
explain the hard parts. Complexity measures how hard the code is to follow and to test, using
cyclomatic complexity, cognitive complexity, NPath, exit counts, and a maintainability index.

**Twenty-three languages**, described by data profiles rather than parsers, so adding one is a data
edit.

**Per-path cutoffs.** A repository is not one uniform standard, so `[[section]]` entries hold tests,
vendored code, and generated clients to their own bars.

**Actionable output.** Findings rank by the share of penalty each accounts for, name their worst
offender as `path:line`, and carry a suggested fix. `monostyle fix` applies the one edit that is
safe to automate.

**Fast.** A 2,198-file repository with 232,000 lines of code completes in about a second.
