---
name: "monostyle"
description: "Use when scoring or improving the readability and complexity of code. Measures whether complex sections are given room, whether sequential control flow is separated, how deeply logic nests, and whether comments explain why. Use for reviewing a codebase, a file, or a single function, and for checking your own output before finishing."
---

# @monostyle-rs/skill

Scores code for **readability** and **complexity**, out of 100 each, and explains every point lost as a named rule with a location and a suggested fix.

The point of running it is not the number. It is that a vague sense that code "looks bad" becomes a list of specific places to change, ranked by how much each one costs.

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

Run it on the smallest scope that contains the code you changed. Scoring one file is fast and keeps the output focused on what you can actually act on.

## Reading the output

The report is ordered the way you should act on it:

1. **The header** gives both scores, weighted by lines of code, so a large file matters more than a small one. This is the same way coverage is aggregated.
2. **The impact table** ranks rules by the share of total penalty each accounts for. Read the top row first: it is the single change worth the most points.
3. **Each impact names its worst offender** as `path:line`. That is where to look.
4. **`start here`** at the bottom names the highest-value single fix.

When you are asked to improve a score, work the impact table from the top. Fixing the first row often moves the score more than fixing everything below it combined.

## The rules

Readability:

| Rule                               | What it means                                                                          | What to do                                                                                          |
| ---------------------------------- | -------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| `blank-line-before-control-flow`   | An `if`/`for`/`while` is crowded against the line above                                | Put a blank line before each control-flow statement                                                 |
| `blank-line-after-control-flow`    | A statement crowded against the block above it                                         | Put a blank line after the block                                                                    |
| `detached-comment`                 | A comment separated from the code it documents                                         | Delete the blank; the comment belongs to the code                                                   |
| `blank-line-before-return`         | A `return` is buried against the code above it                                         | Put a blank line before the return                                                                  |
| `group-separation`                 | A run of statements longer than `max-statements-per-group` (8 by default) with no gaps | Break the run at a concept boundary — see "Blank lines are paragraphs"                              |
| `excessive-blank-lines`            | A run of blank lines longer than the language allows                                   | Delete the extras: one blank is the allowance for Rust, Go, and TypeScript, two for Python and Dart |
| `deep-nesting`                     | Control flow nested past the limit                                                     | Flatten with an early return or extract the block                                                   |
| `excessive-indentation`            | A line indented past the limit                                                         | Same fix: flatten or extract                                                                        |
| `long-parameter-list`              | An argument list that should be split                                                  | One argument per line                                                                               |
| `overlong-line`                    | A line wider than the readable limit                                                   | Break it at a logical boundary                                                                      |
| `oversized-unit`                   | A function too long to hold in your head                                               | Extract the distinct phases into named helpers                                                      |
| `oversized-file`                   | A file too large to navigate                                                           | Split it along its natural seams                                                                    |
| `mixed-indentation`                | Tabs and spaces in one file                                                            | Pick one                                                                                            |
| `comment-required-on-complex-unit` | A complex function with no explanation                                                 | Add a comment explaining why it is complex                                                          |
| `comment-explains-why`             | **Credit** for explaining reasoning                                                    | Keep it                                                                                             |
| `comment-narrates-code`            | A comment restating the code                                                           | Delete it or explain why instead                                                                    |
| `excessive-comments`               | More commentary than the code can carry                                                | Keep the why, delete the what                                                                       |
| `thin-documentation`               | A doc block listing structure without purpose                                          | Add a sentence on what it is for and why                                                            |
| `magic-number`                     | A meaningful numeric literal left unnamed                                              | Give it a named constant                                                                            |
| `short-identifier`                 | A name too short to convey meaning                                                     | Rename it to say what it holds                                                                      |
| `empty-handler`                    | An error handler that discards the error                                               | Log it, propagate it, or explain why it is safe                                                     |
| `commented-out-code`               | Code commented out instead of deleted                                                  | Delete it; version control remembers                                                                |

Complexity:

