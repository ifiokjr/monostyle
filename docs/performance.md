# Performance

Measured on an Apple Silicon laptop against four repositories, three of them real projects. Each number is the best of three runs of `monostyle check --quiet`, which is what a person or a CI job would actually invoke.

| Repository | Files | Lines of code | Time  |
| ---------- | ----- | ------------- | ----- |
| monostyle  | 87    | 16,529        | 0.12s |
| mdt        | 228   | 35,749        | 0.13s |
| monochange | 318   | 193,900       | 0.42s |
| pina       | 2,198 | 232,417       | 1.08s |

A 2,198-file repository with 232,000 lines of code completes in about a second.

## What made it fast

Three quadratic paths were removed. Each was invisible on a small file and dominated on a large one.

### The scanner copied the file on most characters

Every step of the scan collected the **entire remaining file** into a `String`, purely to test whether a short delimiter matched:

```rust
let rest: String = characters[index..].iter().collect();

if self.profile.block_comment_at(&rest) { ... }
```

On a 7,139-line file this alone meant gigabytes of allocation, and the file **did not finish within sixty seconds**. Delimiter matching now reads a bounded six-character window, which answers the same question in constant work per character. That file analyzes in 2.07 seconds.

The same pattern appeared in `literal_at`, which ran on a large fraction of all characters: `l`, `b`, `u`, `f`, and `r` are ordinary identifier letters and also string prefixes, so the lookahead was triggered constantly. It was 65 seconds of the 66-second runtime. Bounding it to the same six-character window is what took the file from 65s to 0.57s.

### Unit detection ran seven times per file

Seven rules ask for a file's function-like units, and detection is a structural scan over every line. On a 17,000-line file with 499 functions that was 499 units times seven scans. Detection and the per-unit metrics now run once and are memoized, keyed by the unit's line range so two functions with the same name in different places cannot be confused.

### Halstead used linear membership tests

Distinct operators and operands were held in `Vec`s and tested with `iter().any()`, which is quadratic in the token count. They are now `HashSet`s.

## The disk cache, and why it was removed

The scanner's line model was once cached on disk under `target/monostyle`, keyed by path, modification time, size, language, and a schema number, so a repeated run could skip the scan. It was removed because measurement showed it cost more than it saved:

- Restoring a line model is more work than producing it. An entry stores the raw text, the masked text, and the indentation of every line — around five times the source's bytes as JSON — and deserializing plus re-cloning all of it costs more than scanning the original file. On a 671-file, 28 MB corpus, a warm run took 3.2s against 1.5s for a cold run: the cache made the tool **twice as slow**. The crossover was visible even in the table above before the removal: pina's warm run (1.12s) was slower than its cold run (1.08s), and only the two smallest repositories won, by tens of milliseconds.
- The scanner is already fast — a full cold analysis runs at roughly 20 MB per second across the whole pipeline — so there was little for a cache to save.
- The invalidation key could serve stale results. Entries were keyed by modification time and size, which two same-length edits within one timestamp can defeat on coarse-mtime filesystems, and the scanner fingerprint that was meant to invalidate entries across development rebuilds was derived from compile-time constants, so a rebuilt scanner with changed behavior kept serving entries written by the old one. A stale hit reports findings that do not match the source, which is the worst failure a cache can have.

A cache could only pay if restoring the line model were cheaper than scanning the file. Since the line model is necessarily larger than the file, it cannot be, and the honest fix was deletion rather than a faster serializer.

## Measuring it yourself

Every pull request is benchmarked automatically: the `benchmark` workflow builds release binaries of the PR and of the commit it branched from, runs both over this repository's own tree and an eightfold copy of it, and posts a comparison comment with a merge recommendation. The comment is updated in place on every push, and re-running the workflow regenerates it.

To measure locally:

```console
cargo build --release --package monostyle
time ./target/release/monostyle check /path/to/repository --quiet
```
