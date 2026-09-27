{ lib }:
let
  body = ''
    escaped ''${kept} literal
    dollar ''$ sign and {braces}
  '';


in
{
  text = body;
  enabled = lib.mkOption { type = lib.types.bool; };
}
