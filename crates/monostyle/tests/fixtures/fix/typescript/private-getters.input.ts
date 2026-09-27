class Meter {
  #count = 0;

  get #doubled(): number {
    return this.#count * 2;
  }

  tick(): number {
    this.#count += 1;

    return this.#doubled;
  }
}
