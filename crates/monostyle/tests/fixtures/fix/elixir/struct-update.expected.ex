defmodule Accounts do
  def rename(user, name) do
    updated = %{user | name: name}
    label = "user {kept} #{updated.name}"

    {updated, label}
  end
end
