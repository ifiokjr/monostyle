<?php

function banner(string $host): string {
    $text = <<<EOT
Host is {$host} with {braces} and '$host'
Done
EOT;
    return $text;
}

echo banner("example.org");
