---
monostyle: minor
monostyle_rules: minor
---

# Breathing room is uniform and nesting is measured where it lives

Rolling the relaxed rules across the corpus showed three places where the tool had gone quiet on real findings. Each is restored here, and the false-positive guards stay.

**Breathing room applies uniformly.** A decision written on one line (`if (a > b) return 1;`) and a single-line binding followed by the return that uses it were exempted from the blank-line rules; both now separate like any other statement. What stays exempt is everything that is not a statement at all: continuations, value-form expressions, attributes, block declarations, and — new here — match and switch arms in every syntax (`case x:`, `default:`, `=>`, and Java's `->`), which are alternatives within one decision rather than a sequence. Ruby's `def`/`class`/`module` openers are recognized as block declarations, so a modifier-if guard no longer takes a blank as the first statement of a method.

**Deep nesting inside expressions is measured again.** The indentation rule exempted every line inside an unclosed call, which silenced it on callback bodies and widget trees — the exact shape it exists to flag. The exemption now covers only the formatter's own layout: continuation lines, closers, and arm labels. A line inside an expression that opens a block, carries a decision, or returns is the author's nesting, and a multi-line arm body is measured like any other block — only its label is conventional.

**The short-identifier vocabulary is trimmed.** Single letters beyond the loop-and-coordinate conventions (`d`, `e`, `s`, `o`, …) and ad-hoc abbreviations (`op`, `ar`, `ir`, `fd`, `rc`, `ch`) are choices again; the coordinates (`dx`, `dy`, `rx`, `ry`, …), the time units (`ms`, `ns`, `us`), and the language's own names (`Ok`, `Err`, `u8`, `T`, `V1`) stay, because no author can rename those.
