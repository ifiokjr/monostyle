def compute
  doubled = [1, 2, 3].map { |y| y * 2 }
  total = 0
  [1, 2, 3].each do |x|
    total += x
  end
  puts doubled.inspect

  puts total
  total
end

compute
