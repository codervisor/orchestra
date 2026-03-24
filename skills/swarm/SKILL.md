---
name: swarm
description: "Speculative swarm — forks N agents to explore divergent strategies for the same task, cross-pollinates insights at checkpoints, prunes convergent branches, and fuses the best fragments into a composite result. Use when a problem has multiple viable approaches and you want to explore them simultaneously, or when the user invokes /swarm run <spec-path>."
---

# Speculative Swarm Skill

Parallel FORK → CHECKPOINT → PRUNE → MERGE pipeline for divergent exploration.

## Usage

```
/swarm run <spec-path>
```

Example:
```
/swarm run specs/044-factory-skill-mvp/README.md
```

## Execution

This skill is orchestrated by the pipeline engine. When invoked, execute:

```
synodic harness run --pipeline swarm --spec <spec-path>
```

Pipeline definition: `.harness/pipelines/swarm.yml`
Prompt templates: `skills/swarm/prompts/`
Output schemas: `schemas/strategy-set.json`, `schemas/branch-report.json`, `schemas/merge-report.json`

## Pipeline Steps

1. **STRATEGIZE** — Generate N divergent strategies (algorithm, architecture, data structure, paradigm)
2. **EXPLORE** — Fan-out branch solving in isolated worktrees (parallel)
3. **CHECKPOINT** — Jaccard similarity on changed file sets; cross-pollinate insights
4. **PRUNE** — Kill convergent branches (similarity > threshold); min 2 survivors
5. **MERGE** — Fragment-fusion of best parts from surviving branches
6. **MERGE GATE** — Preflight checks on merged result
7. **CREATE PR** — Push and create pull request

## Algorithmic Spine

Deterministic CLI commands (zero LLM cost):
- `synodic swarm checkpoint` — Pairwise Jaccard similarity + cross-pollination
- `synodic swarm prune` — Convergence detection with min 2 survivors

## Governance

- Checkpoint map: CHECKPOINT → Layer 1, MERGE GATE → Layer 2
- Governance log: `.harness/swarm.governance.jsonl`
- See [HARNESS.md](../../HARNESS.md) for the governance protocol
