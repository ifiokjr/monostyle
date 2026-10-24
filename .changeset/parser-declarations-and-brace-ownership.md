---
"monostyle_metrics": patch
"monostyle_rules": patch
---

# Preserve function declarations and brace ownership

Recognize Rust functions after restricted visibility and qualifiers, while keeping
constant tables and function-pointer data out of function metrics. Preserve real
function names, boundaries, decisions and parameter findings.

Walk braces in reverse character order when locating an enclosing group. Nested
objects and blocks no longer overflow or lose the required whitespace findings.
