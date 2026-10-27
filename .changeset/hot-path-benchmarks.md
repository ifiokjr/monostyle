---
"monostyle": patch
---

# Benchmarks and an allocation ceiling for the hot paths

`cargo bench --package monostyle` runs Criterion micro-benchmarks for each stage a file goes through — lexing as bytes per second in ten languages, unit measurement for each block style, the rule set, and the whole analysis — over a corpus written to exercise the scanner's hardest paths. The pull-request benchmark workflow runs two new scenarios on the same corpus: a mixed-language tree, which the all-Rust corpora never reached, and a single seven-thousand-line file, the size at which every quadratic path so far first showed.

The scanner is also held to an allocation ceiling per line for each line shape, counted through a wrapping global allocator, so a per-character allocation returning to it fails a test rather than the next profiling session. The benches and their corpus are excluded from the published crate.
