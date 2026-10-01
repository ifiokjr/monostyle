---
monostyle: patch
monostyle_rules: patch
---

# A multiline ternary is one statement

A ternary spread over lines carries no brackets for the depth table to see, so `group-separation` read its `?` and `:` arms as separate statements and split one — inserting a blank between a condition and its arm that the project's formatter removes. Continuation openers now join the statement above them.
