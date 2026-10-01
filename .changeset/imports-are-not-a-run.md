---
monostyle: patch
monostyle_rules: patch
---

# Import headers are not a statement run

A header of `use`, `import`, or `using` lines is one block the formatter owns — rustfmt orders it, ktlint forbids blanks inside it — so `group-separation` no longer counts directives as statements and no longer splits the header. The split had been inserting blanks the project's own formatter or linter rejects.
