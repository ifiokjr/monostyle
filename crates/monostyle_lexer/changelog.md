# Changelog

All notable changes to this project are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.24](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.24) (2026-10-01)

### Changed

- **No package-specific changes were recorded; `monostyle_lexer` was updated to 0.3.24 as part of group `release`.**

## [0.3.23](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.23) (2026-10-01)

### Changed

- **No package-specific changes were recorded; `monostyle_lexer` was updated to 0.3.23 as part of group `release`.**

## [0.3.22](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.22) (2026-10-01)

### Changed

- **No package-specific changes were recorded; `monostyle_lexer` was updated to 0.3.22 as part of group `release`.**

## [0.3.21](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.21) (2026-10-01)

### Changed

- **No package-specific changes were recorded; `monostyle_lexer` was updated to 0.3.21 as part of group `release`.**

## [0.3.20](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.20) (2026-09-29)

### Changed

- **No package-specific changes were recorded; `monostyle_lexer` was updated to 0.3.20 as part of group `release`.**

## [0.3.19](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.19) (2026-09-28)

### Changed

- **No package-specific changes were recorded; `monostyle_lexer` was updated to 0.3.19 as part of group `release`.**

## [0.3.18](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.18) (2026-09-28)

### Changed

- **No package-specific changes were recorded; `monostyle_lexer` was updated to 0.3.18 as part of group `release`.**

## [0.3.17](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.17) (2026-09-28)

### Changed

- **No package-specific changes were recorded; `monostyle_lexer` was updated to 0.3.17 as part of group `release`.**

## [0.3.16](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.16) (2026-09-28)

### Changed

- **No package-specific changes were recorded; `monostyle_lexer` was updated to 0.3.16 as part of group `release`.**

## [0.3.15](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.15) (2026-09-28)

### Changed

- **No package-specific changes were recorded; `monostyle_lexer` was updated to 0.3.15 as part of group `release`.**

## [0.3.14](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.14) (2026-09-28)

### Changed

- **No package-specific changes were recorded; `monostyle_lexer` was updated to 0.3.14 as part of group `release`.**

## [0.3.13](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.13) (2026-09-28)

### Changed

- **No package-specific changes were recorded; `monostyle_lexer` was updated to 0.3.13 as part of group `release`.**

## [0.3.12](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.12) (2026-09-28)

### Changed

- **No package-specific changes were recorded; `monostyle_lexer` was updated to 0.3.12 as part of group `release`.**

## [0.3.11](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.11) (2026-09-28)

### Changed

- **No package-specific changes were recorded; `monostyle_lexer` was updated to 0.3.11 as part of group `release`.**

## [0.3.10](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.10) (2026-09-28)

### Changed

- **No package-specific changes were recorded; `monostyle_lexer` was updated to 0.3.10 as part of group `release`.**

## [0.3.9](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.9) (2026-09-28)

### Changed

- **No package-specific changes were recorded; `monostyle_lexer` was updated to 0.3.9 as part of group `release`.**

## [0.3.8](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.8) (2026-09-28)

### Fixes

#### Blank lines land only where formatters keep them

Three fixer defects came out of running `monostyle fix` across real repositories, each one a blank line a formatter removed or a build rejected.

`impl From<X> for Y` counted `for` as a loop decision and `Address::default()` counted `default` as a switch arm, so keywords are now validated in position: a loop `for` needs the loop after it, an arm `default` needs arm punctuation. Complexity scores stop inventing decisions from trait calls.

Method chains, boolean-operator chains, and Dart conditional imports continue a statement with no enclosing brackets, so the blank-line rules now recognise a continuation line directly and stand down. A blank line also never lands between an outer attribute and its item — that one was a hard clippy error under `-D warnings`.

`blank-line-before-control-flow` now reports without fixing. Auto-padding before branches dominated real diffs — 97% of one rollout PR's thousand added lines were blank lines — and where the break belongs is the one placement formatters argued with. The fixable set is the edits every formatter agrees with.

`--format github` caps annotations at the ten per level GitHub renders and prints one notice for the remainder, instead of flooding the log with commands the interface discards. A new formatter-agreement test tier runs the fixer over formatter-clean input and asserts rustfmt and `dart format` still accept the result.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #44](https://github.com/ifiokjr/monostyle/pull/44)

## [0.3.7](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.7) (2026-09-28)

### Changed

- **No package-specific changes were recorded; `monostyle_lexer` was updated to 0.3.7 as part of group `release`.**

## [0.3.6](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.6) (2026-09-28)

### Features

