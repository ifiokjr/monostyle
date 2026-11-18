---
"monostyle_lexer": patch
---

# Mask JavaScript regex literals that extend past the opener peek

The scanner looked for a regex closer inside the six-character delimiter peek. A literal longer than that window, such as one whose character class contains a backtick, left the opening slash as code, and the backtick opened a template that was never closed. Every score after that point in the file was then wrong. The closer is now found on the rest of the line, and the literal counts as a value so the next slash still divides.
