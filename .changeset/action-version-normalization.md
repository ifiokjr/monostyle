---
monostyle: patch
---

# The action normalizes the version tag

The action resolved its binary version from the latest-release redirect, whose URL ends in the tagged name — `v0.3.17` — and then built a download URL that prefixed another `v`, 404ing on every run that took the fallback path. The leading `v` is now stripped once, after resolution, on every path.
