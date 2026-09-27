<?php

function page(string $title): string {
    $body = <<<EOT
<h1>{$title}</h1>
<p>literal {kept} here</p>
EOT;


    return trim($body);
}
