---
"monostyle_lexer": patch
"monostyle_metrics": patch
---

# Preserve Shell and YAML literal boundaries

Recognize Shell here-strings without opening a heredoc from their operator suffix. Keep complete quoted heredoc openers and every body character inside their correct literal boundaries, including Unicode byte ranges.

Treat YAML literal and folded block payloads as data until dedent or EOF. Embedded quotes and comments no longer hide later mappings or create false diagnostics.

End braced Shell function metrics at their closing brace while preserving the language's decision and nesting rules.
