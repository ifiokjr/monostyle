#!/usr/bin/env bash

stamp() {
    local when="$(date -u +%Y)"
    local both="$(echo "${when} and {kept}")"


    printf '%s\n' "$both"
}

stamp
