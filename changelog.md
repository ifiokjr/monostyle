# Changelog

All notable changes to this project are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0](https://github.com/ifiokjr/monostyle/releases/tag/v0.1.0) (2026-09-20)

Grouped release for `release`.

### Breaking changes

#### First release

_Packages:_ 🔴 _monostyle_, 🔴 _monostyle_core_, 🔴 _monostyle_languages_, 🔴 _monostyle_lexer_, 🔴
_monostyle_markdown_, 🔴 _monostyle_metrics_, 🔴 _monostyle_rules_, 🔴 _@monostyle-rs/cli-, 🔴
_@monostyle-rs/cli-darwin-arm64-, 🔴 _@monostyle-rs/cli-darwin-x64-, 🔴
_@monostyle-rs/cli-linux-arm64-gnu-, 🔴 _@monostyle-rs/cli-linux-arm64-musl-, 🔴
_@monostyle-rs/cli-linux-x64-gnu-, 🔴 _@monostyle-rs/cli-linux-x64-musl-, 🔴
_@monostyle-rs/cli-win32-arm64-msvc-, 🔴 _@monostyle-rs/cli-win32-x64-msvc-, 🔴
_@monostyle-rs/skill-

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
