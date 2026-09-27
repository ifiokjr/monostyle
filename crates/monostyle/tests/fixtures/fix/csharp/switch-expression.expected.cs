using System;

class Labels
{
    static string Name(int code) => code switch
    {
        1 => "one",
        2 or 3 => "few",
        _ => "many",
    };
}
