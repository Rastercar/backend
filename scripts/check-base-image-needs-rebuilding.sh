#!/bin/bash

# checks if the base rust image used by the services needs to be rebuild

for file in Dockerfile Cargo.lock Cargo.toml .dockerignore; do
    if ! git diff --exit-code -s HEAD~1 -- "$file"; then
        echo "true"
        exit 0
    fi
done

echo "false"
exit 0
