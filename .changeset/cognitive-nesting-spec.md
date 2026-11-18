---
"monostyle_metrics": patch
---

# Count cognitive nesting the way the specification does

The depth counter treated the unit's own body, every match-arm block, and every `} else {` as a nesting level, so a function paid points the SonarSource specification never charges. Nesting now rises only for `if`, loops, `match`/`switch`, `catch`, nested functions, and closures. `else if` chains, including Ruby's `elsif`, stay flat, and a block that opens and closes on one line no longer leaves its level behind. Scores for nested code drop to the spec's numbers: four sequential top-level `if`s score 4, and a three-arm `match` of `if`/`else` scores 10.
