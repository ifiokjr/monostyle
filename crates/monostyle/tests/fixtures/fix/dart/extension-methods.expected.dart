// Extension methods add behavior to existing types.

extension Shout on String {
  String shout() => '${toUpperCase()}! {louder}';
  String twice() => '$this$this';
}

void main() {
  final s = 'hey';
  final t = s.shout();


  print(t);
  print(s.twice());
}
