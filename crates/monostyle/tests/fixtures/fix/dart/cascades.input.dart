// Cascades chain calls on the same receiver with .. notation.

class Builder {
  final List<String> parts = <String>[];
  void add(String p) => parts.add(p);
  void clear() => parts.clear();
}

void main() {
  final b = Builder()
    ..add('x {y}')
    ..add("z")
    ..clear()
    ..add('final');



  print(b.parts.length);
}
