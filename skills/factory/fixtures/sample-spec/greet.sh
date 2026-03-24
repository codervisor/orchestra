#!/usr/bin/env bash
# greet.sh — Print a greeting for the given name (or "World" if none provided).

name="${1:-World}"
echo "Hello, ${name}!"
exit 0
