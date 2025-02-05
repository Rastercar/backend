#!/bin/bash

# checks if any file has changed within a folder

# Folder to check for changes
TARGET_FOLDER="./services/mailer"

# Detect changes in the target folder
if git diff --quiet HEAD~1 HEAD -- "$TARGET_FOLDER"; then
    echo "No changes detected in $TARGET_FOLDER."
    exit 1 # Exit with 1 to indicate no changes
else
    echo "Changes detected in $TARGET_FOLDER."
    exit 0 # Exit with 0 to indicate changes exist
fi
