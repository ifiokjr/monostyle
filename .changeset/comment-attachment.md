---
monostyle: major
monostyle_rules: major
monostyle_core: major
---

# Padding is symmetric, and comments stay attached

Reviewing the fixes from this release cycle on real pull requests surfaced two one-sided rules.

## New: `readability/blank-line-after-control-flow`

The existing rule puts a blank line before a branch; nothing put one after its block. A `}` crowded against the next `let` reads as though the branch were still open, so the new rule asks for the same gap on the way out — when statements follow in the same scope. It stays quiet at the end of an enclosing body, where closing punctuation asks for nothing, and leaves the blanks its sibling rules already report to them, so one missing line is never counted twice.

## New: `readability/detached-comment`

A comment above a line describes that line, so the blank the old remediation runs inserted between a comment and its `if` did not make the comment breathe — it orphaned it. Comments belong downward: padding goes above the comment, never between the comment and its code. The rule reports the detachment with a fix that re-attaches it, which repairs the mdt damage directly: the `// Position at
end of line` comment the report came from is now flagged and fixed by the tool itself.

Both rules carry fixes, are formatter-safe, and are on by default. The `--fixable` discovery sample now exercises all four fixable shapes, so `monostyle rules --fixable` lists them.

## Why major before 1.0

Two new on-by-default rules change what a run reports on unchanged code. Under the pre-1.0 semver convention monochange applies, `major` moves the minor digit: 0.2.1 to 0.3.0.
