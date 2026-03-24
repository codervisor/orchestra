---
status: planned
created: 2026-03-19
priority: high
tags:
- factory
- fractal
- orchestration
- patterns
- composition
depends_on:
- 044-factory-skill-mvp
- 050-fractal-algorithmic-spine
parent: 037-coding-factory-vision
created_at: 2026-03-19T05:26:38.293431754Z
updated_at: 2026-03-19T05:26:38.293431754Z
---

# Fractal + Factory Composition — Orchestration Patterns

## Overview

Factory (BUILD → INSPECT) and Fractal (DECOMPOSE → SOLVE → REUNIFY) are complementary primitives. This spec defines how they compose: when to use each alone, when to combine them, and what the combined orchestration looks like.

## Design

### Decision Rule: When to Use What

```
Is the spec atomic? (single concern, one agent can hold full context)
  YES → /factory run <spec>       (BUILD → INSPECT)
  NO  → /fractal decompose <spec> (DECOMPOSE → leaves → SOLVE → REUNIFY)
            └→ each leaf solved by: /factory run <leaf-spec>
```

Fractal handles decomposition and integration. Factory handles quality at each leaf.

### Composition Pattern

```
/fractal decompose specs/large-feature/README.md

Fractal DECOMPOSE
  └─ leaf-a/spec.md  →  /factory run  →  BUILD → INSPECT → approved branch
  └─ leaf-b/spec.md  →  /factory run  →  BUILD → INSPECT → approved branch
  └─ leaf-c/spec.md  →  /factory run  →  BUILD → INSPECT → approved branch

Fractal REUNIFY
  └─ merge leaf branches (git merge-tree)
  └─ detect interface conflicts (AST check)
  └─ resolve semantic conflicts (AI only if needed)
  └─ produce integration PR
```

### Why This Layering

- Fractal's REUNIFY already uses git merge-tree for code integration
- Factory's INSPECT provides adversarial review *before* integration — catching defects at the leaf level, not after merge
- Each leaf gets independent governance (factory review log)
- The integration itself gets a governance review (harness review on the merged diff)

### Routing Logic

The fractal skill's SOLVE step is parameterized:

```
solve_mode: "direct"   → spawn a general-purpose SOLVE subagent (current behavior)
solve_mode: "factory"  → invoke /factory run on the leaf spec (new behavior)
```

`solve_mode: "factory"` is recommended when:
- Leaf spec has a Test section with verifiable criteria
- Leaf requires > ~200 lines of implementation
- Quality gate matters more than speed

`solve_mode: "direct"` is preferred when:
- Leaf is a pure refactor or text change
- No test infrastructure available
- Speed is the priority (e.g., documentation, config changes)

### Governance Integration

Each layer produces its own governance record:
- Each factory run → `factory.governance.jsonl` entry (per leaf)
- Fractal reunify → `fractal.governance.jsonl` entry (integration review)
- Combined run → parent entry linking all child review IDs

This gives full traceability: from the original spec to each leaf's build/inspect history to the final integration.

## Plan

- [ ] Add `solve_mode` parameter to fractal SOLVE step in `skills/fractal/SKILL.md`
- [ ] Implement factory invocation path: when `solve_mode=factory`, fractal SOLVE spawns factory subagent instead of direct SOLVE
- [ ] Test composition on a 3-leaf example spec
- [ ] Verify governance records are produced at both leaf and integration levels
- [ ] Document decision rule in `skills/fractal/SKILL.md` (when to use factory vs. direct)

## Test

- [ ] `/fractal decompose` on a 3-part spec with `solve_mode=factory` produces 3 factory runs
- [ ] Each leaf produces a `.factory/{work-id}/manifest.json`
- [ ] Fractal REUNIFY successfully merges the 3 factory branches
- [ ] Final PR contains changes from all 3 leaves with no conflicts (clean case)
- [ ] Governance log contains entries at both leaf level (factory) and integration level (fractal)

## Notes

- Depends on 044 (factory skill proven) and 050 (fractal algorithmic spine proven) being complete
- This is Phase 3 of the production roadmap (spec 051)
- The `solve_mode` parameter is backward-compatible — existing fractal runs are unaffected
