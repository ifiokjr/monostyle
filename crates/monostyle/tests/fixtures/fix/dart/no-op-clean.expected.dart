// A clean, idiomatic file: tricky strings, but nothing for the fixer to do.

String config(String user) {
  final json = '''
{
  "user": "$user",
  "note": "don't edit {this}",
  "active": true
}
''';

  return json.trim();
}

void main() {
  final c = config('ada');

  print(c.length);
}
