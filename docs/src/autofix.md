# Auto-fix

<!-- {=autofixExplanation} -->

Two rules are auto-fixable, and both edits are ones a formatter leaves alone.

`blank-line-before-control-flow` inserts a blank line before a control-flow statement. Rustfmt, Prettier, Black, and `dart format` all preserve a blank line between statements and none of them remove one.

`excessive-blank-lines` removes the blank lines past the allowance. The number to keep is the same one the finding measured against, so the fix cannot disagree with the rule and running it twice changes nothing the second time.

Those two together are what make the whitespace rules safe to follow automatically: the other rules ask for gaps without bounding them, and this pair supplies both the gap and the ceiling, so a fixer cannot grow a file into mostly whitespace.

Every other rule explains itself and leaves the change to you — breaking a long line, renaming an identifier, extracting a function, and adding an explanatory comment are judgement calls whose automated version would be worse than the problem. The fix output shows both: what was applied, and what still needs a decision, with the suggestion attached.

<!-- {/autofixExplanation} -->

## Running it

```console
monostyle fix .                # apply every fixable finding
monostyle fix . --dry-run      # show what would change
monostyle fix . --rule readability/blank-line-before-control-flow
monostyle fix . --rule readability/excessive-blank-lines
```

## Why only two rules are auto-fixable

Inserting a blank line before a control-flow statement, and deleting the blank lines past the allowance, are the only edits guaranteed to survive a formatter. Rustfmt, Prettier, Black, and `dart format` all preserve a blank line between statements and none of them remove one, so neither fix fights the project's own tooling.

Together they are what makes the rest safe to follow automatically. Every other whitespace rule asks for a gap without bounding it, so a fixer that only inserted blank lines could grow a file into mostly whitespace — which is exactly what happened before `excessive-blank-lines` existed. The pair supplies both the gap and the ceiling, and the collapse fix is idempotent: the number of blank lines it keeps is the same one the finding measured against, so a second run changes nothing.

Every other rule is a judgement call — breaking a long line, renaming an identifier, extracting a function, adding an explanatory comment — and the automated version would be worse than the problem. The fix output shows what was applied and what still needs a decision, with the suggestion attached, so the reader has the same information the fixer had.
