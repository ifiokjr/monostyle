---
monostyle_lexer: minor
monostyle_metrics: patch
monostyle_rules: patch
---

# An else-if is one decision, not three

The lexer recorded "if", "else if", and "else" as three nesting keywords for a single `} else if x {` line, so cognitive complexity charged one construct three times, once at full nesting depth. A function whose honest score is 12 measured 17. The else-if count is now subtracted from the bare "if" and "else" counts, which charged every else-if-bearing codebase the tool has scored.
