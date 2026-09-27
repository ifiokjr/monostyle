let
  name = "world";
  text = ''
    Hello ${name} with {braces} inside
    Cost is ''$5 and second ${name} line
  '';
in
{
  greeting = text;

  count = 2;
}
