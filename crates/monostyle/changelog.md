# Changelog

All notable changes to this project are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.3](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.3) (2026-09-24)

### Features

#### Inline annotations on GitHub pull requests

`monostyle check . --format github` now emits every finding as a GitHub Actions workflow command. GitHub renders those as inline annotations on the pull request diff — the same surface an ESLint or Clippy annotation uses — so a reviewer sees where the improvements are without opening the full report.

Severity maps to the annotation level: `Minor` findings are warnings and `Major` and `Critical` findings are errors. The `%`, `\r`, and `\n` characters in the message are encoded, so a message containing a percent sign or a newline cannot truncate the annotation.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #30](https://github.com/ifiokjr/monostyle/pull/30)

## [0.3.2](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.2) (2026-09-23)

### Features

#### `blank-line-before-return` is now auto-fixable

The return rule asked for a blank line but carried no fix, which made the largest single finding class in every repository a manual chore. The fix inserts the blank above the return — the same edit, and the same formatter-safety argument, as the blank-line insertion the before-control-flow rule has always carried: Rustfmt, Prettier, Black, and `dart format` all preserve blank lines between statements and none of them add one.

`monostyle rules --fixable` now lists five fixable rules, and the discovery sample fires on every one of them. The idempotence property holds: a second `monostyle fix` run changes nothing.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #28](https://github.com/ifiokjr/monostyle/pull/28)

## [0.3.1](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.1) (2026-09-23)

### Fixes

#### Fix Dart raw strings swallowing the rest of the file

In Dart, `r'...'` is a raw string: the backslash is an ordinary character, not an escape. The scanner honored `\'` inside `r'...'` anyway, so a raw string holding a backslash never closed. Every following line was classified as blank string content, with two consequences: the layout rules went blind for the rest of the file, and the blank-line fixer deleted the "blank" lines — real code — as formatting. On a downstream repository the fixer removed seven lines of live Dart from a build script this way.

Raw prefixed forms no longer honor escapes, and a backslash at the end of a raw line no longer continues the literal. The file now lexes as code, and the fixer leaves it alone.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #26](https://github.com/ifiokjr/monostyle/pull/26)

## [0.3.0](https://github.com/ifiokjr/monostyle/releases/tag/v0.3.0) (2026-09-23)

### Breaking changes

#### Padding is symmetric, and comments stay attached

Reviewing the fixes from this release cycle on real pull requests surfaced two one-sided rules.

##### New: `readability/blank-line-after-control-flow`

The existing rule puts a blank line before a branch; nothing put one after its block. A `}` crowded against the next `let` reads as though the branch were still open, so the new rule asks for the same gap on the way out — when statements follow in the same scope. It stays quiet at the end of an enclosing body, where closing punctuation asks for nothing, and leaves the blanks its sibling rules already report to them, so one missing line is never counted twice.

##### New: `readability/detached-comment`

A comment above a line describes that line, so the blank the old remediation runs inserted between a comment and its `if` did not make the comment breathe — it orphaned it. Comments belong downward: padding goes above the comment, never between the comment and its code. The rule reports the detachment with a fix that re-attaches it, which repairs the mdt damage directly: the `// Position at
end of line` comment the report came from is now flagged and fixed by the tool itself.

Both rules carry fixes, are formatter-safe, and are on by default. The `--fixable` discovery sample now exercises all four fixable shapes, so `monostyle rules --fixable` lists them.

##### Why major before 1.0

Two new on-by-default rules change what a run reports on unchanged code. Under the pre-1.0 semver convention monochange applies, `major` moves the minor digit: 0.2.1 to 0.3.0.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #24](https://github.com/ifiokjr/monostyle/pull/24)

## [0.2.1](https://github.com/ifiokjr/monostyle/releases/tag/v0.2.1) (2026-09-22)

### Fixes

#### List every fixable rule in `monostyle rules --fixable`

The listing worked by running each rule over a hard-coded sample and keeping the ones that produced an edit. The sample contained a crowded control-flow statement but no stacked blank lines, so the blank-line collapse added in this release never appeared in the list: `monostyle rules --fixable` reported one fixable rule where there were two.

The sample now holds one problem of every fixable shape. The approach itself — discovering fixability by running the rule rather than declaring it — is unchanged, and is the right one: a declared flag is a second place to forget, as this bug demonstrates.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #22](https://github.com/ifiokjr/monostyle/pull/22)

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

