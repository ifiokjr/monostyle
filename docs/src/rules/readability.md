# Readability rules

<!-- {=readabilityRules} -->

| Rule                               | What it catches                                                         |
| ---------------------------------- | ----------------------------------------------------------------------- |
| `blank-line-before-control-flow`   | An `if`/`for`/`while` crowded against the statement above               |
| `blank-line-before-return`         | A `return` buried against the code above it                             |
| `group-separation`                 | A run of statements longer than `max-statements-per-group` with no gaps |
| `excessive-blank-lines`            | A run of blank lines longer than the language allows                    |
| `excessive-indentation`            | Lines indented past the readable limit                                  |
| `deep-nesting`                     | Control flow nested past the configured depth                           |
| `long-parameter-list`              | An argument list that should be split across lines                      |
| `overlong-line`                    | A line wider than the readable limit                                    |
| `oversized-unit`                   | A function too long to hold in your head                                |
| `oversized-file`                   | A file too large to navigate                                            |
| `mixed-indentation`                | A file that indents with both tabs and spaces                           |
| `comment-required-on-complex-unit` | A complex function with no explanation                                  |
| `comment-explains-why`             | **Credit** for a comment that explains reasoning                        |
| `comment-narrates-code`            | A comment that restates what the code already says                      |
| `excessive-comments`               | More commentary than the code can carry                                 |
| `thin-documentation`               | A doc block that lists structure without explaining purpose             |
| `magic-number`                     | A meaningful numeric literal that should be a named constant            |
| `short-identifier`                 | An identifier too short to convey meaning                               |
| `empty-handler`                    | An error handler that discards the error                                |
| `commented-out-code`               | A block of code commented out instead of deleted                        |

<!-- {/readabilityRules} -->
