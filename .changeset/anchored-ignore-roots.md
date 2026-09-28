---
monostyle: patch
monostyle_core: patch
---

# Anchored ignore patterns match under absolute roots

Patterns are root-relative by gitignore semantics, but the directory walker handed the whole absolute path to the matcher, whose leading segments are the machine's directory names — so `content/**` silently ignored nothing when monostyle was invoked with an absolute path, while `**/`-prefixed patterns masked the bug by matching anywhere. The walker now strips the root before pattern matching; the generated-header and bundle checks keep the full path because they read the file.
