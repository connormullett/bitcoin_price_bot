#!/usr/bin/env bash
PATH_TO_ENV=$1
# Show env vars
grep -v '^#' $PATH_TO_ENV
# Export env vars
export $(grep -v '^#' $PATH_TO_ENV | xargs)