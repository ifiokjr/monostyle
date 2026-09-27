({int x, int y}) origin() => (x: 0, y: 0);

void main() {
  final point = origin();
  final (x: px, y: py) = origin();


  print('${point.x} $px $py');
}
