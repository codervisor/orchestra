---
status: complete
created: 2026-03-23
priority: high
tags:
- harness
- cli
- algorithms
depends_on:
- '061'
parent: 058-code-harness-orchestration
created_at: 2026-03-23T00:53:46.635444447Z
updated_at: 2026-03-23T01:18:06.819850343Z
---

# Algorithmic Commands: Fractal and Swarm Deterministic CLI Operations

## Overview

Deterministic CLI commands required by the fractal and swarm pipelines. These are pure algorithms (JSON-in/JSON-out) with zero LLM cost — the "algorithmic spine" that keeps orchestration deterministic while agents handle creative work.

## Design

### Fractal commands

| Command | Input | Output | Algorithm |
|---------|-------|--------|-----------|
| `synodic fractal complexity` | Spec path | `{score, threshold, skip}` | Line count + cyclomatic estimate + dependency fan-out |
| `synodic fractal gate` | Manifest tree | `{valid, violations[]}` | TF-IDF orthogonality check + cycle detection on node DAG |
| `synodic fractal schedule` | Manifest path | `{waves: [[node_id]]}` | Topological sort → wave assignment (parallel within wave) |
| `synodic fractal reunify` | Child node paths | `{merged, conflicts[]}` | `git merge-tree` across child worktrees |
| `synodic fractal prune` | Manifest tree | `{pruned: [node_id]}` | Set cover: remove nodes whose file sets are subsets of others |

### Swarm commands

| Command | Input | Output | Algorithm |
|---------|-------|--------|-----------|
| `synodic swarm checkpoint` | Manifest path | `{similarities: {}, cross_pollination: {}}` | Jaccard similarity on changed file sets between branches |
| `synodic swarm prune` | Manifest + threshold | `{pruned: [], surviving: []}` | Remove branches with similarity > threshold; enforce min 2 survivors |

### Implementation approach

All commands share a pattern:
1. Read JSON input (manifest or spec path)
2. Run deterministic algorithm
3. Write JSON output to stdout
4. Exit 0 on success, non-zero on error

Implemented as subcommands of `synodic fractal` and `synodic swarm` in the existing CLI structure. The `synodic fractal` dispatcher already exists at `cli/synodic/src/cmd/fractal.rs`.

## Plan

- [ ] Implement `synodic fractal complexity` (line count + dependency analysis)
- [ ] Implement `synodic fractal gate` (TF-IDF orthogonality + cycle detection)
- [ ] Implement `synodic fractal schedule` (topological sort → waves)
- [ ] Implement `synodic fractal reunify` (git merge-tree wrapper)
- [ ] Implement `synodic fractal prune` (set cover redundancy)
- [ ] Implement `synodic swarm checkpoint` (Jaccard similarity)
- [ ] Implement `synodic swarm prune` (convergence detection)
- [ ] Add `synodic swarm` subcommand dispatcher

## Test

- [ ] `complexity`: known spec produces expected score
- [ ] `gate`: overlapping nodes detected, cycles rejected
- [ ] `schedule`: DAG with dependencies produces correct wave ordering
- [ ] `reunify`: clean merge succeeds, conflicting merge reports conflicts
- [ ] `prune`: subset nodes removed, non-subsets preserved
- [ ] `checkpoint`: identical file sets → similarity 1.0, disjoint → 0.0
- [ ] `swarm prune`: min 2 survivors enforced even when all converge
- [ ] All commands: malformed input → non-zero exit + error JSON
