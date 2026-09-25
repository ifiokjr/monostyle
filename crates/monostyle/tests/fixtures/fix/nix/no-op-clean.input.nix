let
  name = "world";
  text = ''
    Hello ${name} with {braces}
    Literal ''$ and "quotes" too
  '';
in
{
  greeting = text;
  plain = "a {b} c ${name}";
}
