local function demo()
  local a = "line one\nline two"
  local b = 'single {quoted} text'
  local c = "brace {body} \"inner\""
  print(a)
  print(b)



  print(c)
  return a .. b .. c
end

demo()
