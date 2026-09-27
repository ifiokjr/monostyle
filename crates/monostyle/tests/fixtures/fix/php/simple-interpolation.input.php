<?php

function profile(string $name, int $age): string {
    $line = "user is $name and {braces} after";
    $extra = "age {$age} and $age plain";
    echo $line;
    echo $extra;



    return $line . $extra;
}

echo profile("Ada", 36);
