// Triple-quoted strings of both flavors, each containing the other delimiter.

String build() {
  final s1 = '''
has "" inside and {braces} and \$sigil
''';
  final s2 = """
has '' inside and {braces} and ${'x'} hole
""";

  return s1 + s2;
}

void main() {
  final out = build();


  print(out.length);
}
