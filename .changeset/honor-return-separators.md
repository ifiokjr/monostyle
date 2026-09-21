---
monostyle_rules: patch
---

# Honor blank separators before returns

The return-spacing rule now checks the physical separation above a return before looking for the previous code line, so a blank line satisfies the rule instead of being skipped.
