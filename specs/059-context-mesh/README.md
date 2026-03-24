---
status: draft
created: 2026-03-22
priority: high
tags:
- harness
- context-mesh
- coordination
- dag
- substrate
parent: '058'
created_at: 2026-03-22T21:03:06.121406604Z
updated_at: 2026-03-22T21:46:53.778438186Z
transitions:
- status: draft
  at: 2026-03-22T21:46:53.778438186Z
---

# Context Mesh: DAG-Based Knowledge Substrate for Agent Coordination

## Overview

In traditional orchestration, information flows through an orchestrator hub (lossy, bottlenecked). Context Mesh eliminates the hub — it's a **routerless, managerless global data bus** where all design constraints, research findings, and intermediate artifacts exist as nodes in a DAG. Agents read and write directly to the mesh. The harness guards the DAG's structural integrity.

This is the "omniscient nervous system" of the AI Harness — the foundation layer that all pipelines operate on.

**AI-native property exploited:** Zero context transfer cost. When Agent B needs Agent A's findings, it reads a node — no summarization loss, no meeting overhead, no context window pollution from relay.

## Design

### DAG structure

The mesh is a directed acyclic graph stored on disk:

```
.harness/mesh/
├── mesh.json                 # DAG adjacency + node metadata index
├── nodes/
│   ├── {hash}.md             # Node content (constraint, finding, artifact)
│   └── ...
└── markers/                  # Stigmergic markers (cross-ref with 060)
```

Each node has: `id`, `type` (constraint | finding | artifact | decision), `created_by` (pipeline + step), `depends_on` (edges), `tags`, `content_hash`.

### Node types

| Type | What it captures | Written by | Example |
|------|-----------------|------------|---------|
| `constraint` | Design requirement or invariant | Spec, human, or crystallized rule | "API must be backward-compatible" |
| `finding` | Research result or discovered fact | Swarm branch, fractal decompose | "Library X doesn't support async" |
| `artifact` | Produced code, design, or analysis | Factory BUILD, fractal SOLVE | "auth module at commit abc123" |
| `decision` | Architectural choice with rationale | Route step, human | "Chose strategy-2 because..." |

### Gap detection (harness responsibility)

The harness periodically scans the DAG for structural gaps:

- **Dangling dependencies**: Node A depends on B, but B doesn't exist → spawn agent to fill B
- **Stale nodes**: Artifact node older than its constraint → flag for re-evaluation
- **Orphan clusters**: Subgraph disconnected from any active pipeline → surface for human review
- **Coverage gaps**: Spec requirements not traced to any artifact node → flag as unimplemented

Gap detection is deterministic (graph traversal), not LLM-based. When a gap is found, the harness can auto-spawn a pipeline to fill it — a factory run for a missing artifact, a swarm run for an unexplored constraint.

### Conflict resolution

When two agents write conflicting nodes (e.g., two decisions about the same architectural choice):

1. **Last-write-wins** for artifacts (git handles this via branches)
2. **Flag-and-surface** for decisions and constraints — human resolves
3. **Merge** for findings — both can coexist as evidence

### How pipelines interact with the mesh

- **Read**: Any agent step can query the mesh for relevant context (injected into prompt via `context: mesh.query("tag:auth AND type:constraint")`)
- **Write**: Agent output schemas include optional `mesh_nodes` field — the harness extracts and persists them
- **Subscribe**: Stigmergic watchers (060) can trigger on mesh node creation/update

### Relationship to CLAUDE.md / AGENTS.md

The mesh is the structured, queryable version of what AGENTS.md tries to be. Where AGENTS.md is a flat file that rots, the mesh is a living DAG that evolves with the project. OpenAI's harness engineering insight: "If information isn't discoverable to the agent, it's illegible." The mesh makes ALL project knowledge discoverable.

## Plan

- [ ] Define mesh.json schema (DAG adjacency list, node metadata)
- [ ] Define node types and required fields
- [ ] Implement mesh read/write commands (`synodic mesh add`, `synodic mesh query`)
- [ ] Implement gap detection algorithm (dangling deps, stale nodes, coverage gaps)
- [ ] Implement mesh context injection for agent steps (YAML `context: mesh.query(...)`)
- [ ] Implement auto-spawn on gap detection
- [ ] Integration: factory pipeline writes artifact nodes after BUILD
- [ ] Integration: swarm pipeline writes finding/decision nodes after merge

## Test

- [ ] DAG operations: add node, add edge, query by type/tag
- [ ] Gap detection: dangling dep detected, stale node flagged
- [ ] Conflict resolution: two conflicting decisions flagged for human review
- [ ] Pipeline integration: factory BUILD output creates artifact node in mesh
- [ ] Context injection: agent prompt includes relevant mesh nodes

