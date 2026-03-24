---
status: complete
created: 2026-03-22
priority: critical
tags:
- harness
- orchestration
- architecture
- runtime
- pipeline
- middleware
created_at: 2026-03-22T16:42:58.873730006Z
updated_at: 2026-03-22T20:41:42.952829462Z
---
# AI Harness: Intelligent Manufacturing Network for Autonomous Agent Coordination

## Overview

Building the factory, fractal, swarm, and adversarial skills as SKILL.md files revealed that prompt-based orchestration is unreliable — agents skip steps, miss gates, and fail silently. This matches patterns observed in OpenAI Swarm, Dagger's pipeline engine, and Elastic's agent coordination work: **the agent isn't the hard part — the harness is**.

This umbrella spec defines the **AI Harness** — a deterministic pipeline engine that replaces SKILL.md prompt-based orchestration with code-based orchestration. Implementation is decomposed into 5 child specs.

**Pipeline Engine** (deterministic orchestration):
- 4 step types: `agent`, `run`, `branch`, `fan`
- Declarative YAML pipelines with composable middleware
- Preflight gate system with file-match filtering
- Direct `claude -p` integration (no premature abstraction)

**Skill Topologies** (production patterns built on the engine):

| Phase | Pattern | Pipeline | Role |
|-------|---------|----------|------|
| Design | **Speculative Swarm** | swarm.yml | Divergent exploration |
| Design | **Fractal Decomposition** | fractal.yml | Orthogonal decomposition with scope isolation |
| Production | **Generative-Adversarial** | adversarial.yml | Quality control at each node |
| Production | **Factory** | factory.yml | Linear BUILD → INSPECT → PR |

## Design

### Step types (4, reduced from 7)

| Type | Replaces | What it does |
|------|----------|-------------|
| `agent` | agent | Invoke `claude -p` with constrained tools and structured output |
| `run` | shell, gate, watch | Execute commands. Flags: `match` (file filter), `poll` (watch), `check` (gate groups) |
| `branch` | route | Branch on structured output (approve/rework/escalate) with iteration cap |
| `fan` | loop, parallel | Collection processing. `mode: parallel\|sequential\|loop`, `until` for termination |

Rationale: `shell` was `gate` without file matching; `watch` was `shell` in a polling loop; `route` was an if-statement. Collapsing preserves all pipeline semantics with half the parser surface.

### Middleware

`retry(n)`, `timeout(ms)`, `log(path)`, `manifest()`, `on_fail(action)`.

**Resolution order** (outside-in): `log(retry(timeout(step)))`.
- `timeout` resets per retry attempt
- `on_fail` fires only after all retries exhaust
- `log` records each attempt with `attempt: N` field

### Variable interpolation

Simple substitution only: `${scope.field}`. No filters, pipes, or expressions.
- Scopes: `config.*`, `spec.*`, `manifest.*`, `steps.{name}.*`, `loop.*`
- Unset variable → runtime error (fail-fast)
- Complex transformations → computed `context` maps in runtime code

### Agent invocation

Direct `claude -p` subprocess. No provider abstraction until a second provider materializes.

No session continuity in v1. Context passing via prompt injection — build diff, errors, and manifest piped into subsequent agent steps as `context` maps.

### Gate philosophy

Gates are **preflight checks** — speed optimization, not correctness guarantees.
- Local: `cargo check`, `clippy` (fast, catches obvious errors)
- CI: full test suite, cross-platform, coverage, integration
- Gate group renamed from `static`/`ci` to `preflight` (local only)

### Cross-cutting runtime concerns

| Concern | Automatic | Pipeline author declares |
|---------|-----------|------------------------|
| Manifest lifecycle | Create, update, finalize after each step | Output schemas |
| Governance logging | Append to `.harness/{skill}.governance.jsonl` | — |
| Gate execution | Read `gates.yml`, match files, run | Which gate groups |
| Worktree management | Create/cleanup for isolated steps | `isolation: worktree` |
| Cost control | Enforce per-step | `max_turns` |

