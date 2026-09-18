# Examples

These files are the worked examples behind the scoring tests, and they are documentation as much as
test data: a reader should be able to see what a good score and a bad score look like without
reading the rule set.

They are real inputs to `crates/monostyle/tests/scoring.rs`, so the separation between them is
enforced by the test suite rather than merely asserted in prose.

## `good/rust.rs`

Scores **100** for both readability and complexity.

Every function follows the same shape:

- Guard clauses handle the failure cases and exit early.
- Each control-flow statement gets a blank line above it.
- The return gets a blank line above it.
- Comments explain _why_ — the constraint, tradeoff, or history — rather than restating the code.

Note the comment on the environment-variable precedence in `resolve_config`. It earns credit rather
than merely being tolerated, because explaining a subtle ordering is exactly the work that saves the
next reader an hour.

## `bad/rust.rs`

Scores **0** for both readability and complexity.

The file is deliberately unpleasant, but every problem in it is one monostyle names:

| Problem                                         | Rule that catches it                        |
| ----------------------------------------------- | ------------------------------------------- |
| Six stacked `if` statements with no blank lines | `blank-line-before-control-flow`            |
| Five levels of nesting                          | `deep-nesting`, `excessive-indentation`     |
| A seven-argument call on one line               | `long-parameter-list`                       |
| Twenty statements with no visible groups        | `group-separation`                          |
| No comment on a cognitively complex function    | `comment-required-on-complex-unit`          |
| Eighteen independent paths                      | `cyclomatic-per-unit`, `cognitive-per-unit` |

The point of the file is that it is hard to read _and_ hard to test — the two scores fall together,
which is what you would expect of genuinely bad code and what makes the metric trustworthy.

## Running them yourself

```console
monostyle check examples/good/rust.rs --explain
monostyle check examples/bad/rust.rs --explain
```
