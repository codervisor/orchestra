#!/usr/bin/env bash
# greet_test.sh — Tests for greet.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
failures=0

# Test 1: Greeting with a name argument
expected="Hello, Alice!"
actual="$(bash "$SCRIPT_DIR/greet.sh" Alice)"
if [[ "$actual" != "$expected" ]]; then
  echo "FAIL: expected '$expected', got '$actual'"
  failures=$((failures + 1))
else
  echo "PASS: greet.sh Alice -> $actual"
fi

# Test 2: Greeting with no arguments (default)
expected="Hello, World!"
actual="$(bash "$SCRIPT_DIR/greet.sh")"
if [[ "$actual" != "$expected" ]]; then
  echo "FAIL: expected '$expected', got '$actual'"
  failures=$((failures + 1))
else
  echo "PASS: greet.sh (no args) -> $actual"
fi

# Test 3: Exit code is 0
bash "$SCRIPT_DIR/greet.sh" >/dev/null 2>&1
rc=$?
if [[ $rc -ne 0 ]]; then
  echo "FAIL: expected exit code 0, got $rc"
  failures=$((failures + 1))
else
  echo "PASS: exit code is 0"
fi

if [[ $failures -gt 0 ]]; then
  echo "$failures test(s) failed."
  exit 1
fi

echo "All tests passed."
exit 0
