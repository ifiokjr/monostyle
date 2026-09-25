{ name }:
{
  "${name}-suffix" = {
    enable = true;
    text = "prefix {kept} ${name}";
  };
}
