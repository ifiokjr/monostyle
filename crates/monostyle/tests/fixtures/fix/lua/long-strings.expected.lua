local function first()
  local text = [[
multi {braces} line with "quotes"
second line done]]
  print(text)
end

local function second()
  local raw = [==[
has ]] and "quotes" inside
]==]
  print(raw)
end

first()
second()
