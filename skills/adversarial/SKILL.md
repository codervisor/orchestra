---
name: adversarial
description: "Generative-adversarial coordination — locks a generator and critic agent in an escalating quality loop. The critic actively tries to break the generator's output with increasing sophistication. Quality emerges from adversarial pressure, not checklist compliance. Use when you need deep quality hardening beyond standard review, or when the user invokes /adversarial run <spec-path>."
---

# Generative-Adversarial Skill

Iterative GENERATE ↔ ATTACK loop with escalating critic modes for deep quality hardening.

## Usage

```
/adversarial run <spec-path>
```

Example:
```
/adversarial run specs/044-factory-skill-mvp/README.md
```

## Execution

This skill is orchestrated by the pipeline engine. When invoked, execute:

```
synodic harness run --pipeline adversarial --spec <spec-path>
```

Pipeline definition: `.harness/pipelines/adversarial.yml`
Prompt templates: `skills/adversarial/prompts/`
Output schemas: `schemas/generate-report.json`, `schemas/attack-report.json`

## Pipeline Steps

1. **ADVERSARIAL LOOP** (fan/loop, max 5 iterations):
   - **GENERATE** — Implement spec + harden against anticipated attacks (worktree)
   - **STATIC GATE** — Preflight checks (cap 2 rework)
   - **ATTACK** — Adversarial critic tries to break output (read-only)
2. **Termination** — 2 consecutive clean rounds OR max rounds OR quality plateau
3. **CREATE PR** — Push and create pull request

## Escalation Ladder

Critic modes escalate progressively:
1. `syntax-and-types` — Type errors, API misuse, obvious bugs
2. `edge-cases` — Boundary conditions, empty inputs, overflow
3. `concurrency-safety` — Race conditions, deadlocks, data races
4. `adversarial-inputs` — Injection, malformed data, fuzzing
5. `semantic-analysis` — Logic correctness, invariant violations

## Governance

- Checkpoint map: STATIC GATE → Layer 1, ATTACK → Layer 2, Plateau → escalate
- Governance log: `.harness/adversarial.governance.jsonl`
- See [HARNESS.md](../../HARNESS.md) for the governance protocol
