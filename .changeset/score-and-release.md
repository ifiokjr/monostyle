---
"monostyle": minor
"monostyle_core": minor
"monostyle_languages": minor
"monostyle_lexer": minor
"monostyle_markdown": minor
"monostyle_metrics": minor
"monostyle_rules": minor
"@monostyle_rs/cli": minor
"@monostyle_rs/cli_darwin_arm64": minor
"@monostyle_rs/cli_darwin_x64": minor
"@monostyle_rs/cli_linux_arm64_gnu": minor
"@monostyle_rs/cli_linux_arm64_musl": minor
"@monostyle_rs/cli_linux_x64_gnu": minor
"@monostyle_rs/cli_linux_x64_musl": minor
"@monostyle_rs/cli_win32_arm64_msvc": minor
"@monostyle_rs/cli_win32_x64_msvc": minor
"@monostyle_rs/skill": minor
---

# Read the config file, and ship releases from changesets

The configuration file was only read when `--config` was passed, so a checked-in `monostyle.toml`
had no effect. Sections, per-path floors, and every threshold were silently inert, which meant the
scores a project saw were not the ones it had asked for.

Sections are now also matched when a path carries `.` or `..` components, so `monostyle check .`
applies them rather than skipping them.

`monostyle fix` honours ignored sections instead of rewriting files that `check` deliberately skips.
It was rewriting the deliberately-bad scoring fixtures, changing the inputs the scoring tests assert
against.

One false positive is fixed. `readability/empty-handler` matched the handler keyword anywhere on a
line, so a function named `complexity_rules_catch_a_hard_function` was reported as discarding an
error it never caught. The keyword now has to be a word of its own.

Three functions were flattened after the tool named them as the largest single costs in their files:
the glob matcher, the cognitive-complexity line loop, and the overlong-line rule. Each was verified
to produce byte-identical findings before and after.

`score_color` gave the same green to `good` and `excellent`, so the band the colour exists to convey
was invisible above 75. The grade thresholds are now shared constants, and an excellent score is
distinct.

Releases are driven by changesets and published with trusted publishing where available, falling
back to a registry token for a package that has not been enrolled yet.
