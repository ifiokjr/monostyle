defmodule Router do
  def path(route) when is_binary(route) and route != "", do: "/" <> route

  def path(_route) do
    "/"


  end
end