#### Make grouping measurable, and document what the rules are

##### `group-separation` now measures what it claims to

The rule never looked inside a function body. A single flag was set by a type declaration and never cleared, and every statement in a function was classified as a new item, so the runs the rule exists to find went unreported while a struct's fields were counted as statements.

Splitting a long run could not clear the finding either. Each half of a long run stays over the limit, and because the finding points at the run's start, a fixer that inserted a blank line there added whitespace outside the measured run and re-triggered itself on the next pass. That loop is what stacked five and eight consecutive blank lines in downstream repositories.

- The limit is now `max-statements-per-group` (default 8) instead of a hard-coded constant, and a run only violates it when it is longer than the limit, so a run of exactly 8 is legal.
- The finding names both the run length and the limit, so a run can be split until it clears.
- Function and control-flow bodies are measured; type bodies, `switch` and `match` arms, and literals are not. A struct's fields are one item, an arm is one alternative, and a twelve-field constructor is data.
- A body closes when its own block ends, so a declaration nested inside a function no longer silences the statements after it.

##### `excessive-blank-lines` is now auto-fixable

`monostyle fix` deletes the blank lines past the allowance and keeps exactly that many — one in Rust, Go, and TypeScript, two in Python and Dart. The number kept is the one the finding measured against, so the fix cannot disagree with the rule and a second run changes nothing.

Together with the blank-line insertion the tool already had, this closes the loop the stacking came from: every other layout rule asks for a gap without bounding it, and this pair supplies both the gap and the ceiling.

##### Four Markdown rules can be disabled

`markdown/fence-without-language`, `markdown/fence-language-unknown`, `markdown/no-title`, and `markdown/skipped-heading-level` were documented as rules while being emitted from inside another one, so naming any of them in `disabled-rules` did nothing and `monostyle rules` never listed them. Each is a registry entry now. `markdown/heading-structure` is kept as an alias so configuration written against it still disables both heading rules.

##### Checked-in bundles are skipped

A committed esbuild or webpack bundle is a dependency's code, and scoring it describes the dependency rather than the project. A bundle's banner on the first line is now recognized; a hand-written file that merely mentions a build tool in a comment is still analyzed.

##### Faster on large files

The expression-depth check walked every earlier line for every line, which made analysis quadratic in file size: a twenty-thousand-line file took sixteen seconds and a forty-thousand-line file took sixty. The depth is now computed once per file and each lookup is constant. The same inputs take about a second.

##### The rule table is complete and guarded

Five registered rules were missing from the documentation (`magic-number`, `short-identifier`, `empty-handler`, `commented-out-code`, and `excessive-blank-lines`), and the two new configuration keys are now documented with the per-language blank-line floor explained. Tests check both directions — every registered rule appears in the table, and every name in the table has a registry entry — so the drift that made four rules impossible to disable cannot return.

##### Why this is a `major` bump before 1.0

The changeset is `major` because monochange applies the pre-1.0 semver convention, where a `major` bump on `0.y.z` moves the minor digit. For a package below 1.0 that is how you say "the numbers a user sees will change, and a threshold that was failing may now pass" without claiming an API break that has not happened. A `minor` bump under the same convention would produce `0.1.1`, which reads as a patch and would understate the change.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #19](https://github.com/ifiokjr/monostyle/pull/19)

## [0.1.0](https://github.com/ifiokjr/monostyle/releases/tag/v0.1.0) (2026-09-20)

### Breaking changes

#### First release

monostyle scores the complexity and readability of a codebase, a file, or a function, out of 100. Every point lost is traced to a named rule with an explanation and a suggested fix, so a report is a worklist rather than a grade.

**Two scores.** Readability measures how the code looks: whether complex sections have room to breathe, whether sequential control flow is separated, how deeply logic nests, and whether comments explain the hard parts. Complexity measures how hard the code is to follow and to test, using cyclomatic complexity, cognitive complexity, NPath, exit counts, and a maintainability index.

**Twenty-three languages**, described by data profiles rather than parsers, so adding one is a data edit.

**Per-path cutoffs.** A repository is not one uniform standard, so `[[section]]` entries hold tests, vendored code, and generated clients to their own bars.

**Actionable output.** Findings rank by the share of penalty each accounts for, name their worst offender as `path:line`, and carry a suggested fix. `monostyle fix` applies the one edit that is safe to automate.

**Fast.** A 2,198-file repository with 232,000 lines of code completes in about a second.
