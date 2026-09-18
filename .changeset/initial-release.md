---
"monostyle": minor
"monostyle_core": minor
"monostyle_languages": minor
"monostyle_lexer": minor
"monostyle_markdown": minor
"monostyle_metrics": minor
"monostyle_rules": minor
"@monostyle_rs/cli": minor
"@monostyle_rs/cli_darwin_arm64": minor
"@monostyle_rs/cli_darwin_x64": minor
"@monostyle_rs/cli_linux_arm64_gnu": minor
"@monostyle_rs/cli_linux_arm64_musl": minor
"@monostyle_rs/cli_linux_x64_gnu": minor
"@monostyle_rs/cli_linux_x64_musl": minor
"@monostyle_rs/cli_win32_arm64_msvc": minor
"@monostyle_rs/cli_win32_x64_msvc": minor
"@monostyle_rs/skill": minor
---

# Initial release

monostyle scores the complexity and readability of a codebase, a file, or a function, out of 100.
Every point lost is traced to a named rule with an explanation and a suggested fix.

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
