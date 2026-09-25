typedef Callback = void Function(int tick, {String? label});
typedef IntList = List<int>;

void schedule(Callback onTick) {
  onTick(0, label: 'start');
}
