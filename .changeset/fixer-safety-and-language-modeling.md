---
monostyle: minor
monostyle_rules: minor
monostyle_lexer: minor
monostyle_core: patch
---

# The fixer refuses to corrupt, and the lexer models raw strings and text blocks

Three rules could edit bytes they did not understand. Each now stands on a guarantee.

`blank-line-before-control-flow` padded `} else if`, `} catch`, and bare `else` — the middle of a single decision chain — and `blank-line-after-control-flow` inserted blanks inside function-call argument lists after a `match` used as an expression. Both now read the shape before firing: a line that continues the statement above is not a new decision, and a line starting inside an open expression is not a statement.

The detached-comment fix spanned from its blank through the comment _below_ the gap, so applying it could erase the comment (the structural check caught this downstream and reverted whole files, silently withholding every other fix). The fix now spans the blank run only, and a comment below the gap starts its own block.

The fixer itself gained four guards. A fix may not land inside a literal's bytes or a comment's; a file whose scan hit an unterminated construct is skipped outright, because the lexer was guessing where its constructs end; a whitespace-only rewrite is re-lexed and compared structurally before it is written, and any mismatch throws the whole rewrite away; and a pure-CRLF file stays pure-CRLF.

The scanner learned two constructs it flattened into plain strings. C++'s `R"(…)"` and `R"delim(…)delim"` — braces, quotes, and parens inside are data, and the closer joins the custom delimiter. Java text blocks, where a `"""` block's braces and single quotes are content until the closing triple.
