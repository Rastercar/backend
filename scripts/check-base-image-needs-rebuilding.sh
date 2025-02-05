#!/bin/bash

# checks if the base rust image used by the services needs to be rebuild

## Did we found IP address? Use exit status of the grep command ##
if git diff --exit-code -s HEAD~1 -- Dockerfile; then
    echo "root Dockerfile not changed"
    exit 0
else
    echo "root Dockerfile changed, base image needs rebuilding"
    exit 1
fi

#
