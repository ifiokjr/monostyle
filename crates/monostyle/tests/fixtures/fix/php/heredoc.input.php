<?php

function render(string $table): string {
    $var = "rows";
    $text = <<<EOT
Table {$table} with {braces} and 'quotes'
Second line {$var} done
EOT;
    echo $text;



    return $text;
}

echo render("users");
