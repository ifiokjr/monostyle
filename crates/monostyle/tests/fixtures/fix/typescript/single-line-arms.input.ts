function label(code: number): string {
  switch (code) {
    case 0: return "zero";
    case 1: return "one";
    default: return "many";
  }
}

function guarded(code: number): string {
  switch (true) {
    case code < 0: return "negative";
    case code < 10: return "small";
    default: return "large";
  }
}
