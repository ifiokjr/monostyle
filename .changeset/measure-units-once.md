---
"monostyle_core": minor
"monostyle_rules": minor
"monostyle": minor
---

# Measure each file's units once and stop copying the findings

Seven rules each built their own set of measured units per file, so unit detection and the per-unit cyclomatic, cognitive, Halstead, and maintainability figures ran seven times over every file. The run context now holds one `UnitSet` shared by the rules and the report, and rules that never look at units never pay for them.

Findings borrow their text: the rule name is a registry constant and most suggestions are fixed sentences, so a rule name is no longer copied into every finding and a fixed suggestion owns nothing. Project-wide ranking and scoring work from references instead of cloning every finding and its path, and the worst-offender location for a rule is built only when a finding actually wins that title.

Attributing findings to units is now a binary search per unit rather than a scan of every finding for every unit, and rules are matched by name through a hash map rather than a linear scan of the registry. `f64` ordering uses `total_cmp` throughout, and the rule table is a static rather than a per-file allocation.

Deleting the orphaned `metrics_bridge.rs` removes the last copy of the address-keyed unit cache whose wrong-by-construction keying was documented as unsound when `UnitSet` replaced it.
