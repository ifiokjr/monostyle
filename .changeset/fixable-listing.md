---
monostyle: patch
---

# List every fixable rule in `monostyle rules --fixable`

The listing worked by running each rule over a hard-coded sample and keeping the ones that produced an edit. The sample contained a crowded control-flow statement but no stacked blank lines, so the blank-line collapse added in this release never appeared in the list: `monostyle rules --fixable` reported one fixable rule where there were two.

The sample now holds one problem of every fixable shape. The approach itself — discovering fixability by running the rule rather than declaring it — is unchanged, and is the right one: a declared flag is a second place to forget, as this bug demonstrates.
