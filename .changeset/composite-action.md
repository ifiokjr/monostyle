---
monostyle: minor
---

# A composite action for CI annotations

The repository now ships a GitHub Actions composite action at its root, so a workflow needs two lines to render findings as inline pull-request annotations. The action downloads the release binary matching the tag the action was referenced at, which pins the action and the binary together; branch and SHA references fall back to the latest release. The platform detection and download that each consumer previously carried in its workflow live here now.
