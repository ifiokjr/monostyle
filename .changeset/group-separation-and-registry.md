---
"monostyle": major
"monostyle_core": major
"monostyle_rules": major
---

# Make grouping measurable, and document what the rules are

## `group-separation` now measures what it claims to

The rule never looked inside a function body. A single flag was set by a type declaration and never cleared, and every statement in a function was classified as a new item, so the runs the rule exists to find went unreported while a struct's fields were counted as statements.

Splitting a long run could not clear the finding either. Each half of a long run stays over the limit, and because the finding points at the run's start, a fixer that inserted a blank line there added whitespace outside the measured run and re-triggered itself on the next pass. That loop is what stacked five and eight consecutive blank lines in downstream repositories.

- The limit is now `max-statements-per-group` (default 8) instead of a hard-coded constant, and a run only violates it when it is longer than the limit, so a run of exactly 8 is legal.
- The finding names both the run length and the limit, so a run can be split until it clears.
- Function and control-flow bodies are measured; type bodies, `switch` and `match` arms, and literals are not. A struct's fields are one item, an arm is one alternative, and a twelve-field constructor is data.
- A body closes when its own block ends, so a declaration nested inside a function no longer silences the statements after it.

## `excessive-blank-lines` is now auto-fixable

`monostyle fix` deletes the blank lines past the allowance and keeps exactly that many — one in Rust, Go, and TypeScript, two in Python and Dart. The number kept is the one the finding measured against, so the fix cannot disagree with the rule and a second run changes nothing.

Together with the blank-line insertion the tool already had, this closes the loop the stacking came from: every other layout rule asks for a gap without bounding it, and this pair supplies both the gap and the ceiling.

## Four Markdown rules can be disabled

`markdown/fence-without-language`, `markdown/fence-language-unknown`, `markdown/no-title`, and `markdown/skipped-heading-level` were documented as rules while being emitted from inside another one, so naming any of them in `disabled-rules` did nothing and `monostyle rules` never listed them. Each is a registry entry now. `markdown/heading-structure` is kept as an alias so configuration written against it still disables both heading rules.

## Checked-in bundles are skipped

A committed esbuild or webpack bundle is a dependency's code, and scoring it describes the dependency rather than the project. A bundle's banner on the first line is now recognized; a hand-written file that merely mentions a build tool in a comment is still analyzed.

## Faster on large files

The expression-depth check walked every earlier line for every line, which made analysis quadratic in file size: a twenty-thousand-line file took sixteen seconds and a forty-thousand-line file took sixty. The depth is now computed once per file and each lookup is constant. The same inputs take about a second.

## The rule table is complete and guarded

Five registered rules were missing from the documentation (`magic-number`, `short-identifier`, `empty-handler`, `commented-out-code`, and `excessive-blank-lines`), and the two new configuration keys are now documented with the per-language blank-line floor explained. Tests check both directions — every registered rule appears in the table, and every name in the table has a registry entry — so the drift that made four rules impossible to disable cannot return.

## Why this is a `major` bump before 1.0

The changeset is `major` because monochange applies the pre-1.0 semver convention, where a `major` bump on `0.y.z` moves the minor digit. For a package below 1.0 that is how you say "the numbers a user sees will change, and a threshold that was failing may now pass" without claiming an API break that has not happened. A `minor` bump under the same convention would produce `0.1.1`, which reads as a patch and would understate the change.
