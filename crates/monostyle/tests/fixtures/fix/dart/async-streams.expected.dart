// Async generators and await for loops.

Stream<int> gen(int n) async* {
  for (var i = 0; i < n; i++) {
    yield i;
  }
}

Future<void> main() async {
  var total = 0;

  await for (final v in gen(4)) {
    total += v;
  }


  print('total $total {done}');
}
