class Builder {
  final parts = <String>[];

  Builder add(String part) => this..parts.add(part);
}

void main() {
  final built = Builder()..add('a').add('b');


  print(built.parts.length);
}
