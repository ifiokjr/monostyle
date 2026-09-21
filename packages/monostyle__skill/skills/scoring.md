# Scoring

How the two scores are computed, and the reasoning behind each choice.

## Two scores, not one

**Readability** measures how the code looks: whether complex sections have room to breathe, whether sequential control flow is separated, whether deep nesting has been flattened, and whether comments explain the hard parts.

**Complexity** measures how hard the code is to follow and to test: cyclomatic complexity (how many independent paths exist) and cognitive complexity (how much nesting taxes the reader).

They are kept separate because they answer different questions and can disagree. A flat function with two hundred sequential statements is simple to test and exhausting to read. A tight recursive parser is hard to test and perfectly readable to someone who knows the grammar. Collapsing them into one number would hide exactly the cases where the two disagree.

`overall` is their mean, and is offered only as a convenience for a dashboard.

## From findings to a number

Rules emit findings. Each finding carries a `weight`, which severity scales into a `penalty`:

| Severity   | Multiplier                 |
| ---------- | -------------------------- |
| `info`     | 0 (reported, never scored) |
| `minor`    | 1.0                        |
| `major`    | 2.5                        |
| `critical` | 5.0                        |

Penalties are summed per category and divided by code volume to produce a **penalty density** — findings per 100 lines of code. Density, not raw count, is what the score uses, so a large well-written file is not punished for its size.

Density maps to 0–100 through exponential decay:

```text
score = 100 × 2 ^ (-density / half_life)
```

The half-life is the density at which a category scores exactly 50. That makes the whole curve tunable with one readable number: raising it makes the tool more forgiving, lowering it stricter.

The curve never reaches zero, which keeps scores comparable across files instead of collapsing to a floor. It is also monotonic, so more penalty can never mean a higher score.

## Weighted aggregation

A project's score is **not** the average of its files' scores. It is a line-weighted mean of their densities:

```text
project_density = Σ(densityᵢ × linesᵢ) / Σ linesᵢ
```

This is the same way test coverage is aggregated, and for the same reason. Under a plain average, a ten-line file scoring 0 counts as much as a thousand-line file scoring 0, even though the first is a rounding error and the second is the repository.

Summing raw penalties and dividing once would be simpler and is wrong, because a file's density is computed against a floor (20 lines by default). A one-line file with one finding therefore carries an enormous density that is not proportional to its size, and summing penalties lets that file dominate the project — the opposite of weighting by lines.

Three invariants follow, and all three are enforced by tests in `crates/monostyle/tests/aggregation.rs`:

- Splitting a file into two does not change the project score.
- Doubling both penalty and volume does not change the score.
- Fifty two-line files cannot outweigh one five-thousand-line file.

## Credit for good comments

A comment that explains _why_ earns a **negative** penalty, which offsets other penalties inside the same density number.

Modelling credit as a negative finding rather than as a separate bonus channel means one number — the density — always explains the final score. A reader never has to reconcile a penalty total with a bonus total to understand where a score came from.

Credit is deliberately small and roughly symmetric with the penalty for narration, so commenting is not a way to buy a score. The surrounding code still has to be readable.

## Impact attribution

Every finding records its penalty, so the report can rank rules by how much of the total each accounts for. That is what turns a score into a worklist.

The `share` figure is a rule's penalty divided by the category's total, and the worst offender is tracked separately from the rule total: the message shown is the one from the costliest single finding, because that is the one to act on first.

## Why thresholds are configurable

Thresholds live in `monostyle.toml` rather than in rule bodies, so a project can tighten or relax a rule without forking the tool, and so a report can always state the threshold it measured against.

The trade-off is that scores are only comparable within one configuration. A score recorded before a threshold change is not comparable to one after it.
