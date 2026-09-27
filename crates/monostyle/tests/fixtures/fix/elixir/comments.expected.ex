defmodule Scanner do
  @moduledoc """
  Scans targets {kept}.
  """

  @doc """
  Skips when the target is missing.
  """
  def scan(nil), do: :missing

  def scan(target) do
    # The heavy work happens below.
    read(target)

  end
end
