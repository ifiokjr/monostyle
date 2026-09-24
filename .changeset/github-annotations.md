---
monostyle: minor
---

# Inline annotations on GitHub pull requests

`monostyle check . --format github` now emits every finding as a GitHub Actions workflow command. GitHub renders those as inline annotations on the pull request diff — the same surface an ESLint or Clippy annotation uses — so a reviewer sees where the improvements are without opening the full report.

Severity maps to the annotation level: `Minor` findings are warnings and `Major` and `Critical` findings are errors. The `%`, `\r`, and `\n` characters in the message are encoded, so a message containing a percent sign or a newline cannot truncate the annotation.
