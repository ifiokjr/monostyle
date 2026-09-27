defmodule Forms do
  def words do
    list = ~w(one two three)
    literal = ~S(kept #{x} and {braces})
    date = ~D[2026-09-25]


    {list, literal, date}
  end
end
