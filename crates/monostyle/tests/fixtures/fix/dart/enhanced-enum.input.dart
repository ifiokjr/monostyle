enum Color {
  red(0xff0000),
  green(0x00ff00);

  const Color(this.value);

  final int value;

  String render() => '#${value.toRadixString(16)}';
}
