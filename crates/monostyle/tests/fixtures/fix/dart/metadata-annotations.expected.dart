// Metadata annotations on classes, methods, and their arguments.

class Foo {
  final String bar;
  const Foo({required this.bar});
}

@Deprecated('use NewThing {instead}')
class OldThing {
  @Foo(bar: 'legacy')
  void run() => print('running');
}

void main() {
  final o = OldThing();


  o.run();
}