#### Fixer safety net and raw-string lexing

Three rules could edit bytes they did not understand. Each now stands on a guarantee.

`blank-line-before-control-flow` padded `} else if`, `} catch`, and bare `else` — the middle of a single decision chain — and `blank-line-after-control-flow` inserted blanks inside function-call argument lists after a `match` used as an expression. Both now read the shape before firing: a line that continues the statement above is not a new decision, and a line starting inside an open expression is not a statement.

The detached-comment fix spanned from its blank through the comment _below_ the gap, so applying it could erase the comment (the structural check caught this downstream and reverted whole files, silently withholding every other fix). The fix now spans the blank run only, and a comment below the gap starts its own block.

The fixer itself gained four guards. A fix may not land inside a literal's bytes or a comment's; a file whose scan hit an unterminated construct is skipped outright, because the lexer was guessing where its constructs end; a whitespace-only rewrite is re-lexed and compared structurally before it is written, and any mismatch throws the whole rewrite away; and a pure-CRLF file stays pure-CRLF.

The scanner learned two constructs it flattened into plain strings. C++'s `R"(…)"` and `R"delim(…)delim"` — braces, quotes, and parens inside are data, and the closer joins the custom delimiter. Java text blocks, where a `"""` block's braces and single quotes are content until the closing triple.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #35](https://github.com/ifiokjr/monostyle/pull/35)

## [0.3.5](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.5) (2026-09-27)

### Features

#### Fixer safety net and raw-string lexing

Three rules could edit bytes they did not understand. Each now stands on a guarantee.

`blank-line-before-control-flow` padded `} else if`, `} catch`, and bare `else` — the middle of a single decision chain — and `blank-line-after-control-flow` inserted blanks inside function-call argument lists after a `match` used as an expression. Both now read the shape before firing: a line that continues the statement above is not a new decision, and a line starting inside an open expression is not a statement.

The detached-comment fix spanned from its blank through the comment _below_ the gap, so applying it could erase the comment (the structural check caught this downstream and reverted whole files, silently withholding every other fix). The fix now spans the blank run only, and a comment below the gap starts its own block.

The fixer itself gained four guards. A fix may not land inside a literal's bytes or a comment's; a file whose scan hit an unterminated construct is skipped outright, because the lexer was guessing where its constructs end; a whitespace-only rewrite is re-lexed and compared structurally before it is written, and any mismatch throws the whole rewrite away; and a pure-CRLF file stays pure-CRLF.

The scanner learned two constructs it flattened into plain strings. C++'s `R"(…)"` and `R"delim(…)delim"` — braces, quotes, and parens inside are data, and the closer joins the custom delimiter. Java text blocks, where a `"""` block's braces and single quotes are content until the closing triple.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #35](https://github.com/ifiokjr/monostyle/pull/35)

## [0.3.4](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.4) (2026-09-25)

### Changed

- **No package-specific changes were recorded; `monostyle_lexer` was updated to 0.3.4 as part of group `release`.**

## [0.3.3](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.3) (2026-09-24)

### Changed

- **No package-specific changes were recorded; `monostyle_lexer` was updated to 0.3.3 as part of group `release`.**

## [0.3.2](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.2) (2026-09-23)

### Changed

- **No package-specific changes were recorded; `monostyle_lexer` was updated to 0.3.2 as part of group `release`.**

## [0.3.1](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.1) (2026-09-23)

### Fixes

#### Fix Dart raw strings swallowing the rest of the file

In Dart, `r'...'` is a raw string: the backslash is an ordinary character, not an escape. The scanner honored `\'` inside `r'...'` anyway, so a raw string holding a backslash never closed. Every following line was classified as blank string content, with two consequences: the layout rules went blind for the rest of the file, and the blank-line fixer deleted the "blank" lines — real code — as formatting. On a downstream repository the fixer removed seven lines of live Dart from a build script this way.

Raw prefixed forms no longer honor escapes, and a backslash at the end of a raw line no longer continues the literal. The file now lexes as code, and the fixer leaves it alone.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #26](https://github.com/ifiokjr/monostyle/pull/26)

## [0.3.0](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.0) (2026-09-23)

### Changed

#### No package-specific changes were recorded; `monostyle_lexer` was updated to 0.3.0 as part of group `release`.

## [0.2.1](https://github.com/ifiokjr/monostyle/releases/tag/v0.2.1) (2026-09-22)

### Changed

- **No package-specific changes were recorded; `monostyle_lexer` was updated to 0.2.1 as part of group `release`.**

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
