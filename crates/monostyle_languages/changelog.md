# Changelog

All notable changes to this project are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.24](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.24) (2026-10-01)

### Changed

- **No package-specific changes were recorded; `monostyle_languages` was updated to 0.3.24 as part of group `release`.**

## [0.3.23](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.23) (2026-10-01)

### Changed

- **No package-specific changes were recorded; `monostyle_languages` was updated to 0.3.23 as part of group `release`.**

## [0.3.22](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.22) (2026-10-01)

### Changed

- **No package-specific changes were recorded; `monostyle_languages` was updated to 0.3.22 as part of group `release`.**

## [0.3.21](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.21) (2026-10-01)

### Changed

- **No package-specific changes were recorded; `monostyle_languages` was updated to 0.3.21 as part of group `release`.**

## [0.3.20](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.20) (2026-09-29)

### Fixes

#### Measure what the author chose, not the formatter

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #69](https://github.com/ifiokjr/monostyle/pull/69)

Rolling monostyle across ten real repositories produced more than 40,000 findings, and the largest clusters were the tool being wrong rather than the code: the formatter's own layout punished as nesting, valid fence tags called misspellings, and names that belong to the language reported as naming problems. Each is fixed here.

**Indentation and line width.** A call's arguments, a chained call, an operator continuation, a closing bracket, and a match arm's label and body all sit deeper than the statement they belong to because the formatter put them there. Only a line that opens something is measured now, and a new `tab-width` key measures columns the way the project's formatter draws them — dprint's common TypeScript setting writes two-column tabs, and charging four made every multi-level line report as both over-indented and over-long.

**Magic numbers.** A literal is named when the thing beside it names it: an enum variant's discriminant, a named field's value, a type's own parameter (`[u8; 32]`, `String<64>`), and a row of a data table. Those four covered the overwhelming majority of the 6,380 findings lootbox reported and the 8,233 solana_kit reported, none of which any author could act on.

**Short identifiers.** `Ok`, `Err`, `u8`, `i32`, `V1`, and the coordinate conventions (`dx`, `dy`, `rx`, `ry`) are vocabulary rather than choices, and a numeric literal is never an identifier.

**Markdown.** `toml`, `yaml`, `json`, `text` and thirty more are valid fence tags that monostyle simply does not analyze; only a tag naming no language at all is reported now, which is what makes the rule able to catch a real typo. A `title:` in frontmatter counts as the document's title, so a docs site's pages are no longer reported for having none.

**Two rules corrected.** A same-line Swift `} catch { result(error) }` was reported as an empty handler because the non-empty body fell through to the next line — the enclosing brace. And `case`/`when`/`default` left the nesting keyword lists: an arm label is a branch of a decision already counted, and its body sits at the arm's level rather than a level deeper.

## [0.3.19](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.19) (2026-09-28)

### Changed

- **No package-specific changes were recorded; `monostyle_languages` was updated to 0.3.19 as part of group `release`.**

## [0.3.18](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.18) (2026-09-28)

### Changed

- **No package-specific changes were recorded; `monostyle_languages` was updated to 0.3.18 as part of group `release`.**

## [0.3.17](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.17) (2026-09-28)

### Changed

- **No package-specific changes were recorded; `monostyle_languages` was updated to 0.3.17 as part of group `release`.**

## [0.3.16](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.16) (2026-09-28)

### Changed

- **No package-specific changes were recorded; `monostyle_languages` was updated to 0.3.16 as part of group `release`.**

## [0.3.15](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.15) (2026-09-28)

### Changed

- **No package-specific changes were recorded; `monostyle_languages` was updated to 0.3.15 as part of group `release`.**

## [0.3.14](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.14) (2026-09-28)

### Changed

- **No package-specific changes were recorded; `monostyle_languages` was updated to 0.3.14 as part of group `release`.**

## [0.3.13](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.13) (2026-09-28)

### Fixes

- **Rust plain strings may span lines.** A Rust `"…"` can legally contain a bare newline, but the plain quote was registered as a single-line literal, so an opening quote with no closer on its line was treated as a mis-read and the string's first line was scanned as code — the `return` inside an embedded JavaScript mock fired the before-return rule, and the inserted blank landed inside the literal where rustfmt removed it. The plain quote is now a multiline rule; the char-literal quote stays single-line, where an unterminated one really is a mis-read. _Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #54](https://github.com/ifiokjr/monostyle/pull/54)

