# Installation

<!-- {=installation} -->

```console
cargo install monostyle
```

Or build from source:

```console
cargo build --release --package monostyle
```

<!-- {/installation} -->

## From npm

```console
npm install -g @monostyle-rs/cli
```

## Usage

<!-- {=usageExamples} -->

```console
monostyle check [PATH]...          # analyze a directory, file, or list of paths
monostyle fix [PATH]...            # apply every fixable finding
monostyle fix . --dry-run          # show what would change
monostyle check . --units          # include per-function scores
monostyle check . --explain        # list every finding with its explanation
monostyle check . --format json    # machine-readable output
monostyle check . --fail-under 75  # exit non-zero below a threshold
monostyle rules                    # list every rule
monostyle config                   # print the effective configuration
```

<!-- {/usageExamples} -->
