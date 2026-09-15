# Architecture

## The decision: profile-driven lexer first, tree-sitter as a deliberate phase two

monostyle needs to read source in ~24 languages. There were two ways to do that, and the
choice shapes the whole project.

**Chosen: a profile-driven lexer.** Each language is a data profile — comment syntax,
string delimiters, interpolation style, decision keywords, block style. One scanner
serves every language, and adding a language is a data edit rather than a code change.

**Deferred: tree-sitter grammars**, loaded as Wasm at runtime, behind a cargo feature.

### Why the profile lexer, stated honestly

The tempting argument against tree-sitter is binary size, and it is wrong. Helix and Zed
ship small binaries *while* using tree-sitter because they package grammars externally
(Zed fetches Wasm grammars on demand; Helix keeps a runtime directory). For a developer
CLI, a 30 MB binary would not matter either — `rust-code-analysis` statically links around
ten grammars and ships fine.

The real costs are different:

1. **Build latency and version coupling.** Generated parser C is large; the C++ and
   TypeScript grammars take tens of seconds each to compile cold. Thirty grammars means a
   C toolchain in the build graph, minutes of cold compilation, and thirty separate
   parser-to-runtime ABI relationships to keep aligned. In a devenv setup aiming for
   reproducibility across macOS and Linux, that tax is paid on every fresh environment.

2. **Tree-sitter only replaces half the work.** Our metric is layout-first. Whether there
   is a blank line before an `if`, whether logical groups are separated, how dense the
   comments are, how deep the indentation runs — these are line- and token-level
   properties. Tree-sitter hands back byte ranges; detecting blank lines still requires a
   separate pass over the source with its own trivia handling. Paying for thirty C
   toolchains to still write the layout pass is a poor trade.

3. **Two coverage gaps point the same way.** Dart — the language this project exists for —
   is absent from `rust-code-analysis` and has a less mature grammar than the flagship
   ones. Markdown code fences need per-fence grammar injection under either design.

### What this costs us, and how we pay for it

The honest risk is **parsing correctness in exactly one place: comments and string
literals**. A hand-written scanner gets literal syntax wrong, and those errors corrupt the
scores we exist to produce. The concrete failure modes, all of which have dedicated
regression tests:

| Case | Why naive scanning breaks |
| --- | --- |
| Dart `'''`/`"""` and `$`/`${}` interpolation | The closing delimiter is three characters, and interpolation can nest quotes |
| Rust `r#"…"#` with arbitrary hash counts | The close delimiter depends on an open-time count |
| Rust nested block comments | `/* /* */ */` closes after the second `*/`, not the first |
| JS regex versus division | `/` is a regex only in operand position; a misread swallows the line |
| JS template literals with nested `${…}` | Interpolation can contain braces, strings, and further templates |
| Python f-strings | `{…}` nests (and the rules changed in 3.12); `{{` is a literal brace |
| Shell heredocs | The body is data until a line equal to the delimiter; `<<'EOF'` disables interpolation |
| Nix `''…''` indented strings | `''$` and `''${` are escapes, so `''` alone does not always close |

So the correctness effort is concentrated deliberately: the scanner is a pushdown state
machine with an explicit context stack, and `crates/monostyle_lexer/tests/trivia.rs` is the
heaviest test suite in the repository.

### Why the decision is reversible

The scanner's contract is a line-oriented model: indentation, blankness, comment intent,
decision points, parameter spans. Tree-sitter can be added later behind a feature flag
without touching the rules, because the rules consume that model rather than tokens.
The honest framing of the phase-two upgrade is not "tree-sitter is heavy" but: **cognitive
complexity's nesting penalty is the one metric that genuinely benefits from real structure,
and the right way to add it is Wasm-loaded grammars, not thirty statically linked C
parsers.** That upgrade should land once the scoring rules have settled.

## Crate layout

```
monostyle_core       domain types: Span, Finding, Category, Severity, Score, Language
monostyle_languages  language profiles as data
monostyle_lexer      the scanner and its line model
monostyle_metrics    cyclomatic + cognitive complexity, unit detection
monostyle_rules      the rule set
monostyle_markdown   Markdown fence extraction and prose structure
monostyle            the CLI
```

Dependencies flow one direction: `core` is depended on by everything and depends on
nothing; `rules` sits at the top of the library stack. This is enforced by `cargo deny`
and by the workspace having no cycles.

## Scoring model

Findings carry a `weight`; `Severity` scales it into a penalty. Penalties sum per
category and are normalized by code volume into a penalty density (per 100 lines). Density
maps to 0–100 through exponential decay with a configurable half-life:

```
score = 100 * 2 ^ (-density / half_life)
```

The half-life is the density at which a category scores exactly 50, which makes the whole
curve tunable with one readable number. Density rather than raw count means a large,
well-written file is not punished for its size.

Findings may carry a **negative** weight. That is how a well-placed why-comment earns
credit: it offsets other penalties inside the same density number, so one number always
explains the final score.
