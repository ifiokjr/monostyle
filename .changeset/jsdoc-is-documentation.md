---
monostyle_lexer: patch
monostyle_rules: patch
---

# JSDoc blocks are documentation, not prose

A `/** */` block comment is documentation the same way Rust's `///` is, but only line comments carried the style, so every line of a JSDoc block was keyword-judged as a casual inline comment. The style now travels with the block. `step_literal` also reads as the ordering rule it documents: each check is a named helper that consumes what it matched, which took its cognitive complexity from 21 to under the limit.
