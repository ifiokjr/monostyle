using System;

class Paths
{
    static string Describe(string folder) => $@"root: C:\{folder}\logs {DateTime.Now:yyyy}";
}
