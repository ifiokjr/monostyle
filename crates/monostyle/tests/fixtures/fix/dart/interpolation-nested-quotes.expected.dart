// Nested quotes inside interpolation holes must not confuse the tokenizer.

String describe(String inner) => 'inner: $inner';

void main() {
  final a = 'outer ${describe("double")} tail';
  final b = "outer ${describe('single')} tail";
  final c = 'mix ${describe("a ${'b'} c")} end';
  print(a);
  print(b);


  print(c);
}
