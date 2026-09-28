---
monostyle: patch
monostyle_rules: patch
---

# Value-form decisions stay unpadded

A decision written as a value is not a statement. Three shapes proved it downstream, each by making the fixer's output fail a repository's formatter.

`final fill = color ?? switch (icon) { … };` assigns a decision's result, so both blank-line rules now walk up through continuation lines to the binding that is still unfinished and stand down. `} else if (name.startsWith('h') ||` reopens a chain that continues onto the next line, so the after-rule requires the chain to actually be finished. `final m = { for (…) …, if (…) … };` puts a `for` or `if` on element lines of one map value, and the enclosing opener is inspected directly because the depth table deliberately ignores braces.

Two rules anchoring a blank at the same byte — a `return [for (…) …];` is both a return and a control-flow statement — both applied and wrote two blanks where one was asked for. The fix engine now collapses identical insert anchors into the single edit they agree on.
