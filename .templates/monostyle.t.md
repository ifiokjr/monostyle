<!-- {@projectOverview} -->

`monostyle` scores the complexity and readability of a codebase, a file, or a function, out of 100.
Every point lost is traced to a named rule with an explanation and a suggested fix, so a report is a
worklist rather than a grade.

**Readability** measures how the code looks: whether complex sections have room to breathe, whether
sequential control flow is separated, whether deep nesting has been flattened, and whether comments
explain the hard parts.

**Complexity** measures how hard the code is to follow and to test: cyclomatic complexity (how many
independent paths exist) and cognitive complexity (how much nesting taxes the reader).

<!-- {/projectOverview} -->

<!-- {@projectWhy} -->

Code is read far more often than it is written, and the things that make it pleasant to read are
mostly layout: a blank line before a branch, space around a long argument list, a gap between
logical groups, and a comment on the one function that is genuinely hard to follow.

Those things are invisible to every existing metric. Cyclomatic complexity will happily call a flat,
unreadable function simple; a formatter will happily preserve a 200-line function with no blank
lines anywhere. monostyle exists to make the visual properties of code measurable, so that "this is
hard to read" becomes a specific, fixable list.

<!-- {/projectWhy} -->

<!-- {@installation} -->

```console
cargo install monostyle
```

Or build from source:

```console
cargo build --release --package monostyle
```

<!-- {/installation} -->

<!-- {@usageExamples} -->

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

<!-- {@readabilityRules} -->

| Rule                               | What it catches                                             |
| ---------------------------------- | ----------------------------------------------------------- |
| `blank-line-before-control-flow`   | An `if`/`for`/`while` crowded against the statement above   |
| `blank-line-before-return`         | A `return` buried against the code above it                 |
| `group-separation`                 | A long run of statements with no blank lines between groups |
| `excessive-indentation`            | Lines indented past the readable limit                      |
| `deep-nesting`                     | Control flow nested past the configured depth               |
| `long-parameter-list`              | An argument list that should be split across lines          |
| `overlong-line`                    | A line wider than the readable limit                        |
| `oversized-unit`                   | A function too long to hold in your head                    |
| `oversized-file`                   | A file too large to navigate                                |
| `mixed-indentation`                | A file that indents with both tabs and spaces               |
| `comment-required-on-complex-unit` | A complex function with no explanation                      |
| `comment-explains-why`             | **Credit** for a comment that explains reasoning            |
| `comment-narrates-code`            | A comment that restates what the code already says          |
| `excessive-comments`               | More commentary than the code can carry                     |
| `thin-documentation`               | A doc block that lists structure without explaining purpose |

<!-- {/readabilityRules} -->

<!-- {@complexityRules} -->

| Rule                  | What it catches                                                      |
| --------------------- | -------------------------------------------------------------------- |
| `cyclomatic-per-unit` | A function with too many independent paths to test                   |
| `cognitive-per-unit`  | A function that is hard to follow, reported with its nesting penalty |
| `npath-per-unit`      | A function with too many execution paths                             |
| `exits-per-unit`      | A function that returns from too many places                         |
| `low-maintainability` | A function with a low maintainability index                          |
| `cyclomatic-per-file` | A file dense with decisions                                          |

<!-- {/complexityRules} -->

<!-- {@markdownRules} -->

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

<!-- {@supportedLanguages} -->

Rust, C, C++, C#, Java, JavaScript, Kotlin, Mozjs, Python, TypeScript, TSX, **Dart**, Go, Swift,
Ruby, PHP, Scala, Shell, Lua, Elixir, Haskell, Nix, and Markdown.

