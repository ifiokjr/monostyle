class Specimen {
  String build(int? level, WiredFont font) {
    final family = level == null
        ? switch (font) {
            WiredFont.casual => 'RecursiveCasualOriginal',
            WiredFont.mono => 'RecursiveMonoOriginal',
          }
        : font.familyFor(level!);
    return family;
  }
}
