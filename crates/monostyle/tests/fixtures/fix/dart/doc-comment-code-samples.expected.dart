/// A formatter with a fenced sample in its doc text:
///
/// ```dart
/// final x = f("{a: 1}");
/// if (x != null) { print(x); }
/// ```
///
/// The braces and quotes above live in doc text only.
class Formatter {
  String wrap(String s) => '[$s]';
}

void main() {
  final f = Formatter();
  final out = f.wrap('body {1}');


  print(out);
}
