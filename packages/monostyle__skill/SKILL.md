---
name: "monostyle"
description: "Use when scoring or improving the readability and complexity of code. Measures whether complex sections are given room, whether sequential control flow is separated, how deeply logic nests, and whether comments explain why. Use for reviewing a codebase, a file, or a single function, and for checking your own output before finishing."
---

# @monostyle_rs/skill

Scores code for **readability** and **complexity**, out of 100 each, and explains every point lost
as a named rule with a location and a suggested fix.

The point of running it is not the number. It is that a vague sense that code "looks bad" becomes a
list of specific places to change, ranked by how much each one costs.

## Running it

```console
monostyle check .                       # score a repository
monostyle check src/lib.rs              # score one file
monostyle check . --units               # add per-function scores
monostyle check . --explain             # list every finding with its fix
monostyle check . --format json         # machine-readable
monostyle check . --fail-under 75       # exit non-zero below a threshold
monostyle rules                         # list every rule
```

Run it on the smallest scope that contains the code you changed. Scoring one file is fast and keeps
the output focused on what you can actually act on.

## Reading the output

The report is ordered the way you should act on it:

1. **The header** gives both scores, weighted by lines of code, so a large file matters more than a
   small one. This is the same way coverage is aggregated.
2. **The impact table** ranks rules by the share of total penalty each accounts for. Read the top
   row first: it is the single change worth the most points.
3. **Each impact names its worst offender** as `path:line`. That is where to look.
4. **`start here`** at the bottom names the highest-value single fix.

When you are asked to improve a score, work the impact table from the top. Fixing the first row
often moves the score more than fixing everything below it combined.

## The rules

Readability:

| Rule                               | What it means                                           | What to do                                          |
| ---------------------------------- | ------------------------------------------------------- | --------------------------------------------------- |
| `blank-line-before-control-flow`   | An `if`/`for`/`while` is crowded against the line above | Put a blank line before each control-flow statement |
| `blank-line-before-return`         | A `return` is buried against the code above it          | Put a blank line before the return                  |
| `group-separation`                 | A long run of statements with no blank lines            | Separate the logical groups within the run          |
| `deep-nesting`                     | Control flow nested past the limit                      | Flatten with an early return or extract the block   |
| `excessive-indentation`            | A line indented past the limit                          | Same fix: flatten or extract                        |
| `long-parameter-list`              | An argument list that should be split                   | One argument per line                               |
| `overlong-line`                    | A line wider than the readable limit                    | Break it at a logical boundary                      |
| `oversized-unit`                   | A function too long to hold in your head                | Extract the distinct phases into named helpers      |
| `oversized-file`                   | A file too large to navigate                            | Split it along its natural seams                    |
| `mixed-indentation`                | Tabs and spaces in one file                             | Pick one                                            |
| `comment-required-on-complex-unit` | A complex function with no explanation                  | Add a comment explaining why it is complex          |
| `comment-explains-why`             | **Credit** for explaining reasoning                     | Keep it                                             |
| `comment-narrates-code`            | A comment restating the code                            | Delete it or explain why instead                    |
| `excessive-comments`               | More commentary than the code can carry                 | Keep the why, delete the what                       |
| `thin-documentation`               | A doc block listing structure without purpose           | Add a sentence on what it is for and why            |

Complexity:

| Rule                  | What it means                                           | What to do                                                              |
| --------------------- | ------------------------------------------------------- | ----------------------------------------------------------------------- |
| `cyclomatic-per-unit` | Too many independent paths to test                      | Extract cohesive branch groups into helpers                             |
| `cognitive-per-unit`  | Hard to follow; the message reports the nesting penalty | Flattening resets the nesting penalty without reducing the branch count |
| `cyclomatic-per-file` | A file dense with decisions                             | Split it into smaller modules                                           |

## What actually improves the score

The rules reward one habit above all: **give complex code room, and flatten it.**

- A blank line before every control-flow statement. This is the cheapest fix and usually the largest
  single win, because it applies everywhere at once.
- Early returns instead of nested `if`s. Extraction resets nesting to zero, so cognitive complexity
  drops even when the number of branches does not.
- A blank line between logical groups inside a function, so its phases are visible.
- Split an argument list across lines when it does not fit — the rule measures rendered width, not
  just the count, so a short numeric call is left alone.
- Comment the _why_ on any function that is genuinely complex. The score asks for this only above a
  cognitive threshold, so it does not nag simple code.

## Two honest caveats

**The comment classifier is a keyword matcher, not comprehension.** It counts phrases from two
curated lists. It cannot see negation, so a comment like "do not increment the counter" reads as
narration. It stays quiet when a comment is ambiguous rather than guessing, which means a good
comment occasionally reports `Neutral` instead of credit. Do not treat a `comment-narrates-code`
finding as authoritative without reading the comment yourself.

**Scores are comparable within a run, not across configurations.** Changing thresholds in
`monostyle.toml` changes what the numbers mean, so a score recorded before a config change is not
comparable to one after it.

## Improving your own output

If you are writing code rather than reviewing it, run monostyle on the files you changed before
finishing. The rules are the mechanical form of the layout conventions in the coding style guide, so
running them catches drift that reading the code back will not.

```console
monostyle check path/to/changed/file.rs --fail-under 85
```

A useful loop: run it, fix the top row of the impact table, run it again. Two or three passes
usually takes a file from fair to good.

## Further reading

- [`skills/reference.md`](./skills/reference.md) — every flag, configuration key, and output format
- [`skills/scoring.md`](./skills/scoring.md) — how the two scores are computed, and why they are
  weighted by lines of code
