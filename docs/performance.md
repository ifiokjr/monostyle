# Performance

Measured on an Apple Silicon laptop against four repositories, three of them real projects. Each number is the best of three runs of `monostyle check --quiet`, which is what a person or a CI job would actually invoke.

| Repository | Files | Lines of code | Cold  | Warm  |
| ---------- | ----- | ------------- | ----- | ----- |
| monostyle  | 87    | 16,529        | 0.12s | 0.07s |
| mdt        | 228   | 35,749        | 0.13s | 0.12s |
| monochange | 318   | 193,900       | 0.42s | 0.36s |
| pina       | 2,198 | 232,417       | 1.08s | 1.12s |

A 2,198-file repository with 232,000 lines of code completes in about a second.

"Warm" means a second run with the cache populated. It is faster only where the cache can be written: monostyle and monochange have a `target` directory, because they have been built, and the cache lives under it. mdt and pina had no build tree in the checkout that was measured, so discovery found nowhere to cache — which is the documented behavior, and the reason those three columns show no improvement.

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

## The cache

The cache stores the **lexed** result rather than the final report. Rules and thresholds change far more often than source does: caching the report would invalidate on every configuration change, while the line model stays valid across all of them and only the cheap rule pass repeats.

An entry is keyed by path, modification time, size, language, and a schema number. Any mismatch is a miss, which is the safe direction — a stale hit would report scores for code that no longer exists. A corrupt entry is discarded rather than reported, because a cache is an optimization and failing an analysis because one went wrong would be the wrong trade.

## Measuring it yourself

```console
cargo build --release --package monostyle
time ./target/release/monostyle check /path/to/repository --quiet
```

The cold and warm figures come from removing `target/monostyle` between runs. To see whether the cache is engaging, check that the directory exists afterward; if it does not, the checkout has no build tree for discovery to find.
