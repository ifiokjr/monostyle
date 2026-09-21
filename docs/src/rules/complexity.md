# Complexity rules

<!-- {=complexityRules} -->

| Rule                  | What it catches                                                      |
| --------------------- | -------------------------------------------------------------------- |
| `cyclomatic-per-unit` | A function with too many independent paths to test                   |
| `cognitive-per-unit`  | A function that is hard to follow, reported with its nesting penalty |
| `npath-per-unit`      | A function with too many execution paths                             |
| `exits-per-unit`      | A function that returns from too many places                         |
| `low-maintainability` | A function with a low maintainability index                          |
| `cyclomatic-per-file` | A file dense with decisions                                          |

<!-- {/complexityRules} -->

## NPath complexity

NPath counts acyclic execution paths rather than independent branches. Sequential branches multiply rather than add, so a function with fourteen sequential `if` statements has a cyclomatic complexity of fifteen and over sixteen thousand execution paths. The two metrics are reported separately because they answer different questions: cyclomatic complexity tells you how many tests you need, while NPath tells you how many paths a reader has to reason about.

## Exit count

A function with many exits is hard to reason about because the reader must hold every escape in mind to know what it guarantees. Early returns are still preferred to nesting, so the rule only fires well past the point where guards are idiomatic.

## Maintainability index

The index combines Halstead volume, cyclomatic complexity, and line count, so it catches a unit that is dense with arithmetic rather than branches — a case the other complexity rules miss.
