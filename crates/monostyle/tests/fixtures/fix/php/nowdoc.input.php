<?php

function literal(): string {
    $var = "unseen";
    $text = <<<'EOT'
Literal {$var} stays as text
With {braces} and 'quotes'
EOT;
    echo $text;



    return $text;
}

echo literal();
