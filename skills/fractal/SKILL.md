---
name: fractal
description: "Fractal decomposition — recursively split a complex task into orthogonal sub-specs, solve each leaf independently via subagents, then reunify results bottom-up. Use when a task is too large for a single agent pass, when you need to decompose a problem into independently-solvable pieces, or when the user invokes /fractal decompose <task-or-spec-path>."
---

# Fractal Decomposition Skill

Recursive DECOMPOSE → SOLVE → REUNIFY tree for complex tasks that are too large for a single agent pass.

## Usage

```
/fractal decompose <task-or-spec-path>
```

Example:
```
/fractal decompose specs/058-code-harness-orchestration/README.md
```

## Execution

This skill is orchestrated by the pipeline engine. When invoked, execute:

```
synodic harness run --pipeline fractal --spec <spec-path>
```

Pipeline definition: `.harness/pipelines/fractal.yml`
Prompt templates: `skills/fractal/prompts/`
Output schemas: `schemas/decompose-verdict.json`, `schemas/solve-report.json`

## Pipeline Steps

1. **COMPLEXITY CHECK** — Deterministic scoring; auto-LEAF if below threshold
2. **DECOMPOSE** — Recursive split into orthogonal sub-tasks (fan/loop)
3. **DECOMPOSE GATE** — TF-IDF orthogonality, cycle detection, budget allocation
4. **SCHEDULE** — DAG topological sort into parallel execution waves
5. **SOLVE** — Fan-out leaf solving in isolated worktrees
6. **SOLVE GATE** — Preflight checks per leaf
7. **REUNIFY** — Algorithmic git merge + AI conflict resolution
8. **PRUNE** — Set cover redundancy detection
9. **CREATE PR** — Push and create pull request

## Algorithmic Spine

Deterministic CLI commands (zero LLM cost):
- `synodic fractal complexity` — Score spec complexity
- `synodic fractal gate` — TF-IDF orthogonality + cycle detection
- `synodic fractal schedule` — Topological sort → waves
- `synodic fractal reunify` — git merge-tree conflict detection
- `synodic fractal prune` — Set cover redundancy

## Governance

- Checkpoint map: DECOMPOSE GATE → Layer 1, SOLVE GATE → Layer 1/2, REUNIFY → Layer 2
- Governance log: `.harness/fractal.governance.jsonl`
- See [HARNESS.md](../../HARNESS.md) for the governance protocol
