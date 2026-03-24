---
status: complete
created: 2026-03-23
priority: critical
tags:
- harness
- runtime
- pipeline
parent: 058-code-harness-orchestration
created_at: 2026-03-23T00:52:43.585037249Z
updated_at: 2026-03-23T01:18:06.801186191Z
---

# Pipeline Engine Core: YAML Schema, Parser, and 4-Type Step Executor

## Overview

Core pipeline engine extracted from spec 058. Defines the YAML pipeline schema, parser, and step executor with 4 step types (reduced from the original 7).

The original 7 types (`agent`, `gate`, `shell`, `watch`, `route`, `loop`, `parallel`) collapse to 4:

| New type | Replaces | Key distinction |
|----------|----------|----------------|
| `agent` | agent | LLM invocation via `claude -p` with tool constraints and structured output |
| `run` | shell, gate, watch | Command execution. Flags: `match` (file filter → gate), `poll: {interval, timeout}` (→ watch), `check: [static, ci]` (→ gate group) |
| `branch` | route | Verdict-based branching with `approve`/`rework`/`exhaust` handlers |
| `fan` | loop, parallel | Collection processing. `mode: parallel\|sequential\|loop`, `until` for termination, `over` for dynamic sets |

## Design

### Step schema (simplified)

```yaml
steps:
  - name: build
    type: agent
    prompt: prompts/build.md
    tools: [Read, Edit, Write, Bash, Glob, Grep]
    max_turns: 50
    isolation: worktree
    output_schema: schemas/build-report.json

  - name: gates
    type: run
    check: [static]              # gate behavior: run gate groups from gates.yml
    match: ["*.rs", "*.ts"]      # file-match filtering
    retry: 2
    on_fail: rework(build)

  - name: route
    type: branch
    input: inspect.verdict
    approve: create-pr
    rework: build
    max_iterations: 3
    exhaust: escalate

  - name: solve-leaves
    type: fan
    mode: parallel
    over: pending_nodes
    step: { type: agent, prompt: ... }

  - name: adversarial-loop
    type: fan
    mode: loop
    until: terminated
    max_iterations: 5
    termination:
      consecutive_clean: 2
      plateau_rounds: 3
    steps: [...]
```

### Middleware resolution order

Middleware applies outside-in: `log(retry(timeout(step)))`.

| Composition | Behavior |
|-------------|----------|
| `retry` + `timeout` | Timeout resets per retry attempt |
| `retry` + `on_fail` | `on_fail` fires only after all retries exhaust |
| `log` + `retry` | Each attempt logged with `attempt: N` field |
| `retry` + `condition` | Condition checked before first attempt only |

Available middleware: `retry(n)`, `timeout(ms)`, `log(path)`, `manifest()`, `on_fail(action)`.

### Variable interpolation

Constrained to simple substitution only — no filters, pipes, or expressions.

- Syntax: `${scope.field}` only
- Scopes: `config.*`, `spec.*`, `manifest.*`, `steps.{name}.*`, `loop.*`
- Unset variable → runtime error (fail-fast)
- Complex transformations → computed `context` maps in runtime code

### Agent invocation

Hardcode `claude -p` integration directly. No provider abstraction until a second provider materializes.

No session continuity in v1. Context passing via prompt injection (build diff + errors + manifest piped into subsequent agent steps).

## Plan

- [ ] Define YAML schema for 4 step types + middleware (JSON Schema)
- [ ] Implement YAML parser with schema validation
- [ ] Implement `agent` step executor (`claude -p` subprocess)
- [ ] Implement `run` step executor (command + match + poll + check modes)
- [ ] Implement `branch` step executor (verdict routing)
- [ ] Implement `fan` step executor (parallel/sequential/loop modes)
- [ ] Implement middleware chain (retry, timeout, log, manifest, on_fail)
- [ ] Implement variable interpolation (simple `${scope.field}` only)
- [ ] Add `synodic harness validate` for pre-execution schema checking
- [ ] Add `synodic harness run --pipeline <name>` entry point

## Test

- [ ] YAML parsing for all 4 step types + middleware
- [ ] Middleware interaction: retry + timeout resets per attempt
- [ ] Middleware interaction: on_fail fires after retries exhaust
- [ ] Variable interpolation: valid scopes resolve, unset vars error
- [ ] `fan` mode=loop: termination conditions (consecutive clean, plateau, max)
- [ ] `fan` mode=parallel: fan-out over dynamic set, results collected
- [ ] `branch`: approve/rework/exhaust branches
- [ ] `run` with poll: timeout triggers on_fail
- [ ] Agent tool constraints enforced (read-only tools list)
- [ ] `synodic harness validate` catches schema errors before execution
