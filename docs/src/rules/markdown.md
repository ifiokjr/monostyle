# Markdown rules

<!-- {=markdownRules} -->

Code inside documentation is scored too, because README examples are what people copy.

| Rule | What it catches |
| --- | --- |
| `fence-readability` | A cramped code example inside a fence |
| `fence-without-language` | A fence with no language tag |
| `fence-language-unknown` | A fence naming a language monostyle does not know |
| `prose-run` | A wall of prose with no structure to break it up |
| `skipped-heading-level` | A heading level that skips a step, breaking the outline |
| `no-title` | A document that does not start with a top-level heading |

<!-- {/markdownRules} -->

## Why fences are scored differently from source

Complexity rules are deliberately excluded from fences. A documentation example often walks through a messy
state on purpose, and penalizing it would push authors toward hiding the very complexity they are explaining.
Only the layout rules apply, and their findings are mapped back to the Markdown file's line numbers so a
reader can jump to the exact fence.
