#!/usr/bin/env bash

name="world"

cat <<EOF
Hello ${name} with {braces} and 'quotes'
Done
EOF

echo "finished: ${name}"
