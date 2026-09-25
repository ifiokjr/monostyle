const meter = {
  _value: 0,
  get value() {
    return this._value;
  },
  set value(next) {
    this._value = Number(next) || 0;
  },
};

class Timer {
  #elapsed = 0;

  get elapsed() {
    return this.#elapsed;
  }
}