## Plan

Implementation decomposed into 5 child specs:

- [ ] **061 Pipeline Engine Core** — YAML schema, parser, 4-type step executor, middleware, variable interpolation, `synodic harness validate` and `synodic harness run`
- [ ] **062 Gate System** — `gates.yml` schema, file-match filtering, preflight execution, integration with `run` step
- [ ] **063 Pipeline Definitions** — Factory, fractal, swarm, adversarial YAMLs + prompt templates + output schemas
- [ ] **064 Algorithmic Commands** — `synodic fractal` (complexity, gate, schedule, reunify, prune) + `synodic swarm` (checkpoint, prune) CLI commands
- [ ] **065 Skill Migration** — SKILL.md → pipeline YAML shims, prompt template extraction, governance log compatibility

Execution order: 061 → 062 + 064 (parallel) → 063 → 065

## Test

- [ ] All 4 step types parse and execute correctly (061)
- [ ] Middleware interactions behave per resolution order spec (061)
- [ ] Gate file-match filtering and preflight execution (062)
- [ ] All 4 pipeline YAMLs validate and encode SKILL.md semantics (063)
- [ ] All 7 algorithmic commands produce correct JSON output (064)
- [ ] Migrated skills produce identical governance log format (065)
- [ ] End-to-end: factory pipeline on a trivial spec

## Notes

### Why YAML, not code?

- Modifiable without recompilation
- Readable by agents (an agent can reason about pipeline structure)
- Statically validatable (schema check before execution)
- Industry pattern (GitHub Actions, Dagger, CI systems)

DSL complexity is bounded: no user-defined functions, macros, arithmetic, or string manipulation. `synodic harness validate` catches errors before execution.

### Relationship to existing specs

- **Supersedes 057** (CI feedback loop): CI monitoring is a `run` step with `poll`
- **Evolves 044** (factory MVP): Same pipeline, deterministic orchestration
- **Enables 049** (factory test harness): Code pipelines are testable
- **Enables 052** (fractal + factory composition): Both are pipelines sharing gates
- **Enables 056** (harness bug fixes): Configurable gates replace brittle static_gate.sh

### Relationship to SKILL.md files

SKILL.md files become thin shims after migration (spec 065):
```
/factory run <spec> → synodic harness run --pipeline factory --spec <spec>
/fractal decompose <spec> → synodic harness run --pipeline fractal --spec <spec>
/swarm run <spec> → synodic harness run --pipeline swarm --spec <spec>
/adversarial run <spec> → synodic harness run --pipeline adversarial --spec <spec>
```

### Future: Coordination substrates

Child specs 059 (Context Mesh) and 060 (Stigmergic Coordination) explore optional coordination layers. Both are **draft** pending evidence that the pipeline engine alone is insufficient.

### Issues resolved (2026-03-23)

All 9 issues from the 2026-03-22 logical correctness evaluation have been addressed:

1. **7→4 step types** — `shell`/`gate`/`watch` → `run`; `route` → `branch`; `loop`/`parallel` → `fan`
2. **Middleware semantics defined** — Outside-in resolution order with explicit interaction rules
3. **Variable interpolation constrained** — `${scope.field}` only, fail-fast on unset, no filters
4. **Provider abstraction removed** — Direct `claude -p`, add abstraction when needed
5. **YAML DSL bounded** — No functions/macros/arithmetic; `synodic harness validate` pre-checks
6. **Session continuity removed from v1** — Context passing via prompt injection instead
7. **Gates ≠ CI** — Preflight only (fast checks); CI runs the full matrix
8. **Decomposed into 5 child specs** — 061 (engine), 062 (gates), 063 (pipelines), 064 (algorithms), 065 (migration)
9. **Citation grounded** — Replaced vague "industry consensus" with specific observed failures