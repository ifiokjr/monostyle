# Ignoring files

Generated files are skipped by default, along with dependency caches and build output. Generated
code is not written for a human, so its findings are not actionable.

## Configuration

<!-- {=ignoreConfigExample} -->

```toml
[rules.ignore]
patterns = ["**/*.spec.ts", "crates/legacy/**"]
generated = true # skip generated code (the default)
include = ["lib/hand_edited.g.dart"] # always score this one
```

Recognized as generated: `.g.dart`, `.freezed.dart`, `.pb.rs`, `.pb.go`, `_pb2.py`, `.designer.cs`,
`.gen.ts`, `.min.js`, `.bundle.js`, and lock files. Ignored directories include `node_modules`,
`target`, `dist`, `build`, `vendor`, `.venv`, `.dart_tool`, and `__pycache__`.

<!-- {/ignoreConfigExample} -->

## Command-line overrides

```console
monostyle check . --include-generated    # score generated code too
monostyle check . --no-ignore            # read no ignore files at all
```

## Why generated files are excluded by default

On one real repository, generated files were 784,000 of 822,000 lines and produced 615,000 findings.
Those findings described the generator rather than the project, which made the score unactionable.
Excluding them by default is what makes the tool useful on a repository with a large code-generation
surface.

A project that ships generated code as part of its public surface can turn the exclusion off with
`generated = false` in `monostyle.toml`.
