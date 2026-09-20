# Changelog

All notable changes to this project are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.1](https://github.com/ifiokjr/monostyle/releases/tag/v0.1.1) (2026-09-20)

### Features

#### Initial release

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

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #3](https://github.com/ifiokjr/monostyle/pull/3)

#### Read the config file, and ship releases from changesets

The configuration file was only read when `--config` was passed, so a checked-in `monostyle.toml`
had no effect. Sections, per-path floors, and every threshold were silently inert, which meant the
scores a project saw were not the ones it had asked for.

Sections are now also matched when a path carries `.` or `..` components, so `monostyle check .`
applies them rather than skipping them.

`monostyle fix` honours ignored sections instead of rewriting files that `check` deliberately skips.
It was rewriting the deliberately-bad scoring fixtures, changing the inputs the scoring tests assert
against.

One false positive is fixed. `readability/empty-handler` matched the handler keyword anywhere on a
line, so a function named `complexity_rules_catch_a_hard_function` was reported as discarding an
error it never caught. The keyword now has to be a word of its own.

Three functions were flattened after the tool named them as the largest single costs in their files:
the glob matcher, the cognitive-complexity line loop, and the overlong-line rule. Each was verified
to produce byte-identical findings before and after.

`score_color` gave the same green to `good` and `excellent`, so the band the colour exists to convey
was invisible above 75. The grade thresholds are now shared constants, and an excellent score is
distinct.

Releases are driven by changesets and published with trusted publishing where available, falling
back to a registry token for a package that has not been enrolled yet.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #3](https://github.com/ifiokjr/monostyle/pull/3)
