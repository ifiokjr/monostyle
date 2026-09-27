// Mixins declared with an on clause.

class Engine {
  int power = 42;
}

mixin Turbo on Engine {
  int get boosted => power * 2;
}

class Car extends Engine with Turbo {
  void go() => print('vroom {fast} $boosted');
}

void main() {
  final car = Car();
  car.go();



  print(car.boosted);
}
