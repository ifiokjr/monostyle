String describe(int value, bool loud) {
  switch (value) {
    case 0 when loud:
      return 'zero (loud)';
    case 0:
      return 'zero';
    case final n when n > 10:
      return 'many';
    default:
      return 'few';
  }
}
