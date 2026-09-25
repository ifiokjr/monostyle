def describe
  hash = { "a" => 1, "b" => 2 }
  text = "#{ {"a" => 1}.keys } nested braces"
  other = "outer { #{hash.size} } inner"
  puts text
  puts other

  [text, other]
end

describe
