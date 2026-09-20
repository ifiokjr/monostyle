# monostyle

<!-- {=projectOverview} -->

`monostyle` scores the complexity and readability of a codebase, a file, or a function, out of 100.
Every point lost is traced to a named rule with an explanation and a suggested fix, so a report is a
worklist rather than a grade.

**Readability** measures how the code looks: whether complex sections have room to breathe, whether
sequential control flow is separated, whether deep nesting has been flattened, and whether comments
explain the hard parts.

**Complexity** measures how hard the code is to follow and to test: cyclomatic complexity (how many
independent paths exist) and cognitive complexity (how much nesting taxes the reader).

<!-- {/projectOverview} -->

<!-- {=projectWhy} -->

Code is read far more often than it is written, and the things that make it pleasant to read are
mostly layout: a blank line before a branch, space around a long argument list, a gap between
logical groups, and a comment on the one function that is genuinely hard to follow.

Those things are invisible to every existing metric. Cyclomatic complexity will happily call a flat,
unreadable function simple; a formatter will happily preserve a 200-line function with no blank
lines anywhere. monostyle exists to make the visual properties of code measurable, so that "this is
hard to read" becomes a specific, fixable list.

<!-- {/projectWhy} -->

## Finding what to fix

<!-- {=impactExample} -->

```console
$ monostyle check . --units
readability: what is costing you points

  ██████░░░░  61.2%  readability/blank-line-before-control-flow  (4,181 findings)
             crates/example/src/main.rs:91 [minor]
             `if` follows the previous statement with no blank line between them
             -> Add a blank line before this statement so the reader can treat it as
             a separate decision rather than part of the previous block.
```

Each entry names its worst offender as `path:line`, and the report ends with the single
highest-value fix:

```console
start here
  Fixing readability/blank-line-before-control-flow at crates/example/src/main.rs:91 would
  recover 30.7% of the available points.
```

<!-- {/impactExample} -->
