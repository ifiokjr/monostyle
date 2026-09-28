---
monostyle_rules: patch
---

# A do-while tail is the same statement

`} while (…);` closes the loop the lines above opened, but its `while` read as a new decision and the restored before-control-flow fix padded a blank before it, which dart format removed. Only the braced form is exempt — a bare `while (…) {` is still a statement of its own.
