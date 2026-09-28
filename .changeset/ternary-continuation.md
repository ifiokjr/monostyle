---
monostyle_rules: patch
---

# A ternary arm continues its statement

A switch expression as one arm of a ternary — `? switch (…) { … }` — closes with a brace, and the `: fallback` arm below it continues the binding. The after-rule read that closer as a finished chain and padded a blank before the `:`, which dart format removed. A closer followed by a continuation line now stands down, and `:` joins the continuation openers.
