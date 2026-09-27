async function* pages<T>(source: AsyncIterable<T[]>): AsyncGenerator<T> {
  for await (const batch of source) {
    for (const item of batch) {
      yield item;
    }
  }
}
