defmodule Demo do
  def render(name) do
    sigil = ~s(raw {braces} text with "quotes")
    text = """
    Hello #{name} with {braces} inside
    Second #{name} line done
    """
    IO.puts(sigil)

    IO.puts(text)
    text
  end
end

Demo.render("world")
