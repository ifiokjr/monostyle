---
monostyle: minor
monostyle_rules: minor
---

# `blank-line-before-return` is now auto-fixable

The return rule asked for a blank line but carried no fix, which made the largest single finding class in every repository a manual chore. The fix inserts the blank above the return — the same edit, and the same formatter-safety argument, as the blank-line insertion the before-control-flow rule has always carried: Rustfmt, Prettier, Black, and `dart format` all preserve blank lines between statements and none of them add one.

`monostyle rules --fixable` now lists five fixable rules, and the discovery sample fires on every one of them. The idempotence property holds: a second `monostyle fix` run changes nothing.
