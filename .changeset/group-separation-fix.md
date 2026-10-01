---
monostyle: minor
monostyle_rules: minor
---

# Group separation becomes fixable

`group-separation` now carries a fix: a blank line above the run's next group — the (limit + 1)-th statement — with the blank opening above an attached comment rather than stranding it from its code. The edit is the same one the other blank-line rules make, which is the argument for its safety: formatters keep blank lines between statements. A run longer than twice the limit takes one blank per `fix` invocation, each the same instruction.
