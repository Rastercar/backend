#!/bin/bash

# checks if any file has changed within a folder

# Get folder to check from the first argument
TARGET_FOLDER="$1"

# Fail if no folder is provided
if [ -z "$1" ]; then
    echo "ERROR: TARGET_FOLDER argument is required."
    exit 1
fi

# Detect changes in the target folder
if git diff --quiet HEAD~1 HEAD -- "$TARGET_FOLDER"; then
    echo "false"
else
    echo "true"
fi

exit 0
