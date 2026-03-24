---
status: in-progress
created: 2026-03-16
priority: critical
tags:
- factory
- mvp
- skill
- build
- inspect
- pipeline
parent: 037-coding-factory-vision
depends_on: []
---

# Factory Skill MVP

> **Status**: in-progress · **Priority**: critical · **Created**: 2026-03-16

## Overview

Spec 038 designed the factory MVP as a Rust binary orchestrating Claude Code
subprocesses. The scope review (March 2026) concluded that Claude Code's
built-in subagents already provide the orchestration primitives we need.

This spec replaces 038 with a skill-first approach: a single Claude Code skill
that orchestrates two subagents (BUILD and INSPECT) to transform a spec into a
reviewed PR. No Rust binary, no subprocess JSON parsing, no custom message bus.

The core thesis remains unchanged: adversarial review by a separate agent
instance (fresh context, no builder bias) produces measurably better results
than a single agent run, with acceptable overhead.

## Design

### Skill Architecture

```
/factory run specs/044-factory-skill-mvp/README.md

Main conversation (orchestrator)
  │
  ├─→ BUILD subagent (general-purpose, isolation: worktree)
  │     • Reads spec, implements code
  │     • Runs tests, fixes failures
  │     • Commits to factory/{work-id} branch
  │     • Returns: files changed, test results, tokens used
  │
  ├─→ INSPECT subagent (general-purpose, fresh context)
  │     • Reads diff + spec only (no builder context = adversarial)
  │     • Reviews correctness, security, completeness
  │     • Returns: VERDICT: APPROVE or VERDICT: REWORK + items
  │
  └─→ Orchestration loop
        • If REWORK: re-invoke BUILD with rework items (max 3 loops)
        • If APPROVE: record metrics, create PR via gh
```

### BUILD Subagent

- **Type:** general-purpose with `isolation: worktree`
- **Input:** Spec README.md content, optional rework feedback from prior INSPECT
- **Process:**
  1. Read the spec's Plan section for implementation steps
  2. Implement each step (code changes, new files)
  3. Run tests from the spec's Test section
  4. Commit to `factory/{work-id}` branch
- **Output:** Summary of files changed, test results, commit SHA
- **Quality gate:** Tests pass, code compiles, no syntax errors

### INSPECT Subagent

- **Type:** general-purpose (no worktree — read-only review)
- **Input:** Git diff from BUILD + original spec (no builder context)
- **Process:**
  1. Review diff against spec's acceptance criteria
  2. Check correctness, security, completeness
  3. Return structured verdict
- **Output:** `VERDICT: APPROVE` or `VERDICT: REWORK` with specific items
- **Quality gate:** All review dimensions addressed
- **Rework limit:** Max 3 cycles before escalation to human

### Orchestration Loop

The main conversation acts as the conveyor:

1. Parse spec path from user invocation
2. Generate work ID (`factory-{timestamp}`)
3. Create `.factory/{work-id}/` directory for artifacts
4. Spawn BUILD subagent with spec content
5. On BUILD return: extract diff, record build report
6. Spawn INSPECT subagent with diff + spec (fresh context)
7. Parse verdict from INSPECT response
8. If REWORK and attempts < 3: go to step 4 with rework feedback
9. If APPROVE: create PR via `gh pr create`, record metrics
10. If attempts >= 3: escalate (log failure, skip PR)

### Work Manifest

Stored at `.factory/{work-id}/manifest.json`:

```json
{
  "id": "factory-1710600000",
  "spec": "specs/044-factory-skill-mvp/README.md",
  "status": "approved",
  "branch": "factory/factory-1710600000",
  "attempts": [
    {
      "attempt": 1,
      "build": {
        "files_changed": ["src/lib.rs"],
        "tests_passed": true,
        "commit": "abc1234"
      },
      "inspect": {
        "verdict": "rework",
        "items": ["Missing error handling in parse()"]
      }
    },
    {
      "attempt": 2,
      "build": {
        "files_changed": ["src/lib.rs"],
        "tests_passed": true,
        "commit": "def5678"
      },
      "inspect": {
        "verdict": "approve",
        "items": []
      }
    }
  ],
  "metrics": {
    "cycle_time_seconds": 342,
    "total_attempts": 2,
    "first_pass_yield": false
  }
}
```

### Metrics

Even the MVP measures:

- **Cycle time:** Wall-clock seconds from invocation to approved PR (or escalation)
- **First-pass yield:** Did INSPECT approve on the first attempt? (boolean)
- **Rework count:** How many BUILD-INSPECT loops before approval
- **Attempt history:** Full record of each build/inspect cycle

### Skill Structure

```
skills/factory/
├── SKILL.md                    # Skill definition (AgentSkills.io format)
├── references/
│   └── manifest.schema.json    # Work manifest JSON schema
├── fixtures/
│   └── sample-spec/            # Trivial spec for validation
└── evals/
    ├── evals.json              # Behavioral evals
    └── prompts/                # Eval prompts
```

## Plan

- [x] Create `skills/factory/SKILL.md` with skill definition and orchestration prompt
- [x] Define `manifest.schema.json` for work item tracking
- [x] Implement BUILD subagent prompt (spec reading, implementation, test, commit)
- [x] Implement INSPECT subagent prompt (diff review, verdict protocol)
- [x] Implement orchestration loop (spawn, parse verdict, rework routing, max 3 cycles)
- [x] Implement manifest writing (`.factory/{work-id}/manifest.json` after each cycle)
- [x] Implement PR creation on APPROVE (`gh pr create`)
- [x] Create sample spec fixture for testing
- [x] Create behavioral evals verifying end-to-end flow

## Test

- [ ] `/factory run` on a trivial spec produces a git branch with implementation and a PR
- [ ] INSPECT subagent catches a deliberate bug; rework loop fires and BUILD fixes it
- [ ] Rework limit of 3 is enforced — after 3 failed cycles, escalation occurs
- [ ] Work manifest records full attempt history with build/inspect details
- [ ] INSPECT runs in fresh context (no builder bias leaking through)
- [ ] Two concurrent `/factory run` invocations don't interfere (separate work IDs and branches)

## Notes

This replaces spec 038 (now archived). The key difference is orchestration
via built-in subagents instead of a Rust binary spawning `claude -p`
subprocesses. The workflow is identical — BUILD then INSPECT with rework
loops — but the implementation is radically simpler.

For parallel execution (multiple specs), compose with `/batch` or Agent Teams
rather than reimplementing subprocess management.
