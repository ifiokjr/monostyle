# Auto-fix

<!-- {=autofixExplanation} -->

Five rules are auto-fixable, and every edit is one a formatter leaves alone.

`blank-line-before-control-flow` and `blank-line-after-control-flow` insert a blank line on either side of a decision — before the branch and after its block. `blank-line-before-return` inserts one before a return. `detached-comment` re-attaches a comment that padding has stranded. `excessive-blank-lines` removes the blank lines past the allowance, keeping exactly the number the finding measured against, so it can never disagree with the rule and a second pass changes nothing.

Rustfmt, Prettier, Black, and `dart format` all preserve blank lines between statements and none of them add or remove one, so none of these fixes can fight the project's own tooling. Together they are what make the whitespace rules safe to follow automatically: the rules ask for gaps on every side of a decision, and the fixer applies the gaps while the ceiling keeps them bounded.

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
