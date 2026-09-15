# monostyle

Measure the complexity and readability of a codebase, a file, or a function — and get an
explanation for every point you lose.

```console
$ monostyle check crates
monostyle
============================================================
  files analyzed  36
  lines of code   5727

  readability       79.9  [########..]   good
  complexity        83.2  [########..]   good
  overall           81.5  [########..]   good
```

Two scores out of 100, where 100 is clean:

- **Readability** measures how the code looks: whether complex sections have room to breathe,
  whether sequential control flow is separated, whether deep nesting has been flattened, and
  whether comments explain the hard parts.
- **Complexity** measures how hard the code is to follow and to test: cyclomatic complexity
  (how many independent paths exist) and cognitive complexity (how much nesting taxes the
  reader).

Scores are not verdicts handed down from a black box. Every point lost is attributed to a named
rule with a message and a suggestion, so a report is a worklist rather than a grade.

## Why

Code is read far more often than it is written, and the things that make it pleasant to read are
mostly layout: a blank line before a branch, space around a long argument list, a gap between
logical groups, and a comment on the one function that is genuinely hard to follow.

Those things are invisible to every existing metric. Cyclomatic complexity will happily call a
flat, unreadable function simple; a formatter will happily preserve a 200-line function with no
blank lines anywhere. monostyle exists to make the visual properties of code measurable, so that
"this is hard to read" becomes a specific, fixable list.

## Install

```console
nix profile add github:monostyle/monostyle
```

Or build from source:

```console
cargo build --release --package monostyle
```

## Usage

```console
monostyle check [PATH]...          # analyze a directory, file, or list of paths
monostyle check --explain          # list every finding with its explanation
monostyle check --units            # include per-function scores
monostyle check --format json      # machine-readable output
monostyle check --fail-under 75    # exit non-zero below a threshold
monostyle rules                    # list every rule
monostyle config                   # print the effective configuration
```

Useful flags:

| Flag | Effect |
| --- | --- |
| `--strict` | Lower every threshold's tolerance |
| `--lenient` | Raise every threshold's tolerance |
| `--disable RULE` | Turn one rule off |
| `--top N` | Show only the worst N functions |
| `--max-unit-score S` | List only functions scoring below `S` |
| `--no-ignore` | Ignore `.gitignore` |

## What it measures

### Readability

| Rule | What it catches |
| --- | --- |
| `blank-line-before-control-flow` | An `if`/`for`/`while` crowded against the statement above |
| `blank-line-before-return` | A `return` buried against the code above it |
| `group-separation` | A long run of statements with no blank lines between groups |
| `excessive-indentation` | Lines indented past the readable limit |
| `deep-nesting` | Control flow nested past the configured depth |
| `long-parameter-list` | An argument list that should be split across lines |
| `overlong-line` | A line wider than the readable limit |
| `oversized-unit` | A function too long to hold in your head |
| `oversized-file` | A file too large to navigate |
| `mixed-indentation` | A file that indents with both tabs and spaces |
| `comment-required-on-complex-unit` | A complex function with no explanation |
| `comment-explains-why` | **Credit** for a comment that explains reasoning |
| `comment-narrates-code` | A comment that restates what the code already says |
| `excessive-comments` | More commentary than the code can carry |
| `thin-documentation` | A doc block that lists structure without explaining purpose |

### Complexity

| Rule | What it catches |
| --- | --- |
| `cyclomatic-per-unit` | A function with too many independent paths to test |
| `cognitive-per-unit` | A function that is hard to follow, reported with its nesting penalty |
| `cyclomatic-per-file` | A file dense with decisions |

### Markdown

Code inside documentation is scored too, because README examples are what people copy.

| Rule | What it catches |
| --- | --- |
| `fence-readability` | A cramped code example inside a fence |
| `fence-without-language` | A fence with no language tag |
| `fence-language-unknown` | A fence naming a language monostyle does not know |
| `prose-run` | A wall of prose with no structure to break it up |
| `skipped-heading-level` | A heading level that skips a step, breaking the outline |
| `no-title` | A document that does not start with a top-level heading |

## Supported languages

monostyle reads 23 languages. Adding one is a data edit rather than a code change, because
languages are described by profiles rather than by parsers.

Rust, C, C++, C#, Java, JavaScript, Kotlin, Mozjs, Python, TypeScript, TSX, **Dart**, Go, Swift,
Ruby, PHP, Scala, Shell, Lua, Elixir, Haskell, Nix, and Markdown.

The first eleven match what [`rust-code-analysis`] supports, so numbers from the two tools are
comparable. Dart is included because it is the language this tool was built for. The remaining
ten cover widely used languages that project does not reach.

[`rust-code-analysis`]: https://github.com/mozilla/rust-code-analysis

## How scoring works

Findings carry a weight; severity scales it into a penalty. Penalties sum per category and are
normalized by code volume into a penalty density — findings per 100 lines — so a large
well-written file is not punished for its size. Density maps to 0–100 through exponential decay:

```
score = 100 * 2 ^ (-density / half_life)
```

The half-life is the density at which a category scores exactly 50, which makes the whole curve
tunable with one readable number (see `[scoring] half-life`).

Good comments earn **negative** penalties. That is how a well-placed explanation raises a score:
the credit offsets other penalties inside the same density number, so one number always explains
the result.

## Configuration

monostyle reads `monostyle.toml` from the analyzed directory or any parent:

```toml
[scoring]
half-life = 12.0

[rules]
# Only the fields you set are changed; everything else keeps its default.
max-nesting-depth = 3
max-parameters-inline = 3
max-cyclomatic-per-unit = 10
max-cognitive-per-unit = 15
comment-required-above-cognitive = 10
disabled-rules = ["readability/excessive-comments"]
```

Run `monostyle config` to print every available setting with its default.

## Design

The architecture, and why a profile-driven lexer was chosen over tree-sitter, is documented in
[ARCHITECTURE.md](./ARCHITECTURE.md). The short version: readability is a layout metric, so the
tool needs a trustworthy tokenizer over comments and string literals rather than a full parse
tree, and the tokenizer's correctness is guarded by the heaviest test suite in the repository.

## Examples

- [`examples/good/rust.rs`](./examples/good/rust.rs) scores 100 for both categories.
- [`examples/bad/rust.rs`](./examples/bad/rust.rs) scores 0 for both.

Both are real inputs to the test suite, so the separation between them is enforced rather than
asserted.

## License

Unlicense.
