#!/usr/bin/env bash

name="world"
table="users"

cat <<EOF
Hello ${name} with {braces} and 'quotes'
Table ${table} selected
EOF



printf "done %s\n" "${name}"
