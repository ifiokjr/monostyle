// JSON built from a multiline string; brace and dollar traps stay literal.

String buildQuery(String name, List<String> roles) {
  final query = '''
{
  "user": "$name",
  "roles": $roles,
  "note": "use \$name carefully {braces}"
}
''';
  return query.trim();
}

void main() {
  final out = buildQuery('ada', ['admin']);



  print(out);
}
