---
"monostyle_metrics": patch
"monostyle_rules": patch
"monostyle": patch
---

# The rule pipeline stops copying what it reads

Halstead's distinct-token sets allocated a copy of every token before inserting it — one heap allocation per token, tens per line — and unit detection copied every code line's masked text twice per line. Both borrow now, and the measurement stage allocates a fraction of what it did.

The quality rules borrow their findings' material as well: numeric literals, assigned and declared names, and handler keywords are slices of the line rather than owned strings, keyword comparison no longer lowercases into a heap string, and literal parsing skips separators through a stack buffer. Unit reports carry their risk bands as the static strings they already were, and the group-separation depth table counts all four bracket kinds in one pass over a line instead of four.

Analyzing this repository's largest source file went from 11.1 to 7.3 heap allocations per line, with every test, the allocation ceilings, and the dogfood floor unchanged.
