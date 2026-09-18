# Auto-fix

<!-- {=autofixExplanation} -->

One rule is auto-fixable: inserting a blank line before a control-flow statement. That is the only
edit guaranteed to survive a formatter — rustfmt, Prettier, Black, and `dart format` all preserve a
blank line between statements and none of them remove one. A fixer that fights the project's
formatter produces a diff the next format run reverts, which is worse than the finding itself.

Every other rule explains itself and leaves the change to you. The fix output shows both: what was
applied, and what still needs a decision, with the suggestion attached.

<!-- {/autofixExplanation} -->

## Running it

```console
monostyle fix .                # apply every fixable finding
monostyle fix . --dry-run      # show what would change
monostyle fix . --rule readability/blank-line-before-control-flow
```

## Why only one rule is auto-fixable

Inserting a blank line before a control-flow statement is the only edit guaranteed to survive a
formatter. Every other rule is a judgement call — breaking a long line, renaming an identifier,
extracting a function, adding an explanatory comment — and the automated version would be worse than
the problem. The fix output shows what was applied and what still needs a decision, with the
suggestion attached, so the reader has the same information the fixer had.
