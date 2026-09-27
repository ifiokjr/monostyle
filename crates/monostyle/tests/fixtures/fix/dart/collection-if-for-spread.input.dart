// Collection literals with spread, if, and for elements.

List<String> build(bool flag) {
  final base = <String>['a', 'b'];
  final out = <String>[
    'head {x}',
    if (flag) 'flagged',
    ...base,
    for (final item in base) 'item-$item',
  ];
  return out;
}

void main() {
  final xs = build(true);



  print(xs.join(', '));
}
