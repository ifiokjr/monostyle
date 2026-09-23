---
monostyle_lexer: patch
monostyle_rules: patch
monostyle: patch
---

# Fix Dart raw strings swallowing the rest of the file

In Dart, `r'...'` is a raw string: the backslash is an ordinary character, not an escape. The scanner honored `\'` inside `r'...'` anyway, so a raw string holding a backslash never closed. Every following line was classified as blank string content, with two consequences: the layout rules went blind for the rest of the file, and the blank-line fixer deleted the "blank" lines — real code — as formatting. On a downstream repository the fixer removed seven lines of live Dart from a build script this way.

Raw prefixed forms no longer honor escapes, and a backslash at the end of a raw line no longer continues the literal. The file now lexes as code, and the fixer leaves it alone.
