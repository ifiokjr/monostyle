// Escapes: newline, quote, tab, and dollar inside normal strings.

String receipt() {
  final line1 = 'costs \$5\n';
  final line2 = 'it\'s fine';
  final line3 = "tab\tstop {x}";

  return line1 + line2 + line3;
}

void main() {
  final r = receipt();
  print(r);


  print(r.length);
}