| Rule                  | What it means                                           | What to do                                                              |
| --------------------- | ------------------------------------------------------- | ----------------------------------------------------------------------- |
| `cyclomatic-per-unit` | Too many independent paths to test                      | Extract cohesive branch groups into helpers                             |
| `cognitive-per-unit`  | Hard to follow; the message reports the nesting penalty | Flattening resets the nesting penalty without reducing the branch count |
| `cyclomatic-per-file` | A file dense with decisions                             | Split it into smaller modules                                           |

## What actually improves the score

The rules reward one habit above all: **give complex code room, and flatten it.**

- A blank line before every control-flow statement. This is the cheapest fix and usually the largest single win, because it applies everywhere at once.
- Early returns instead of nested `if`s. Extraction resets nesting to zero, so cognitive complexity drops even when the number of branches does not.
- A blank line between logical groups inside a function, so its phases are visible — placed at a boundary between concepts, never inside one (see the next section).
- Split an argument list across lines when it does not fit — the rule measures rendered width, not just the count, so a short numeric call is left alone.
- Comment the _why_ on any function that is genuinely complex. The score asks for this only above a cognitive threshold, so it does not nag simple code.

## Blank lines are paragraphs

A blank line starts a new paragraph, and a paragraph is one concept. Related lines — a declaration and the lines that complete it, a value and its immediate uses, the steps of one phase — stay in the same paragraph. The break goes **between** concepts: before a new resource is opened, before a new phase begins, before the function's answer is assembled.

When `group-separation` reports a long run, do not insert a blank wherever the count happens to land. Find the nearest concept boundary and break there — usually earlier than the limit. If you cannot find a boundary anywhere in the run, the function is doing several jobs in one paragraph, and the right fix is to name them: extract a helper or regroup the statements so the concepts separate themselves.

Before — the break lands in the middle of the `revision` concept, between the buffer and the lines that fill it:

```rust
let mut sequence = [0u8; 8];
let chunk = plan.take(8);
sequence[..chunk.len()].copy_from_slice(chunk);
let mut revision = [0u8; 4];

let chunk = plan.take(4);
revision[..chunk.len()].copy_from_slice(chunk);
```

After — each buffer and its fill is one paragraph, and the break separates the two concepts:

```rust
let mut sequence = [0u8; 8];
let chunk = plan.take(8);
sequence[..chunk.len()].copy_from_slice(chunk);

let mut revision = [0u8; 4];
let chunk = plan.take(4);
revision[..chunk.len()].copy_from_slice(chunk);
```

The same reading applies everywhere:

- **Build then use.** Constructing an object and the first thing done to it are one paragraph; the break belongs before the construction, not between the construction and its setup calls.
- **Tests.** Arrange, act, assert are the paragraphs. Every line that prepares one fixture belongs together, and the assert block starts its own paragraph.
- **Phases.** Fetch, then validate, then mutate, then return. A phase change is a boundary; a step inside a phase is not.
- **Two statements that mention the same name are not automatically one concept.** A binding that is finished with, followed by a new idea that reads it, is a boundary; a binding followed by the lines that complete its own setup is not.

The mechanical rule counts statements; paragraphs are what the count is a proxy for. Code that is paragraphed well never trips the rule, and code that trips it always has a boundary hiding somewhere near the finding.

## Two honest caveats

**The comment classifier is a keyword matcher, not comprehension.** It counts phrases from two curated lists. It cannot see negation, so a comment like "do not increment the counter" reads as narration. It stays quiet when a comment is ambiguous rather than guessing, which means a good comment occasionally reports `Neutral` instead of credit. Do not treat a `comment-narrates-code` finding as authoritative without reading the comment yourself.

**Scores are comparable within a run, not across configurations.** Changing thresholds in `monostyle.toml` changes what the numbers mean, so a score recorded before a config change is not comparable to one after it.

## Improving your own output

If you are writing code rather than reviewing it, run monostyle on the files you changed before finishing. The rules are the mechanical form of the layout conventions in the coding style guide, so running them catches drift that reading the code back will not.

```console
monostyle check path/to/changed/file.rs --fail-under 85
```

A useful loop: run it, fix the top row of the impact table, run it again. Two or three passes usually takes a file from fair to good.

## Further reading

- [`skills/reference.md`](./skills/reference.md) — every flag, configuration key, and output format
- [`skills/scoring.md`](./skills/scoring.md) — how the two scores are computed, and why they are weighted by lines of code
