#!/usr/bin/env bash

show() {
    local var="value"
    local arr=(one two three)
    echo "${var}suffix"
    echo "${arr[@]}"



    echo "count: ${#arr[@]}"
}

show
