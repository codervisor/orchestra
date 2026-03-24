---
name: factory
description: "Coding factory — transforms a spec into a reviewed PR via BUILD → INSPECT pipeline with adversarial review. Use when you need to implement a spec end-to-end with independent code review, or when the user invokes /factory run <spec-path>."
---

# Factory Skill

Linear BUILD → INSPECT pipeline for spec-driven development. Rework up to 3 times if INSPECT finds issues.

## Usage

```
/factory run <spec-path>
```

Example:
```
/factory run specs/044-factory-skill-mvp/README.md
```

## Execution

This skill is orchestrated by the pipeline engine. When invoked, execute:

```
synodic harness run --pipeline factory --spec <spec-path>
```

Pipeline definition: `.harness/pipelines/factory.yml`
Prompt templates: `skills/factory/prompts/`
Output schemas: `schemas/build-report.json`, `schemas/inspect-verdict.json`

## Pipeline Steps

1. **BUILD** — Agent implements the spec in an isolated worktree
2. **STATIC GATE** — Preflight checks (cargo check, clippy, language-specific linters)
3. **INSPECT** — Adversarial review against 5 dimensions (completeness, correctness, security, conformance, quality)
4. **ROUTE** — APPROVE → PR, REWORK → BUILD (max 3 attempts), EXHAUST → escalate
5. **CREATE PR** — Push branch and create pull request

## Governance

- Checkpoint map: STATIC GATE → Layer 1, INSPECT → Layer 2, Escalate → Layer 3
- Governance log: `.harness/factory.governance.jsonl`
- See [HARNESS.md](../../HARNESS.md) for the governance protocol
