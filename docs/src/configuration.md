# Configuration

monostyle reads `monostyle.toml` from the analyzed directory or any parent. Only the keys you set are changed; everything else keeps its default.

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

Run `monostyle config` to print every available key with its current value. The output is valid TOML and can be pasted into a `monostyle.toml` file as a starting point.

## Unknown keys

An unrecognized key is an error rather than a warning. A typo in a threshold name would otherwise be silently accepted, and the project would score against settings nobody chose.
