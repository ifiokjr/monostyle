---
monostyle_rules: patch
---

# Count multiline continuations as one statement

The group-separation rule now ignores lines that begin inside an open expression or continue a formatted method chain. Multiline calls and collections no longer inflate the statement count, while genuine eight-statement runs remain reportable.
