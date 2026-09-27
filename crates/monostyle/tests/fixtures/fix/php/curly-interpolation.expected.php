<?php

function greet(array $user): string {
    $text = "Hello {$user["name"]} and {braces} text";
    $also = "Path {$user["home"]} end";
    echo $text;
    echo $also;

    return $text . $also;
}

echo greet(["name" => "Ada", "home" => "/tmp"]);
