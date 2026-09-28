---
monostyle: minor
monostyle_lexer: patch
monostyle_rules: minor
---

# Blank lines land only where formatters keep them

Three fixer defects came out of running `monostyle fix` across real repositories, each one a blank line a formatter removed or a build rejected.

`impl From<X> for Y` counted `for` as a loop decision and `Address::default()` counted `default` as a switch arm, so keywords are now validated in position: a loop `for` needs the loop after it, an arm `default` needs arm punctuation. Complexity scores stop inventing decisions from trait calls.

Method chains, boolean-operator chains, and Dart conditional imports continue a statement with no enclosing brackets, so the blank-line rules now recognise a continuation line directly and stand down. A blank line also never lands between an outer attribute and its item — that one was a hard clippy error under `-D warnings`.

`blank-line-before-control-flow` now reports without fixing. Auto-padding before branches dominated real diffs — 97% of one rollout PR's thousand added lines were blank lines — and where the break belongs is the one placement formatters argued with. The fixable set is the edits every formatter agrees with.

`--format github` caps annotations at the ten per level GitHub renders and prints one notice for the remainder, instead of flooding the log with commands the interface discards. A new formatter-agreement test tier runs the fixer over formatter-clean input and asserts rustfmt and `dart format` still accept the result.
