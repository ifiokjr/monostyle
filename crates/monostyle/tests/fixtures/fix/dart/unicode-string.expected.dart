// Unicode text and braces sharing a single string literal.

String greet() {
  final s = 'héllo {wörld} 你好 🎉';

  return s;
}

void main() {
  final g = greet();


  print(g);
  print(g.length);
}
