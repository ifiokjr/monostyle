---
monostyle_rules: patch
---

# Attribute and condition closers keep their statements whole

Two more placements the local formatter gates caught before anything was pushed: a blank line landed between `#[cfg(test)]` and the `return` it configures (a clippy error under `-D warnings`), so the before-return rule now anchors above the attribute block the way the before-control-flow rule does. And a condition ending in a braced expression — `} == compare(target)` — opens its body on the next line, so the after-rule treats a closer sitting above a body brace as unfinished rather than as a finished chain.
