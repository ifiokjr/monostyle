interface Node<T> {
  value: T;
  next: Node<T> | null;
}

function deep(map: Map<string, Array<number>>): Record<string, Partial<Node<number>>> {
  const out: Record<string, Partial<Node<number>>> = {};

  for (const [key, values] of map) {
    out[key] = { value: values.length };
  }

  return out;
}
