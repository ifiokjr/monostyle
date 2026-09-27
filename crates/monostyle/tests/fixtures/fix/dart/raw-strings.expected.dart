// Raw strings keep every backslash and brace literally.

void main() {
  final regex = r'\d+ {x}';
  final pattern = r'''a\b {c\d} "quotes"''';
  final win = r"C:\Users {test}";
  print(regex);
  print(pattern);
  print(win);


  print('done: $regex');
}
