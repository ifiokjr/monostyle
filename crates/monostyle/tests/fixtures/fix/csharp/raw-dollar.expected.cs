using System;

class Template
{
    static string Render(int count) => $$"""
{
  "count": {{count}},
  "literal": "{kept}"
}
""";
}
