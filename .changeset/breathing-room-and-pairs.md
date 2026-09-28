---
monostyle: minor
monostyle_rules: minor
---

# Breathing room around control flow returns as a fix

Reviewer guidance made the rule precise: there should always be a blank line around a control-flow statement, with two exceptions. A decision written on one line — `if (a > b) return 1;` or a ternary — is a clause of its surroundings and reads fine crowded. And a single-line binding followed by the single-line return that uses it is one thought, so the blank between them is gone: `const entries = readdirSync(dir);` then `return entries.some(…)` stands as a pair, while a multi-line return or one unrelated to the binding keeps its gap.

The before-control-flow fix returns with the guards it was missing when it was demoted — continuations, directives, attribute blocks, and the single-line shapes above — and anchors above any attribute the statement carries.
