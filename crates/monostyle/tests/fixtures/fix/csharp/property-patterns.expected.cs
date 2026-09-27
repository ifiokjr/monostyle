using System;

class Gate
{
    static string Describe(Person person) => person switch
    {
        Person { Name: "ada", Age: > 30 } => "founder",
        Person { Age: > 18 } adult => "adult",
        _ => "minor",
    };
}

record Person(string Name, int Age);
