---
monostyle_core: feat
monostyle_languages: feat
monostyle_rules: feat
monostyle_markdown: feat
---

# score YAML and properties files and Ruby podspecs

YAML (.yaml/.yml) and Java properties (.properties, including gradle.properties) join the scored languages with indentation-based block detection: no decision points, but comments, strings, line length, and blank-line structure are scored. Ruby now resolves .podspec files, and yaml fences in Markdown are scored instead of skipped as unanalyzed.
