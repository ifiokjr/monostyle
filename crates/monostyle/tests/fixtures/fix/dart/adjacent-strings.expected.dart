// Adjacent string literals concatenate across lines.

String banner() {
  final text = 'first line '
      'second line '
      "third with {brace} "
      'fourth with \'quote\'';

  return text;
}

void main() {
  final b = banner();


  print(b);
}
