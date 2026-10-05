# Skill evals

These evals pin the skill's **paragraph-grouping** guidance — blank lines separate concepts, never split one — to behaviour that can be checked:

- `mid-concept-break` — a `group-separation` fix whose blank landed between a buffer and the lines that fill it must be regrouped so each buffer is one paragraph.
- `sub-concepts-in-a-long-run` — a long run with three concepts inside it (timeouts, retries, telemetry) must break at those boundaries, not at the statement limit.
- `build-then-populate` — constructing an object and its immediate setup calls are one paragraph.
- `arrange-act-assert` — a test's phases are the paragraphs; a fixture's preparation lines stay together.
- `skill-examples` — `SKILL.md`'s own Before/After pair keeps showing the mid-concept break and its fix, and the After example is clean under the real binary when it stands alone.

## Running

```console
python3 packages/monostyle__skill/evals/run.py                # self mode
python3 packages/monostyle__skill/evals/run.py --answers DIR  # grade an agent
```

Self mode grades the shipped `expected.*` files and runs in CI. Answer mode grades `DIR/<case>.<ext>` produced by any agent that was given the task files and `SKILL.md` — that is the loop for testing whether the skill's guidance is actually followed, not just whether the reference answers are right.

Grading is content-anchored (regexes over the answer) plus the real monostyle binary (the answer must not trip `group-separation`), and every case compares the answer against the input with blank lines removed, so a candidate cannot pass by deleting the code it was asked to regroup.
