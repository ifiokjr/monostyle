defmodule Notes do
  def build(name) do
    sigil = ~s(literal {braces} and "quotes")
    text = """
    value #{name} with {braces}
    done
    """
    sigil <> text
  end
end

IO.puts(Notes.build("world"))
