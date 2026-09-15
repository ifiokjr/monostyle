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

## Monorepo support

Cargo, npm, pnpm, and Dart workspaces are detected from their manifests, and every file is
attributed to the package that owns it. Each package is scored independently, weighted by its own
lines of code, so the report can say which package is the problem rather than only that the
repository scored 72.

```console
$ monostyle check .
  ...
packages

     read    cplx  package
     43.5    40.4  pina_lints cargo · crates/pina_lints · 9,856 LOC
     33.1    37.0  pina_macros cargo · crates/pina_macros · 6,031 LOC
     18.9    46.1  pina_abi cargo · crates/pina_abi · 3,177 LOC
```

## Finding what to fix

A score tells you something is wrong. The impact table tells you what to do about it, ranked by how
much of the score each rule accounts for:

```console
$ monostyle check . --units
readability: what is costing you points

  ██████░░░░  61.2%  readability/blank-line-before-control-flow  (4,181 findings)
             crates/example/src/main.rs:91 [minor]
             `if` follows the previous statement with no blank line between them
             -> Add a blank line before this statement so the reader can treat it as
             a separate decision rather than part of the previous block.
```

Each entry names its worst offender as `path:line`, and the report ends with the single
highest-value fix:

```console
start here
  Fixing readability/blank-line-before-control-flow at crates/example/src/main.rs:91 would
  recover 30.7% of the available points.
```

The percentage is a share of the *whole* score, not of one category, so it is the recovery that fix
can actually deliver.

## Scoring is weighted by lines of code

A project's score is not the average of its files' scores. It is a line-weighted mean of their
penalties, the way coverage is aggregated, so a ten-line file cannot count as much as a
thousand-line file.

Three invariants hold, and all three are enforced by tests:

- Splitting a file into two does not change the project score.
- Doubling both penalty and volume does not change the score.
- Fifty two-line files cannot outweigh one five-thousand-line file.

## npm

```console
npm install -g @monostyle-rs/cli
```

The launcher resolves a prebuilt binary for the current platform, falling through across libc
variants so Alpine and glibc systems both work. `@monostyle-rs/skill` carries the agent guidance for
using the tool.

## CI

The workflows verify builds, tests, and the tool's own quality:

| Job | What it checks |
| --- | --- |
| `lint` | Formatting and clippy with warnings denied |
| `test` | The suite on Linux, macOS, and Windows |
| `lexer-correctness` | The scanner's adversarial trivia suite, run on its own |
| `coverage` | A 70% line floor, so deleting tests fails the build |
| `dogfood` | The repository's own readability floor |
| `npm` | Every manifest agrees and the launcher can resolve each package |
| `docs` | Documentation builds without warnings |
| `package-check` | Every crate packages cleanly |
| `security` | Advisories, licenses, and workflow scanning |
| `real-world` | Analysis of other repositories, asserting a well-formed report |

## Auto-fix

```console
monostyle fix .                # apply every fixable finding
monostyle fix . --dry-run      # show what would change
monostyle fix . --rule readability/blank-line-before-control-flow
```

One rule is auto-fixable: inserting a blank line before a control-flow statement. That is the only edit
guaranteed to survive a formatter — rustfmt, Prettier, Black, and `dart format` all preserve a blank line
between statements and none of them remove one. A fixer that fights the project's formatter produces a
diff the next format run reverts, which is worse than the finding itself.

Every other rule explains itself and leaves the change to you. The fix output shows both: what was
applied, and what still needs a decision, with the suggestion attached.

```console
$ monostyle fix . --dry-run

src/lib.rs — would fix 2 findings:
  + src/lib.rs:5  readability/blank-line-before-control-flow
      `if` follows the previous statement with no blank line between them

  ! 1 finding need a decision:
    src/lib.rs:8  readability/magic-number
        the literal `4096` carries meaning without a name
        -> Name this value as a constant so the reader knows what it represents
```

## Ignoring files

Generated files are skipped by default, along with dependency caches and build output. Generated code is
not written for a human, so its findings are not actionable: on one repository measured here they were
more than half the total penalty, which made the score describe a code generator rather than the code.

```console
monostyle check . --include-generated    # score generated code too
```

A `monostyle.toml` names what else to skip, in the same syntax as `.gitignore`:

```toml
[rules.ignore]
patterns = ["**/*.spec.ts", "crates/legacy/**"]
generated = false                    # measure generated code after all
include = ["lib/hand_edited.g.dart"] # always score this one
```

```console
monostyle check . --no-ignore        # read no ignore files at all
```

Recognized as generated: `.g.dart`, `.freezed.dart`, `.pb.rs`, `.pb.go`, `_pb2.py`, `.designer.cs`,
`.gen.ts`, `.min.js`, `.bundle.js`, and lock files. Ignored directories include `node_modules`, `target`,
`dist`, `build`, `vendor`, `.venv`, `.dart_tool`, and `__pycache__`.

## Line length

The limit is configurable, because it is a formatting decision and the formatter is the authority:

```toml
[rules]
max-line-width = 100
severe-line-width-ratio = 1.35   # columns past the limit before a finding is severe
```

Two exclusions matter in practice. **Markdown prose is never measured** — a paragraph is wrapped by
whoever wrote it, and a table or a long URL legitimately exceeds any code limit. Only fenced code inside a
Markdown file is measured. And **a line that cannot be broken is not reported**: a long string literal or
URL has nowhere to wrap to, so a finding would ask for something the language does not allow.

## Performance

| Repository | Files | Lines of code | Time |
| --- | --- | --- | --- |
| mdt | 228 | 35,749 | 0.13s |
| monochange | 318 | 193,900 | 0.42s |
| pina | 2,198 | 232,417 | 1.08s |

A 2,198-file repository with 232,000 lines of code completes in about a second. See
[docs/performance.md](./docs/performance.md) for what made it fast and what the cache does.

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
