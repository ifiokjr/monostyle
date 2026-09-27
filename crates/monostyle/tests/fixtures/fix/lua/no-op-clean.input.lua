local function render()
  local text = [[
multi {braces} with "quotes"
plain ${x} text]]
  local raw = [==[
body with ]] and {braces}
]==]
  return text .. raw
end

print(render())
