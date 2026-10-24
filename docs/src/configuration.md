# Configuration

monostyle reads `monostyle.toml` from the analyzed directory or any parent. Only the keys you set are changed; everything else keeps its default.

<!-- {=configExample} -->

```toml
# The gate for the whole run: `monostyle check` exits non-zero when a category
# scores below it. A bare number sets both categories to the same value.
[project]
fail-under = { readability = 85, complexity = 80 }

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

# A per-file floor, not the project gate: every file must individually score at
# or above these values.
[fail-under]
readability = 70.0
complexity = 70.0
```

`max-statements-per-group` and `max-consecutive-blank-lines` are the two halves of the same instruction: the first says when a run of statements needs a gap, the second says how large that gap may grow. `max-consecutive-blank-lines` is a floor rather than a cap for languages with their own convention — PEP 8 asks for two blank lines before a top-level Python definition, and `dart format` does the same — so the allowance is whichever is larger, and only a longer run is reported.

<!-- {/configExample} -->

Run `monostyle config` to print every available key with its current value. The output is valid TOML and can be pasted into a `monostyle.toml` file as a starting point.

## Score gates and floors

`fail-under` means two different things depending on where it appears, which is why the two keys have different names in the configuration output.

Under `[project]` it is the gate for the whole run. `monostyle check` compares the project's aggregate readability and complexity against it and exits non-zero when either falls short — exactly what `--fail-under` enforces on the command line. The command-line flags override it for one run: `--fail-under` sets both categories, `--fail-under-readability` and `--fail-under-complexity` set one each, and a flag set this way replaces the configured gate rather than layering onto it.

At the top level, or inside a `[[section]]`, `fail-under` is a per-file floor. Every file must individually score at or above it, and a file that falls short is named in the output. A gate of 80 fails one run; a floor of 80 can fail dozens of files that are individually fine but not exceptional, so the two are not interchangeable.

## Unknown keys

An unrecognized key is an error rather than a warning. A typo in a threshold name would otherwise be silently accepted, and the project would score against settings nobody chose.
