<?php

function message(string $name): string {
    $text = "line one\n and \"quoted\" and \$notvar and {kept}";


    return $text . $name;
}
