#!/bin/bash

# checks if the base rust image used by the services needs to be rebuild

for file in Dockerfile Cargo.lock Cargo.toml .dockerignore; do
    if ! git diff --exit-code -s HEAD~1 -- "$file"; then
        echo "$file changed, base image needs rebuilding"

        # returns 1 if rebuild is needed
        exit 1
    fi
done

echo "base image does NOT need rebuilding"
exit 0
