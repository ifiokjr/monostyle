# Releases

<!-- {=changesetWorkflow} -->

A changeset describes one change and the version bump it needs. Write one with:

```console
monochange run change --package monostyle_rules --bump patch --reason "Fix the long-parameter-list rule firing on every call"
```

Or write the file by hand in `.changeset/`:

```markdown
---
"monostyle_rules": patch
---

# Fix the long-parameter-list rule firing on every call

The rule tested argument count and rendered width separately, so any call over forty columns was reported regardless of how many arguments it had. It now requires both, which is what makes the finding mean something.
```

The release workflow reads every changeset, computes the next version for each package, and opens a release pull request. Merging that pull request is what cuts the release, so the schedule is "whenever a release is worth shipping" rather than a fixed cadence.

## What needs a changeset

A pull request that changes a published package needs one, or the release notes for that version would not mention the change. Documentation, tests, snapshots, and examples do not: none of them change what a published package does.

## Trusted publishing

Releases publish with trusted publishing when a verifiable CI identity is available, and fall back to the `NPM_TOKEN` and `CARGO_REGISTRY_TOKEN` secrets otherwise. The fallback exists because a package has to exist in a registry before it can be enrolled with a trusted publisher, so the first publish of anything always uses a token. Set `force_token_auth` when dispatching the publish workflow to skip the OIDC exchange deliberately.

<!-- {/changesetWorkflow} -->
