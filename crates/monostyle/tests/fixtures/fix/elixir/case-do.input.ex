defmodule Picker do
  def pick(key) do
    label =
      case key do
        "a" -> "alpha {braced}"
        "b" -> "beta {body}"
        other -> "other #{other}"
      end
    IO.puts(label)



    label
  end
end

Picker.pick("a")
