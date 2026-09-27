#!/usr/bin/env bash

# Prepare the environment for the run below.
export PATH="$PATH:/opt/tools"

run_scan() {
    # Skip when the target is missing.
    if [ ! -f "$1" ]; then
        return 1
    fi


    scan "$1"
}

# Final note at end of file.