## Notes

### Not a vector database

The mesh is a typed DAG with explicit edges, not an embedding-based retrieval system. Retrieval is structural (graph traversal + tag filtering), not semantic similarity. This is intentional — structural queries are deterministic and auditable. Semantic search can be added later as an optional query mode.

### Scope: project-level, not global

Each project has its own mesh in `.harness/mesh/`. Cross-project mesh sharing is out of scope for now.

### Logical Correctness Evaluation (2026-03-22)

#### Correctness Issues

1. **"Routerless, managerless" contradicted by design**: The overview claims a "routerless, managerless global data bus." The design then describes the harness guarding DAG integrity, periodically scanning for gaps, and auto-spawning agents. The harness IS the manager — the claim is misleading.

2. **Circular dependency in auto-spawn**: Gap detection auto-spawns pipelines, but spawning requires the pipeline engine (058). The mesh is Layer 1 that pipelines run ON, yet auto-spawn depends on the pipeline engine (Layer 2). Layer 1 depends on Layer 2 which depends on Layer 1.

3. **"Zero context transfer cost" is incorrect**: Reading mesh nodes into an agent context window consumes tokens proportional to node content. The cost is not zero — it's shifted from relay to mesh reads. Correct claim: "zero summarization loss," not "zero cost."

4. **mesh.json concurrency hazard**: Single JSON file for the DAG. Multiple parallel agents writing simultaneously will cause file-level conflicts. Conflict resolution addresses semantic conflicts but ignores file-level contention.

5. **Missing test for auto-spawn**: Auto-spawn on gap detection is a key feature with no test case.

#### Systematic / Design Issues

6. **Over-engineering: solves a problem that doesn't exist yet.** The current codebase has no evidence that information loss between agents is a real bottleneck. Factory (044/058) works by reading specs directly. Fractal passes context through manifests. The mesh is a full knowledge graph with gap detection, auto-spawning, and conflict resolution for a problem nobody has demonstrated. This is classic YAGNI — build the DAG when you have concrete evidence of context loss, not before.

7. **God-object coupling pattern.** The mesh captures everything: constraints, findings, artifacts, decisions. Every pipeline writes to it, every agent reads from it. This makes the mesh a single point of coupling even if not a single point of failure. Any schema change to mesh.json or node format ripples across all pipelines and agent prompts. A simpler per-pipeline artifact store with optional cross-references would achieve the same coordination with less coupling.

8. **Gap detection assumes omniscience it can't have.** "Coverage gaps: Spec requirements not traced to any artifact node → flag as unimplemented." This requires either (a) manual linking of every spec requirement to artifact nodes (defeating automation) or (b) semantic understanding of whether an artifact satisfies a requirement. The spec explicitly rejects semantic search ("not a vector database"). Structural graph traversal alone cannot determine whether artifact X satisfies requirement Y — the gap detection promise is undeliverable with the stated approach.

9. **"Stale nodes" heuristic is unsound.** "Artifact node older than its constraint → flag for re-evaluation." An artifact can predate a constraint and still satisfy it (constraint added after compliant code already existed). Temporal ordering does not imply staleness. This will generate false positives that erode trust in the system.

10. **Auto-spawn is dangerous without throttling.** A poorly connected DAG could trigger cascading auto-spawns (gap found → spawn pipeline → pipeline writes nodes → new gaps detected → more spawns). There's no budget limit, approval gate, or rate limiting on auto-spawns. Combined with the circular L1/L2 dependency (#2), this is a runaway resource consumption risk.

11. **Query language undefined.** Pipeline YAML examples use `mesh.query("tag:auth AND type:constraint")` but no query language is specified. Is it boolean tag filtering? Graph traversal? Regex? Without a defined query semantics, pipeline authors can't reason about what their queries will return.

12. **Node type system is both too rigid and too vague.** Only 4 types: constraint, finding, artifact, decision. Missing: questions, risks, alternatives, trade-offs, hypotheses. Meanwhile, the boundary between types is fuzzy — what's the difference between a "finding" and a "constraint" that was discovered rather than specified? The type system will either be extended repeatedly (schema churn) or worked around (everything becomes "finding").

13. **Layer 1 classification is aspirational, not structural.** The mesh is called "always-on infrastructure" but 058's four pipeline YAMLs are complete without any mesh interaction. The mesh is bolted on, not foundational. Nothing in factory.yml, fractal.yml, swarm.yml, or adversarial.yml requires the mesh to function. If Layer 1 is optional, it's not infrastructure — it's a feature.
