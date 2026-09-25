// Sealed hierarchy with an exhaustive switch statement.

sealed class Shape {}

class Circle implements Shape {
  final double r;
  Circle(this.r);
}

class Square implements Shape {
  final double s;
  Square(this.s);
}

String area(Shape sh) {
  switch (sh) {
    case Circle c:
      return 'circle ${c.r}';
    case Square s:
      return 'square {s} ${s.s}';
  }
}

void main() {
  print(area(Circle(1)));



  print(area(Square(2)));
}
