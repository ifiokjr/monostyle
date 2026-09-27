// Records and destructuring patterns.

String locate((int, int) point) {
  switch (point) {
    case (0, 0):
      return 'origin';

    case (0, var y):
      return 'on y axis at $y';

    case (var x, var y):
      return '($x, $y)';
  }
}

void main() {
  final pair = (3, 4);


  print(locate(pair));
}
