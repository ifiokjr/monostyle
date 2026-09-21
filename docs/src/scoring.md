# How scoring works

<!-- {=scoringModel} -->

Findings carry a `weight`; severity scales it into a penalty. Penalties sum per category and are normalized by code volume into a penalty density — findings per 100 lines — so a large well-written file is not punished for its size. Density maps to 0–100 through exponential decay:

```
score = 100 * 2 ^ (-density / half_life)
```

The half-life is the density at which a category scores exactly 50, which makes the whole curve tunable with one readable number.

Good comments earn **negative** penalties. That is how a well-placed explanation raises a score: the credit offsets other penalties inside the same density number, so one number always explains the result.

Scores are aggregated by **line-weighted mean**, the same way test coverage is aggregated. A ten-line file cannot count as much as a thousand-line file. Three invariants hold, and all three are enforced by tests: splitting a file does not change the project score, doubling penalty and volume does not change it, and fifty two-line files cannot outweigh one five-thousand-line file.

<!-- {/scoringModel} -->

## Why the half-life is tunable

The half-life is the density at which a category scores exactly 50. That makes the whole curve tunable with one readable number: raising it makes the tool more forgiving, lowering it stricter. The curve never reaches zero, which keeps scores comparable across files instead of collapsing to a floor.

## Why scores are weighted by lines

A project's score is not the average of its files' scores. It is a line-weighted mean of their penalties, the same way coverage is aggregated, so a ten-line file cannot count as much as a thousand-line file. Three invariants hold, and all three are enforced by tests: splitting a file does not change the project score, doubling penalty and volume does not change it, and fifty two-line files cannot outweigh one five-thousand-line file.
