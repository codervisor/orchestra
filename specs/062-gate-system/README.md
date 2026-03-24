---
status: complete
created: 2026-03-23
priority: high
tags:
- harness
- gates
depends_on:
- '061'
parent: 058-code-harness-orchestration
created_at: 2026-03-23T00:53:05.700390656Z
updated_at: 2026-03-23T01:18:06.808732563Z
---

# Gate System: Declarative Preflight Checks with File-Match Filtering

## Overview

Extracted gate system from spec 058. Defines the `gates.yml` schema, file-match filtering, command execution, and failure reporting — shared across all pipeline definitions.

Gates are **preflight checks** (speed optimization), not correctness guarantees. CI runs the complete matrix. This distinction eliminates the duplication concern from spec 058's evaluation.

## Design

### Gate definition schema

```yaml
# .harness/gates.yml
gates:
  preflight:                           # fast, local-only
    - name: rust-check
      match: "*.rs"
      command: cd cli && cargo check
    - name: rust-lint
      match: "*.rs"
      command: cd cli && cargo clippy -- -D warnings
    - name: ts-typecheck
      match: "*.ts,*.tsx"
      command: npx tsc --noEmit
    - name: custom-rules
      match: "*"
      command: .harness/scripts/run-rules.sh
```

### Key design decisions

**No `ci` gate group.** Local gates run `preflight` only. CI monitoring (watching PR checks) is a `run` step with `poll` in the pipeline YAML, not a gate.

**File-match filtering.** Gates only run when changed files match the `match` glob. Changed files determined by `git diff --name-only` against base ref.

**Failure reporting.** Gates produce structured JSON: `{"passed": bool, "failures": [{"name": string, "output": string}]}`. Pipeline engine consumes this for `on_fail` routing.

**Relationship to static_gate.sh.** The existing `.harness/scripts/static_gate.sh` is the predecessor. `gates.yml` replaces its hardcoded language detection with declarative match patterns. Migration path: keep `static_gate.sh` as a single custom-rules gate entry during transition.

## Plan

- [ ] Define `gates.yml` JSON Schema
- [ ] Implement gate YAML parser
- [ ] Implement file-match filtering (`git diff --name-only` + glob matching)
- [ ] Implement gate command executor with structured failure output
- [ ] Integrate with pipeline engine's `run` step (`check: [preflight]`)
- [ ] Migrate `.harness/scripts/static_gate.sh` logic into default `gates.yml`

## Test

- [ ] Gate YAML parsing and schema validation
- [ ] File-match filtering: only matching gates execute
- [ ] Structured failure output format
- [ ] Empty match (no changed files) → gate skipped
- [ ] Multiple gates: all run, failures aggregated
- [ ] Integration with `run` step's `check` field
