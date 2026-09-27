using System;

class Greeter
{
    public string Name { get; init; } = "ada";

    public string Greet() => $"Hello, {Name}!";
}
