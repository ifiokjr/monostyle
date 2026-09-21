# monostyle

[Documentation](https://ifiokjr.github.io/monostyle/) | [API docs](https://ifiokjr.github.io/monostyle/api/monostyle/)

<!-- {=projectOverview} -->

`monostyle` scores the complexity and readability of a codebase, a file, or a function, out of 100. Every point lost is traced to a named rule with an explanation and a suggested fix, so a report is a worklist rather than a grade.

**Readability** measures how the code looks: whether complex sections have room to breathe, whether sequential control flow is separated, whether deep nesting has been flattened, and whether comments explain the hard parts.

**Complexity** measures how hard the code is to follow and to test: cyclomatic complexity (how many independent paths exist) and cognitive complexity (how much nesting taxes the reader).

<!-- {/projectOverview} -->

<!-- {=projectWhy} -->

Code is read far more often than it is written, and the things that make it pleasant to read are mostly layout: a blank line before a branch, space around a long argument list, a gap between logical groups, and a comment on the one function that is genuinely hard to follow.

Those things are invisible to every existing metric. Cyclomatic complexity will happily call a flat, unreadable function simple; a formatter will happily preserve a 200-line function with no blank lines anywhere. monostyle exists to make the visual properties of code measurable, so that "this is hard to read" becomes a specific, fixable list.

<!-- {/projectWhy} -->

## Installation

<!-- {=installation} -->

```console
cargo install monostyle
```

Or build from source:

```console
cargo build --release --package monostyle
```

<!-- {/installation} -->

## Usage

<!-- {=usageExamples} -->

```console
monostyle check [PATH]...          # analyze a directory, file, or list of paths
monostyle fix [PATH]...            # apply every fixable finding
monostyle fix . --dry-run          # show what would change
monostyle check . --units          # include per-function scores
monostyle check . --explain        # list every finding with its explanation
monostyle check . --format json    # machine-readable output
monostyle check . --fail-under 75  # exit non-zero below a threshold
monostyle rules                    # list every rule
monostyle config                   # print the effective configuration
```

<!-- {/usageExamples} -->

## Rules

### Readability

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

### Complexity

<!-- {=complexityRules} -->

| Rule                  | What it catches                                                      |
| --------------------- | -------------------------------------------------------------------- |
| `cyclomatic-per-unit` | A function with too many independent paths to test                   |
| `cognitive-per-unit`  | A function that is hard to follow, reported with its nesting penalty |
| `npath-per-unit`      | A function with too many execution paths                             |
| `exits-per-unit`      | A function that returns from too many places                         |
| `low-maintainability` | A function with a low maintainability index                          |
| `cyclomatic-per-file` | A file dense with decisions                                          |

<!-- {/complexityRules} -->

### Markdown

<!-- {=markdownRules} -->

Code inside documentation is scored too, because README examples are what people copy.

| Rule                     | What it catches                                         |
| ------------------------ | ------------------------------------------------------- |
| `fence-readability`      | A cramped code example inside a fence                   |
| `fence-without-language` | A fence with no language tag                            |
| `fence-language-unknown` | A fence naming a language monostyle does not know       |
| `prose-run`              | A wall of prose with no structure to break it up        |
| `skipped-heading-level`  | A heading level that skips a step, breaking the outline |
| `no-title`               | A document that does not start with a top-level heading |

<!-- {/markdownRules} -->

## Supported languages

<!-- {=supportedLanguages} -->

Rust, C, C++, C#, Java, JavaScript, Kotlin, Mozjs, Python, TypeScript, TSX, **Dart**, Go, Swift, Ruby, PHP, Scala, Shell, Lua, Elixir, Haskell, Nix, and Markdown.

The first eleven match what [`rust-code-analysis`](https://github.com/mozilla/rust-code-analysis) supports, so numbers from the two tools are comparable. Dart is included because it is the language this tool was built for. The remaining ten cover widely used languages that project does not reach.

<!-- {/supportedLanguages} -->

## How scoring works

<!-- {=scoringModel} -->

Findings carry a `weight`; severity scales it into a penalty. Penalties sum per category and are normalized by code volume into a penalty density — findings per 100 lines — so a large well-written file is not punished for its size. Density maps to 0–100 through exponential decay:

```
score = 100 * 2 ^ (-density / half_life)
```

The half-life is the density at which a category scores exactly 50, which makes the whole curve tunable with one readable number.

Good comments earn **negative** penalties. That is how a well-placed explanation raises a score: the credit offsets other penalties inside the same density number, so one number always explains the result.

Scores are aggregated by **line-weighted mean**, the same way test coverage is aggregated. A ten-line file cannot count as much as a thousand-line file. Three invariants hold, and all three are enforced by tests: splitting a file does not change the project score, doubling penalty and volume does not change it, and fifty two-line files cannot outweigh one five-thousand-line file.

<!-- {/scoringModel} -->

## Finding what to fix

<!-- {=impactExample} -->

```console
$ monostyle check . --units
readability: what is costing you points

  ██████░░░░  61.2%  readability/blank-line-before-control-flow  (4,181 findings)
             crates/example/src/main.rs:91 [minor]
             `if` follows the previous statement with no blank line between them
             -> Add a blank line before this statement so the reader can treat it as
             a separate decision rather than part of the previous block.
```

Each entry names its worst offender as `path:line`, and the report ends with the single highest-value fix:

```console
start here
  Fixing readability/blank-line-before-control-flow at crates/example/src/main.rs:91 would
  recover 30.7% of the available points.
```

<!-- {/impactExample} -->

## Auto-fix

<!-- {=autofixExplanation} -->

Two rules are auto-fixable, and both edits are ones a formatter leaves alone.

`blank-line-before-control-flow` inserts a blank line before a control-flow statement. Rustfmt, Prettier, Black, and `dart format` all preserve a blank line between statements and none of them remove one.

`excessive-blank-lines` removes the blank lines past the allowance. The number to keep is the same one the finding measured against, so the fix cannot disagree with the rule and running it twice changes nothing the second time.

Those two together are what make the whitespace rules safe to follow automatically: the other rules ask for gaps without bounding them, and this pair supplies both the gap and the ceiling, so a fixer cannot grow a file into mostly whitespace.

Every other rule explains itself and leaves the change to you — breaking a long line, renaming an identifier, extracting a function, and adding an explanatory comment are judgement calls whose automated version would be worse than the problem. The fix output shows both: what was applied, and what still needs a decision, with the suggestion attached.

<!-- {/autofixExplanation} -->

## Ignoring files

<!-- {=ignoreConfigExample} -->

```toml
[rules.ignore]
patterns = ["**/*.spec.ts", "crates/legacy/**"]
generated = true # skip generated code (the default)
include = ["lib/hand_edited.g.dart"] # always score this one
```

Recognized as generated: `.g.dart`, `.freezed.dart`, `.pb.rs`, `.pb.go`, `_pb2.py`, `.designer.cs`, `.gen.ts`, `.min.js`, `.bundle.js`, and lock files. Ignored directories include `node_modules`, `target`, `dist`, `build`, `vendor`, `.venv`, `.dart_tool`, and `__pycache__`.

<!-- {/ignoreConfigExample} -->

## Configuration

<!-- {=configExample} -->

```toml
[scoring]
half-life = 12.0

[rules]
# Only the fields you set are changed; everything else keeps its default.
max-nesting-depth = 3
max-parameters-inline = 3
max-cyclomatic-per-unit = 10
max-cognitive-per-unit = 15
max-line-width = 120
max-statements-per-group = 8
max-consecutive-blank-lines = 1
comment-required-above-cognitive = 10
disabled-rules = ["readability/excessive-comments"]

[rules.ignore]
patterns = ["**/*.spec.ts", "crates/legacy/**"]
generated = true
```

`max-statements-per-group` and `max-consecutive-blank-lines` are the two halves of the same instruction: the first says when a run of statements needs a gap, the second says how large that gap may grow. `max-consecutive-blank-lines` is a floor rather than a cap for languages with their own convention — PEP 8 asks for two blank lines before a top-level Python definition, and `dart format` does the same — so the allowance is whichever is larger, and only a longer run is reported.

<!-- {/configExample} -->

## Performance

| Repository | Files | Lines of code | Time  |
| ---------- | ----- | ------------- | ----- |
| mdt        | 228   | 35,749        | 0.13s |
| monochange | 318   | 193,900       | 0.42s |
| pina       | 2,198 | 232,417       | 1.08s |

See [docs/performance.md](./docs/performance.md) for what made it fast and what the cache does.

## Design

The architecture, and why a profile-driven lexer was chosen over tree-sitter, is documented in [ARCHITECTURE.md](./ARCHITECTURE.md). The short version: readability is a layout metric, so the tool needs a trustworthy tokenizer over comments and string literals rather than a full parse tree, and the tokenizer's correctness is guarded by the heaviest test suite in the repository.

## npm

```console
npm install -g @monostyle-rs/cli
```

## License

Unlicense.
