abstract class HasValue {
  int get value;
}

mixin Ordered<T extends HasValue> implements HasValue {
  bool precedes(T other) => value < other.value;

  @override
  int get value;
}

class Reading with Ordered<Reading> {
  const Reading(this.value);

  final int value;
}
