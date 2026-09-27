def config
  <<~CONFIG
    host = example
    port = '{port}'
    path = #{File.join('a', 'b')}
  CONFIG
end

def show
  puts config
end

show
