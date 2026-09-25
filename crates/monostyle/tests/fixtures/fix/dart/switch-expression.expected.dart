// Dart 3 switch expressions with pattern arrows.

String describe(Object value) {
  return switch (value) {
    int n when n < 0 => 'negative {n}',
    int n => 'positive $n',
    String s => 'text: $s',
    [_, _] => 'pair',
    _ => 'unknown',
  };
}

void main() {
  print(describe(-3));


  print(describe('hi'));
}
