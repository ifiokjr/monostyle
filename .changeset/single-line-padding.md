---
monostyle: patch
monostyle_rules: patch
---

# Single-line control-flow statements read as a sequence

Two rules asked for padding where the code already reads as one shape.

`blank-line-before-control-flow` fired on the second `if` in a pair like `if (a > b) return 1;` / `if (c < d) return 0;` — both single-line, both guards, no blank between them. That is a tight guard sequence, the same structure the return rule already exempts, and it now reads as one too: a control-flow statement whose braces balance and which opens no block is single-line, and two of those in a row need no blank between them. A multi-line block still asks for the blank, because its body is a group.

`blank-line-before-return` fired on `Err(tokens) => return tokens,` inside a match arm, which is what made the fixer insert a blank between two single-line arms. A return inside a match or switch arm is an alternative — a case of one decision — and padding between the arms is the formatter's call.
