# Reference

Every flag, configuration key, and output format.

## Commands

### `monostyle check [PATH]...`

Analyzes paths. Defaults to the current directory.

| Flag | Effect |
| --- | --- |
| `--format text\|json\|toml` | Output format. `text` is the default and is meant for humans. |
| `--explain` | List every finding with its location, message, and suggested fix. |
| `--units` | Add per-function scores, sorted worst first. |
| `--top N` | With `--units`, show only the worst N functions. |
| `--max-unit-score S` | With `--units`, show only functions scoring below S. |
| `--language LANG` | Restrict to a language. Repeatable. |
| `--output FILE` | Write the report to a file. |
| `--fail-under SCORE` | Exit 1 when either score is below the threshold. |
| `--no-ignore` | Walk ignored files too. |
| `--quiet` | Suppress the summary line on stderr. |

### Global flags

| Flag | Effect |
| --- | --- |
| `--config FILE` | Use a specific configuration file. |
| `--disable RULE` | Turn a rule off. Repeatable. |
| `--strict` | Halve every threshold's tolerance. |
| `--lenient` | Double every threshold's tolerance. |
| `--color` | Force colour even when stdout is not a terminal. |
| `--no-color` | Disable colour. `NO_COLOR` and `TERM=dumb` are also honoured. |

### `monostyle rules [RULE]`

Lists every rule, or shows one rule's description.

### `monostyle config`

Prints the effective configuration, including anything loaded from a file.

## Configuration

monostyle reads `monostyle.toml` from the analyzed directory or any parent. Only the keys you set are
changed; everything else keeps its default.

```toml
[scoring]
# Penalty density per 100 lines that yields a score of 50.
# Raise it to be more forgiving; lower it to be stricter.
half-life = 12.0
# Lines used as the denominator floor when normalizing a small file.
min-normalization-lines = 20.0

[rules]
# --- whitespace ---
require-blank-line-before-control-flow = true
min-blank-lines-between-control-flow = 1
require-blank-line-before-return = true
require-group-separation = true

# --- structure ---
max-nesting-depth = 3
max-parameters-inline = 3
max-indent-width = 24

# --- comments ---
require-comment-on-complex-units = true
comment-required-above-cognitive = 10
penalize-narrating-comments = true
max-comment-ratio = 0.6

# --- complexity ---
max-cyclomatic-per-unit = 10
max-cognitive-per-unit = 15
max-unit-lines = 80
max-file-lines = 600

# --- markdown ---
score-markdown-fences = true
max-prose-run = 12

# --- rule control ---
disabled-rules = ["readability/excessive-comments"]
```

Run `monostyle config` to print every key with its current value.

## JSON output

`--format json` emits the full report, including data the text output summarizes:

```jsonc
{
  "files": [
    {
      "path": "src/lib.rs",
      "language": "rust",
      "code_lines": 120,
      "readability": { "value": 82.5, "penalty": 4.0, "density": 3.3 },
      "complexity": { "value": 91.0, "penalty": 1.5, "density": 1.25 },
      "findings": [
        {
          "rule": "readability/deep-nesting",
          "category": "readability",
          "severity": "major",
          "span": { "start_line": 42 },
          "message": "`if` sits at nesting level 4, over the limit of 3",
          "suggestion": "Flatten this with an early return...",
          "weight": 1.5
        }
      ],
      "units": [
        {
          "name": "handle_request",
          "start_line": 10,
          "cyclomatic": 12,
          "cognitive": 18,
          "nesting_penalty": 7,
          "max_nesting": 4,
          "readability": { "value": 55.0 },
          "complexity": { "value": 48.0 }
        }
      ]
    }
  ],
  "packages": [
    {
      "package": { "name": "my-crate", "ecosystem": "cargo", "directory": "crates/my-crate" },
      "code_lines": 4200,
      "readability": { "value": 78.0 },
      "complexity": { "value": 84.0 }
    }
  ],
  "readability": { "value": 77.2, "penalty": 210.0, "density": 8.1 },
  "complexity": { "value": 78.2, "penalty": 175.0, "density": 7.4 },
  "code_lines": 6988,
  "skipped": []
}
```

## Supported languages

Rust, C, C++, C#, Java, JavaScript, Kotlin, Mozjs, Python, TypeScript, TSX, Dart, Go, Swift, Ruby,
PHP, Scala, Shell, Lua, Elixir, Haskell, Nix, and Markdown.

Languages are described by data profiles rather than parsers, so a language is added by editing
`crates/monostyle_languages/src/catalog.rs` rather than by writing analysis code.

## Markdown

Code inside fenced blocks is scored as its own language, with findings mapped back to the Markdown
file's line numbers so a reader can jump to the exact fence. Documentation is where copyable code
lives, so it is held to the same layout standard as source.

Complexity rules are deliberately excluded from fences: a documentation example that walks through a
messy state on purpose is doing its job, and penalizing it would push authors toward hiding the
complexity they are explaining.
