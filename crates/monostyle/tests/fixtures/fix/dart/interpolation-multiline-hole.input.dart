// A multiline template whose interpolation holes sit alone on their lines.

String render(String header, String footer, String body) {
  final page = '''
${header}
$body
${footer}
''';
  return page;
}

void main() {
  final out = render('== TOP ==', '== END ==', 'middle {text}');



  print(out);
}
