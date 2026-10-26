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

Seven rules ask for a file's function-like units, and detection is a structural scan over every line. On a 17,000-line file with 499 functions that was 499 units times seven scans — and each scan re-measured every unit's cyclomatic, cognitive, Halstead, and maintainability figures besides. Detection and the per-unit metrics now run once per file: the rule context holds a `UnitSet` and hands the same measurements to whichever rule asks. The context is a value owned for exactly one file, so there is no cache to key and no way for one file's measurements to reach another — an earlier attempt to share them through a cache keyed on the file's memory address produced exactly that confusion.

### Halstead used linear membership tests

Distinct operators and operands were held in `Vec`s and tested with `iter().any()`, which is quadratic in the token count. They are now `HashSet`s.

### The scanner allocated once per character

Counting allocations with a wrapping global allocator showed the scanner making **64 to 72 heap allocations per line** — about one per character — and allocating 6.3 MB to lex an 82 KB file. None of it was quadratic, which is why the whole-repository timings above looked reasonable; it was simply a constant tax on every character. Three sources accounted for nearly all of it:

- `step_code` read the rest of the line into a `String` for **every code character**, to ask whether a heredoc opened there. Most languages have no heredocs, and in the ones that do the marker starts with one specific character, so the line is now read only when both are true.
- `literal_at` collected a six-character `Vec<char>` lookahead window for every character before checking whether the character could even begin a literal. The window is now a slice of the text already in memory.
- Every character inside a string literal cloned the open `Literal`, whose closing delimiter was an owned `String`. The delimiter borrows from the language profile for every ordinary string now, so the copy touches no heap; only a closer that depends on the opener — a hashed raw string's hash count, a C++ raw string's custom delimiter — is owned.

With those gone, the per-line model was the next layer. Each `LexedLine` owned three `String`s (its text, its leading whitespace, and its masked view) and a `Vec<String>` of keyword names, and the keyword pass tokenized every code line into a second `Vec<String>` plus a joined copy. The line now borrows its text and indentation from the source — `LexedFile` carries the source's lifetime — the keywords are the profile's own `&'static str`s, and the word list is a vector of slices. The masked view is built lazily: it is the text's own prefix until a literal or comment forces it apart, so a plain code line, a comment line, and a line with only a trailing comment never allocate one.

Comment classification normalized every comment body through four allocations (a punctuation-stripped copy, a word list, a joined copy, and a lowercased copy) and the block pass re-copied each line's body to join them; both are single buffers now, and a line's comment body is a slice of the line.

The result on this repository's two largest files: `whitespace.rs` went from 146,116 allocations to 6,822 (64 per line to 3), and `lexer.rs` from 159,766 to 5,268 (72 per line to 2.3), with the bytes allocated down by 2.3×. Plain code lines make exactly one allocation — the word list — and blank lines none.

One latent bug surfaced in the process. The byte range recorded for every CRLF-terminated line ended one byte early, and a blank CRLF line's range ran backwards, because the close stepped back for a carriage return the scan had not yet passed. The old model copied each line's text as it scanned, so the wrong range never showed in the text; the borrowed model slices the text from the range, which made the fixer's own tests fail on every CRLF fixture. The range is now exact, and the regression test asserts the slice equals the text rather than merely excluding the terminator.

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
