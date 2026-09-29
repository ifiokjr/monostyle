---
monostyle: minor
monostyle_core: patch
monostyle_languages: patch
monostyle_rules: minor
---

# Measure what the author chose, not what the formatter produced

Rolling monostyle across ten real repositories produced more than 40,000 findings, and the largest clusters were the tool being wrong rather than the code: the formatter's own layout punished as nesting, valid fence tags called misspellings, and names that belong to the language reported as naming problems. Each is fixed here.

**Indentation and line width.** A call's arguments, a chained call, an operator continuation, a closing bracket, and a match arm's label and body all sit deeper than the statement they belong to because the formatter put them there. Only a line that opens something is measured now, and a new `tab-width` key measures columns the way the project's formatter draws them — dprint's common TypeScript setting writes two-column tabs, and charging four made every multi-level line report as both over-indented and over-long.

**Magic numbers.** A literal is named when the thing beside it names it: an enum variant's discriminant, a named field's value, a type's own parameter (`[u8; 32]`, `String<64>`), and a row of a data table. Those four covered the overwhelming majority of the 6,380 findings lootbox reported and the 8,233 solana_kit reported, none of which any author could act on.

**Short identifiers.** `Ok`, `Err`, `u8`, `i32`, `V1`, and the coordinate conventions (`dx`, `dy`, `rx`, `ry`) are vocabulary rather than choices, and a numeric literal is never an identifier.

**Markdown.** `toml`, `yaml`, `json`, `text` and thirty more are valid fence tags that monostyle simply does not analyze; only a tag naming no language at all is reported now, which is what makes the rule able to catch a real typo. A `title:` in frontmatter counts as the document's title, so a docs site's pages are no longer reported for having none.

**Two rules corrected.** A same-line Swift `} catch { result(error) }` was reported as an empty handler because the non-empty body fell through to the next line — the enclosing brace. And `case`/`when`/`default` left the nesting keyword lists: an arm label is a branch of a decision already counted, and its body sits at the arm's level rather than a level deeper.
