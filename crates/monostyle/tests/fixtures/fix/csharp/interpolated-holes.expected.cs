using System;

class Report
{
    static string Find(string key) => key + "!";

    static string Label(int count) => $"items: {Find("key")} of {count}";
}
