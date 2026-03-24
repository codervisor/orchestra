---
status: planned
created: 2026-03-16
priority: low
tags:
- fixture
- test
parent: 044-factory-skill-mvp
depends_on: []
---

# Sample Spec — Add Greeting Utility

> **Status**: planned · **Priority**: low · **Created**: 2026-03-16

## Overview

A trivial spec used to validate the factory skill end-to-end. Implements a
greeting utility function with tests.

## Design

Create a file `skills/factory/fixtures/sample-spec/greet.sh` that:

1. Accepts a name as the first argument.
2. Prints `Hello, {name}!` to stdout.
3. If no argument is provided, prints `Hello, World!`.
4. Exits with code 0 on success.

## Plan

- [ ] Create `skills/factory/fixtures/sample-spec/greet.sh` with the greeting logic
- [ ] Create `skills/factory/fixtures/sample-spec/greet_test.sh` that validates all cases
- [ ] Ensure the test script exits 0 on success, non-zero on failure

## Test

- [ ] `bash greet.sh Alice` outputs exactly `Hello, Alice!`
- [ ] `bash greet.sh` (no args) outputs exactly `Hello, World!`
- [ ] `bash greet_test.sh` exits with code 0
