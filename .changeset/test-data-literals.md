---
monostyle: patch
monostyle_rules: patch
---

# Test data is not magic

An assertion's literal is the value it claims — `expect(disc(ix), 12)` and `assert_eq!(len, 3)` state what the code must produce, and naming the number would hide the claim the test makes. A collection whose members are all literals is a data table wherever it sits, not only alone on a line: `Uint8List.fromList([1, 2, 3, 4, 5])` is a byte fixture. A named field holds a constructor call of numbers — `unixTimestamp: BigInt.from(123)` — the field names the value and the constructor names the type, and `Ident::new(120, 80)` stays two unnamed arguments because a path operator is not a field separator. `on`, `eq`, and `ne` join the language vocabulary: the event-registration name the web exports and the two methods an equality impl must carry are not choices an author can unmake.
