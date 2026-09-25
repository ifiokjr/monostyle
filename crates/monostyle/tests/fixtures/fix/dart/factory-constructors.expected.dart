// Factory constructors and required named parameters.

class Point {
  final int x;
  final int y;
  const Point({required this.x, required this.y});
  factory Point.origin() => const Point(x: 0, y: 0);
  factory Point.fromJson(Map<String, dynamic> json) {
    return Point(x: json['x'] as int, y: json['y'] as int);
  }
}

void main() {
  final p = Point.fromJson({'x': 1, 'y': 2});


  print(Point.origin().x);
  print(p.y);
}
