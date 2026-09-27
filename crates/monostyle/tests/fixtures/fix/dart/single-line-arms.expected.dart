String label(int value) {
  switch (value) {
    case 0: return 'zero';
    case 1: return 'one';
    default: return 'many';
  }
}

String when(int value) {
  final text = switch (value) {
    0 => 'zero',
    1 => 'one',
    _ => 'many',
  };

  return text;
}