The first eleven match what [`rust-code-analysis`](https://github.com/mozilla/rust-code-analysis)
supports, so numbers from the two tools are comparable. Dart is included because it is the language
this tool was built for. The remaining ten cover widely used languages that project does not reach.

<!-- {/supportedLanguages} -->

<!-- {@scoringModel} -->

Findings carry a `weight`; severity scales it into a penalty. Penalties sum per category and are
normalized by code volume into a penalty density — findings per 100 lines — so a large well-written
file is not punished for its size. Density maps to 0–100 through exponential decay:

```
score = 100 * 2 ^ (-density / half_life)
```

The half-life is the density at which a category scores exactly 50, which makes the whole curve
tunable with one readable number.

Good comments earn **negative** penalties. That is how a well-placed explanation raises a score: the
credit offsets other penalties inside the same density number, so one number always explains the
result.

Scores are aggregated by **line-weighted mean**, the same way test coverage is aggregated. A
ten-line file cannot count as much as a thousand-line file. Three invariants hold, and all three are
enforced by tests: splitting a file does not change the project score, doubling penalty and volume
does not change it, and fifty two-line files cannot outweigh one five-thousand-line file.

<!-- {/scoringModel} -->

<!-- {@configExample} -->

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
comment-required-above-cognitive = 10
disabled-rules = ["readability/excessive-comments"]

[rules.ignore]
patterns = ["**/*.spec.ts", "crates/legacy/**"]
generated = true
```

<!-- {/configExample} -->

<!-- {@impactExample} -->

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

<!-- {/impactExample} -->

<!-- {@autofixExplanation} -->

One rule is auto-fixable: inserting a blank line before a control-flow statement. That is the only
edit guaranteed to survive a formatter — rustfmt, Prettier, Black, and `dart format` all preserve a
blank line between statements and none of them remove one. A fixer that fights the project's
formatter produces a diff the next format run reverts, which is worse than the finding itself.

Every other rule explains itself and leaves the change to you. The fix output shows both: what was
applied, and what still needs a decision, with the suggestion attached.

<!-- {/autofixExplanation} -->

<!-- {@ignoreConfigExample} -->

```toml
[rules.ignore]
patterns = ["**/*.spec.ts", "crates/legacy/**"]
generated = true # skip generated code (the default)
include = ["lib/hand_edited.g.dart"] # always score this one
```

Recognized as generated: `.g.dart`, `.freezed.dart`, `.pb.rs`, `.pb.go`, `_pb2.py`, `.designer.cs`,
`.gen.ts`, `.min.js`, `.bundle.js`, and lock files. Ignored directories include `node_modules`,
`target`, `dist`, `build`, `vendor`, `.venv`, `.dart_tool`, and `__pycache__`.

<!-- {/ignoreConfigExample} -->

<!-- {@changesetWorkflow} -->

A changeset describes one change and the version bump it needs. Write one with:

```console
monochange run change --package monostyle_rules --bump patch --reason "Fix the long-parameter-list rule firing on every call"
```

Or write the file by hand in `.changeset/`:

```markdown
---
"monostyle_rules": patch
---

# Fix the long-parameter-list rule firing on every call

The rule tested argument count and rendered width separately, so any call over forty columns was
reported regardless of how many arguments it had. It now requires both, which is what makes the
finding mean something.
```

The release workflow reads every changeset, computes the next version for each package, and opens a
release pull request. Merging that pull request is what cuts the release, so the schedule is
"whenever a release is worth shipping" rather than a fixed cadence.

## What needs a changeset

A pull request that changes a published package needs one, or the release notes for that version
would not mention the change. Documentation, tests, snapshots, and examples do not: none of them
change what a published package does.

## Trusted publishing

Releases publish with trusted publishing when a verifiable CI identity is available, and fall back
to the `NPM_TOKEN` and `CARGO_REGISTRY_TOKEN` secrets otherwise. The fallback exists because a
package has to exist in a registry before it can be enrolled with a trusted publisher, so the first
publish of anything always uses a token. Set `force_token_auth` when dispatching the publish
workflow to skip the OIDC exchange deliberately.

<!-- {/changesetWorkflow} -->
