---
"monostyle_lexer": minor
"monostyle_rules": minor
"monostyle_metrics": patch
"monostyle": patch
---

# The line model borrows the source instead of copying it

The scanner made one heap allocation per character: it read the rest of the line into a string for every code character to look for a heredoc, collected a lookahead window for every character before checking whether a literal could begin there, and cloned the open literal — owned closer included — for every character inside a string. Each of those is now a slice or a borrowed profile string. On this repository's largest file, lexing went from 146,116 allocations to 6,822 (64 per line to 3), and a plain code line now allocates exactly once.

`LexedFile` and `LexedLine` carry the source's lifetime: a line's text and indentation are slices of the source, its decision and nesting keywords are the profile's own static strings, the keyword pass tokenizes into slices rather than owned words, and the masked view is the text's own prefix until a literal or comment forces it apart. `LexedLine::comment_body` returns a slice rather than a copy, and the unused `markdown::lexed_lines` helper is gone.

Every CRLF-terminated line's byte range ended one byte early, and a blank CRLF line's range ran backwards, because the line was closed one byte too soon. The copied text hid it; the borrowed text exposed it. The range is now exact.
