using System;

class Quote
{
    static string Tell()
    {
        var quote = @"He said ""hi"" then {left}";
        var path = @"C:\Users\{name}\docs";


        return quote + path;
    }

    static void Main() => Console.WriteLine(Tell());
}
