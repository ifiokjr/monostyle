function assertDefined<T>(value: T | null | undefined): asserts value is T {
  if (value === null || value === undefined) {
    throw new Error("missing value");
  }
}

function use(input: string | null): number {
  assertDefined(input);

  return input.length;
}
