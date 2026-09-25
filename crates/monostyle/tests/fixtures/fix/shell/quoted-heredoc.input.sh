#!/usr/bin/env bash

name="value"

cat <<'EOF'
Literal ${name} stays as text
With {braces} and 'quotes'
EOF



echo "real value: ${name}"