## [0.3.12](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.12) (2026-09-28)

### Changed

- **No package-specific changes were recorded; `monostyle_languages` was updated to 0.3.12 as part of group `release`.**

## [0.3.11](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.11) (2026-09-28)

### Changed

- **No package-specific changes were recorded; `monostyle_languages` was updated to 0.3.11 as part of group `release`.**

## [0.3.10](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.10) (2026-09-28)

### Changed

- **No package-specific changes were recorded; `monostyle_languages` was updated to 0.3.10 as part of group `release`.**

## [0.3.9](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.9) (2026-09-28)

### Changed

- **No package-specific changes were recorded; `monostyle_languages` was updated to 0.3.9 as part of group `release`.**

## [0.3.8](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.8) (2026-09-28)

### Changed

- **No package-specific changes were recorded; `monostyle_languages` was updated to 0.3.8 as part of group `release`.**

## [0.3.7](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.7) (2026-09-28)

### Changed

- **No package-specific changes were recorded; `monostyle_languages` was updated to 0.3.7 as part of group `release`.**

## [0.3.6](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.6) (2026-09-28)

### Changed

- **No package-specific changes were recorded; `monostyle_languages` was updated to 0.3.6 as part of group `release`.**

## [0.3.5](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.5) (2026-09-27)

### Changed

- **No package-specific changes were recorded; `monostyle_languages` was updated to 0.3.5 as part of group `release`.**

## [0.3.4](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.4) (2026-09-25)

### Changed

- **No package-specific changes were recorded; `monostyle_languages` was updated to 0.3.4 as part of group `release`.**

## [0.3.3](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.3) (2026-09-24)

### Changed

- **No package-specific changes were recorded; `monostyle_languages` was updated to 0.3.3 as part of group `release`.**

## [0.3.2](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.2) (2026-09-23)

### Changed

- **No package-specific changes were recorded; `monostyle_languages` was updated to 0.3.2 as part of group `release`.**

## [0.3.1](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.1) (2026-09-23)

### Changed

- **No package-specific changes were recorded; `monostyle_languages` was updated to 0.3.1 as part of group `release`.**

## [0.3.0](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.0) (2026-09-23)

### Changed

#### No package-specific changes were recorded; `monostyle_languages` was updated to 0.3.0 as part of group `release`.

## [0.2.1](https://github.com/ifiokjr/monostyle/releases/tag/v0.2.1) (2026-09-22)

### Changed

- **No package-specific changes were recorded; `monostyle_languages` was updated to 0.2.1 as part of group `release`.**

## [0.2.0](https://github.com/ifiokjr/monostyle/releases/tag/v0.2.0) (2026-09-21)

### Changed

#### No package-specific changes were recorded; `monostyle_languages` was updated to 0.2.0 as part of group `release`.

## [0.1.0](https://github.com/ifiokjr/monostyle/releases/tag/v0.1.0) (2026-09-20)

### Breaking changes

#### First release

monostyle scores the complexity and readability of a codebase, a file, or a function, out of 100. Every point lost is traced to a named rule with an explanation and a suggested fix, so a report is a worklist rather than a grade.

**Two scores.** Readability measures how the code looks: whether complex sections have room to breathe, whether sequential control flow is separated, how deeply logic nests, and whether comments explain the hard parts. Complexity measures how hard the code is to follow and to test, using cyclomatic complexity, cognitive complexity, NPath, exit counts, and a maintainability index.

**Twenty-three languages**, described by data profiles rather than parsers, so adding one is a data edit.

**Per-path cutoffs.** A repository is not one uniform standard, so `[[section]]` entries hold tests, vendored code, and generated clients to their own bars.

**Actionable output.** Findings rank by the share of penalty each accounts for, name their worst offender as `path:line`, and carry a suggested fix. `monostyle fix` applies the one edit that is safe to automate.

**Fast.** A 2,198-file repository with 232,000 lines of code completes in about a second.
