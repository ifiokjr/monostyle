def forms
  raw = %q{literal #{kept} and {braces}}
  interpolated = %Q{value #{1 + 1} and {kept}}
  words = %w[one two {three}]
  symbols = %i[alpha beta]

  [raw, interpolated, words, symbols]
end
