def tokens
  words = %w[one two three]
  syms = %i[symbol words here]
  puts words.join(',')
  puts syms.inspect

  [words, syms]
end

tokens
