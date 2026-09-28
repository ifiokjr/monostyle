---
monostyle_rules: patch
---

# A destructuring closer is not a finished statement

`if let Some(Segment { .. }) = classify_vec(inner)` closes its pattern on one line and opens its body on the next; the after-rule read the `})` as a finished chain and padded a blank before the `{`, which rustfmt removed. A closer whose remainder continues into a binding now stands down.
