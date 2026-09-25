// Generic function types as fields and parameters.

class Handler {
  void Function(int) onEvent;
  Handler(this.onEvent);
}

void handle(void Function(int) cb, int value) {
  cb(value);
}

void main() {
  final h = Handler((code) => print('code $code {ok}'));
  h.onEvent(1);


  handle((n) => print(n), 2);
}
