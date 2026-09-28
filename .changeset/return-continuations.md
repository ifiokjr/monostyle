---
monostyle_rules: patch
---

# A return on a continuation line is a clause, not an exit

`) return null;` closes a multi-line `if (` condition and carries the return as a clause of that statement, but the before-return rule read it as an exit crowded against work and placed a blank above it, which dprint removed. The rule now applies the same continuation guards as the before-control-flow rule.
