# Configuration

monostyle reads `monostyle.toml` from the analyzed directory or any parent. Only the keys you set
are changed; everything else keeps its default.

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
comment-required-above-cognitive = 10
disabled-rules = ["readability/excessive-comments"]

[rules.ignore]
patterns = ["**/*.spec.ts", "crates/legacy/**"]
generated = true
```

<!-- {/configExample} -->

Run `monostyle config` to print every available key with its current value. The output is valid TOML
and can be pasted into a `monostyle.toml` file as a starting point.

## Unknown keys

An unrecognized key is an error rather than a warning. A typo in a threshold name would otherwise be
silently accepted, and the project would score against settings nobody chose.
