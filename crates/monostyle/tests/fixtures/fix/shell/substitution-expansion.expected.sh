#!/usr/bin/env bash

rewrite() {
    local text="old-style {kept}"
    local replaced="${text//old/new}"
    local rest="${text:4:6}"

    printf '%s %s\n' "$replaced" "$rest"
}

rewrite
