# Changelog

All notable changes to this project are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.2](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.2) (2026-09-23)

### Changed

- **No package-specific changes were recorded; `monostyle_metrics` was updated to 0.3.2 as part of group `release`.**

## [0.3.1](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.1) (2026-09-23)

### Changed

- **No package-specific changes were recorded; `monostyle_metrics` was updated to 0.3.1 as part of group `release`.**

## [0.3.0](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.0) (2026-09-23)

### Changed

#### No package-specific changes were recorded; `monostyle_metrics` was updated to 0.3.0 as part of group `release`.

## [0.2.1](https://github.com/ifiokjr/monostyle/releases/tag/v0.2.1) (2026-09-22)

### Changed

- **No package-specific changes were recorded; `monostyle_metrics` was updated to 0.2.1 as part of group `release`.**

## [0.2.0](https://github.com/ifiokjr/monostyle/releases/tag/v0.2.0) (2026-09-21)

### Breaking changes

#### Fix five rule-correctness defects

Running monostyle across thirteen real repositories surfaced five defects that made a score misleading or made the advice impossible to follow. The unsatisfiable `blank-line-before-return` rule found in the same run is fixed separately.

- **A configuration section discarded the repository-wide rules.** A section's rules replaced `[rules]` instead of layering onto it, so every threshold a section did not restate silently reverted to its default for the paths that section matched. The file read correctly, which made the effect invisible.
- **The code rules ran against Markdown prose.** A Markdown file lexes as one language, so `magic-number` read a numbered list's `5.` as a numeric literal and `group-separation` read the list as an unbroken statement run. A prose list scored 86.6 where it should score 100.
- **A bodyless declaration ran to the end of the file.** A trait method ending in `;` never opened a body, so the unit stayed open until the file did, and a one-line declaration was reported as an oversized unit with a low maintainability index.
- **A rustdoc `# Errors` list was read as commented-out code.** The rule treats `::` as proof of code, and a doc comment listing variants writes them as `Error::Variant`.
- **A keyword inside an attribute was read as control flow.** `#[serde(default, rename_all = "kebab-case")]` was reported as a `default` branch missing its blank line, so the finding asked for a blank line inside an attribute list. A decorator or annotation has the same shape.

**Added:** `readability/excessive-blank-lines`. Every layout rule asks for a gap and none capped one, so following the tool's own advice could grow a gap without limit; five blank lines between two `match` arms satisfied every rule that asked for a separation. The rule takes the larger of the configured maximum and the language's own convention, so PEP 8's two blank lines before a top-level Python definition are respected.

These fixes change scores, which is why this is a major bump before 1.0 (monochange applies the pre-1.0 convention, where `major` moves the minor digit): fewer findings are reported on the same code, and a repository that was failing a threshold may now pass it.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #18](https://github.com/ifiokjr/monostyle/pull/18) · _Related issues:_ [#12](https://github.com/ifiokjr/monostyle/issues/12)

## [0.1.0](https://github.com/ifiokjr/monostyle/releases/tag/v0.1.0) (2026-09-20)

### Breaking changes

#### First release

monostyle scores the complexity and readability of a codebase, a file, or a function, out of 100. Every point lost is traced to a named rule with an explanation and a suggested fix, so a report is a worklist rather than a grade.

**Two scores.** Readability measures how the code looks: whether complex sections have room to breathe, whether sequential control flow is separated, how deeply logic nests, and whether comments explain the hard parts. Complexity measures how hard the code is to follow and to test, using cyclomatic complexity, cognitive complexity, NPath, exit counts, and a maintainability index.

**Twenty-three languages**, described by data profiles rather than parsers, so adding one is a data edit.

**Per-path cutoffs.** A repository is not one uniform standard, so `[[section]]` entries hold tests, vendored code, and generated clients to their own bars.

**Actionable output.** Findings rank by the share of penalty each accounts for, name their worst offender as `path:line`, and carry a suggested fix. `monostyle fix` applies the one edit that is safe to automate.

**Fast.** A 2,198-file repository with 232,000 lines of code completes in about a second.
