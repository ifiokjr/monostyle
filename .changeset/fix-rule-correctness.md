---
"monostyle": minor
"monostyle_core": minor
"monostyle_lexer": minor
"monostyle_metrics": minor
"monostyle_rules": minor
---

# Fix five rule-correctness defects

Running monostyle across thirteen real repositories surfaced five defects that made a score misleading or made the advice impossible to follow. The unsatisfiable `blank-line-before-return` rule found in the same run is fixed separately.

- **A configuration section discarded the repository-wide rules.** A section's rules replaced `[rules]` instead of layering onto it, so every threshold a section did not restate silently reverted to its default for the paths that section matched. The file read correctly, which made the effect invisible.
- **The code rules ran against Markdown prose.** A Markdown file lexes as one language, so `magic-number` read a numbered list's `5.` as a numeric literal and `group-separation` read the list as an unbroken statement run. A prose list scored 86.6 where it should score 100.
- **A bodyless declaration ran to the end of the file.** A trait method ending in `;` never opened a body, so the unit stayed open until the file did, and a one-line declaration was reported as an oversized unit with a low maintainability index.
- **A rustdoc `# Errors` list was read as commented-out code.** The rule treats `::` as proof of code, and a doc comment listing variants writes them as `Error::Variant`.
- **A keyword inside an attribute was read as control flow.** `#[serde(default, rename_all = "kebab-case")]` was reported as a `default` branch missing its blank line, so the finding asked for a blank line inside an attribute list. A decorator or annotation has the same shape.

**Added:** `readability/excessive-blank-lines`. Every layout rule asks for a gap and none capped one, so following the tool's own advice could grow a gap without limit; five blank lines between two `match` arms satisfied every rule that asked for a separation. The rule takes the larger of the configured maximum and the language's own convention, so PEP 8's two blank lines before a top-level Python definition are respected.

These fixes change scores, which is why this is a minor release rather than a patch: fewer findings are reported on the same code, and a repository that was failing a threshold may now pass it.
