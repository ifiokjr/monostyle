---
monostyle_rules: patch
---

# A match-arm guard continues its arm

A pattern split from its guard — `[root, module, name]` then `if root == "core" && … =>` — puts an `if` at the start of a line that belongs to the arm above it. The restored before-control-flow fix padded a blank between the pattern and its guard, which rustfmt removed. A line whose decision ends with `=>` is an arm clause, never a statement of its own.
