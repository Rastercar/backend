#!/bin/bash

# checks if any file has changed within a folder

# Fail if no folder is provided
if [ -z "$1" ]; then
    echo "ERROR: TARGET_FOLDER argument is required."
    exit 1
fi

# Get folder to check from the first argument
TARGET_FOLDER="$1"

# Detect changes in the target folder
if git diff --quiet HEAD~1 HEAD -- "$TARGET_FOLDER"; then
    echo "NO changes detected in $TARGET_FOLDER (exit 0)"
    exit 0
else
    echo "Changes detected in $TARGET_FOLDER (exit 1)"
    exit 1
fi
