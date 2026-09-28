---
monostyle: patch
monostyle_languages: patch
---

# Rust plain strings may span lines

A Rust `"…"` can legally contain a bare newline, but the plain quote was registered as a single-line literal, so an opening quote with no closer on its line was treated as a mis-read and the string's first line was scanned as code — the `return` inside an embedded JavaScript mock fired the before-return rule, and the inserted blank landed inside the literal where rustfmt removed it. The plain quote is now a multiline rule; the char-literal quote stays single-line, where an unterminated one really is a mis-read.
