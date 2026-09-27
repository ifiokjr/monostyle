// Type aliases and operator overloading.

typedef Pair = (int, int);

class Money {
  final int cents;
  const Money(this.cents);
  @override
  bool operator ==(Object other) => other is Money && other.cents == cents;
  @override
  int get hashCode => cents;
}

void main() {
  const m = Money(5);
  final p = (1, 2);



  print(m == const Money(5));
  print('${p.$1} {pair}');
}
