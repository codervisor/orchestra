---
status: planned
created: 2026-03-24
priority: critical
tags:
- orchestration
- typescript
- architecture
- state-machine
created_at: 2026-03-24T23:03:46.058740697Z
updated_at: 2026-03-24T23:03:46.058740697Z
---

# TypeScript Graph-Based Orchestration Engine

## Overview

Replace the Rust pipeline engine with a lightweight TypeScript state-machine/graph executor. Keep Rust only for algorithmic tools (TF-IDF, toposort, etc.) called via child_process as CLI binaries. The TS package is npm-distributable and serves as the primary orchestration runtime.

### Motivation

The current Rust pipeline engine (~1500+ lines) reimplements workflow orchestration (step sequencing, variable interpolation, branching, retries) that is fundamentally I/O-bound — waiting on AI agent calls and shell commands. Rust's performance strengths are irrelevant here. The valuable Rust code is ~230 lines of pure algorithms.

### Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Primary language | TypeScript | npm distribution, async/await native, fast iteration |
| Rust integration | child_process (CLI) | JSON-in/JSON-out already works, ~10-25ms overhead is negligible for orchestration-level calls |
| napi-rs | **Not used** | Overkill — adds cross-platform build matrix (~10-15 platform packages) for ~5 calls per orchestration cycle |
| WASM | **Not used** | Blocked by `reunify.rs` shelling out to git; regex crate bloats binary |
| State machine | Directed graph (DAG) | Natural representation of orchestration patterns |
| Parallelism | Native async/await + Promise.all | Free with TS, unlike Rust Fan step which runs sequentially |

## Design

### Architecture: Three Layers

```
┌─────────────────────────────────────────┐
│  Graph Definition (YAML/JSON)           │  Declarative state machine
├─────────────────────────────────────────┤
│  TS Graph Executor (~300-500 lines)     │  Lightweight DAG walker with async
├─────────────────────────────────────────┤
│  Algorithmic Tools (Rust CLIs)          │  Focused, pure-function binaries
└─────────────────────────────────────────┘
```

### Layer 1: Graph Definition Schema

Each orchestration pattern is a directed graph where:
- **Nodes** = execution units (agent call, shell command, algorithm invocation)
- **Edges** = transitions with optional conditions
- **Subgraphs** = reusable pattern compositions

```yaml
name: factory
nodes:
  build:
    type: agent
    prompt: skills/factory/build.md
    tools: [Read, Edit, Write, Bash]

  gate:
    type: run
    command: "npm test && npm run lint"
    match: ["*.ts", "*.rs"]

  inspect:
    type: agent
    prompt: skills/factory/inspect.md

  route:
    type: decision
    input: inspect.verdict

edges:
  - from: build -> gate -> inspect -> route
  - from: route
    on: approve -> done
  - from: route
    on: rework -> build
    max: 3
    exhaust: escalate
```

### Layer 2: TypeScript Graph Executor

Core responsibilities:
1. **Parse** graph definition (YAML → typed graph structure)
2. **Walk** the DAG respecting edges and conditions
3. **Execute** nodes (spawn agents, run commands, call Rust CLIs)
4. **Propagate** context between nodes (output of node A available to node B)
5. **Handle** branching, loops with max iterations, parallel fan-out

Key types:

```typescript
interface GraphNode {
  id: string;
  type: "agent" | "run" | "decision" | "fan";
  config: AgentConfig | RunConfig | DecisionConfig | FanConfig;
}

interface GraphEdge {
  from: string;
  to: string;
  condition?: string;   // e.g., "approve", "rework"
  max?: number;         // max traversals (loop cap)
  exhaust?: string;     // action when max exceeded
}

interface OrchestrationGraph {
  name: string;
  nodes: Map<string, GraphNode>;
  edges: GraphEdge[];
  context: ExecutionContext;
}

interface ExecutionContext {
  config: Record<string, unknown>;
  results: Map<string, NodeResult>;
}
```

Executor core (~300 lines estimated):

```typescript
async function execute(graph: OrchestrationGraph): Promise<ExecutionResult> {
  const visited = new Map<string, number>();
  let current = findEntryNode(graph);

  while (current) {
    const result = await executeNode(current, graph.context);
    graph.context.results.set(current.id, result);

    const nextEdge = resolveEdge(current, result, graph.edges, visited);
    if (!nextEdge) break;

    if (nextEdge.max && (visited.get(edgeKey(nextEdge)) ?? 0) >= nextEdge.max) {
      return handleExhaust(nextEdge, graph.context);
    }

    visited.set(edgeKey(nextEdge), (visited.get(edgeKey(nextEdge)) ?? 0) + 1);
    current = graph.nodes.get(nextEdge.to);
  }

  return { status: "completed", context: graph.context };
}
```

Fan-out with native parallelism:

```typescript
async function executeFan(node: FanNode, ctx: ExecutionContext): Promise<NodeResult[]> {
  const items = resolveCollection(node.config.over, ctx);
  if (node.config.mode === "parallel") {
    return Promise.all(items.map(item =>
      executeNode(node.config.body, { ...ctx, item })
    ));
  }
  const results = [];
  for (const item of items) {
    results.push(await executeNode(node.config.body, { ...ctx, item }));
  }
  return results;
}
```

### Layer 3: Rust Algorithmic Tools (preserved)

Keep as standalone CLI binaries, called from TS via child_process:

| Tool | Purpose | Input/Output |
|------|---------|-------------|
| `orchestra-tfidf` | TF-IDF cosine similarity for orthogonality check | JSON → JSON |
| `orchestra-toposort` | Kahn's toposort for cycle detection + wave scheduling | JSON → JSON |
| `orchestra-complexity` | Weighted complexity scoring | JSON → JSON |
| `orchestra-reunify` | 3-way merge conflict detection | JSON → JSON |
| `orchestra-budget` | Proportional budget allocation | JSON → JSON |
| `orchestra-jaccard` | Quick set similarity | JSON → JSON |

TS wrapper:

```typescript
import { execFile } from "node:child_process/promises";

async function callAlgorithm<I, O>(tool: string, input: I): Promise<O> {
  const { stdout } = await execFile(`orchestra-${tool}`, [], {
    input: JSON.stringify(input),
  });
  return JSON.parse(stdout);
}
```

### Pattern Implementations as Graphs

**Factory** (linear with retry):
```
build → gate → inspect → route ─approve─→ done
                           └──rework──→ build (max 3)
```

**Fractal** (recursive DAG):
```
complexity ──simple──→ solve → done
    └──complex──→ decompose → gate → schedule → fan(solve) → reunify → prune → done
```

**Swarm** (parallel exploration):
```
strategize → fan(explore, parallel) → checkpoint-loop → merge → gate → done
```

### npm Package Structure

```
@orchestra/core          # Graph executor + types
@orchestra/cli           # CLI entry point (orchestra run factory.yml)
@orchestra/algorithms    # Rust binary wrapper (optional dep with platform binaries)
```

Distribution options for Rust binaries:
- **Option A**: Platform-specific optional deps (like esbuild/turbo pattern)
- **Option B**: Download on first use
- **Option C**: Pure TS fallback implementations for portability

## Implementation Plan

### Phase 1: Core Graph Executor (TS)
- [ ] Define graph schema types and YAML parser
- [ ] Implement DAG walker with async node execution
- [ ] Decision routing with edge conditions and loop caps
- [ ] Fan-out with Promise.all for parallel mode
- [ ] Context propagation between nodes
- [ ] Variable interpolation in TS

### Phase 2: Node Executors
- [ ] Agent executor: spawn Claude with prompt/tools/constraints
- [ ] Run executor: shell command with file-match filtering
- [ ] Decision executor: verdict-based edge selection
- [ ] Fan executor: parallel and sequential modes over collections

### Phase 3: Rust Algorithm Integration
- [ ] Refactor Rust into focused CLI binaries (split orchestra-core)
- [ ] TS wrapper module with typed callAlgorithm helper
- [ ] Platform binary distribution via npm optional deps

### Phase 4: Pattern Migration
- [ ] Convert factory.yml to graph format
- [ ] Convert fractal.yml to graph format (recursive subgraph)
- [ ] Convert swarm.yml to graph format
- [ ] Integration tests validating equivalent behavior

### Phase 5: npm Distribution
- [ ] Package structure (@orchestra/core, @orchestra/cli)
- [ ] Platform binary packaging for Rust tools
- [ ] CI/CD for cross-platform builds and npm publish

## What Gets Deleted

- `rust/orchestra-core/src/pipeline/` — executor, schema, vars, validate (replaced by TS)
- `rust/orchestra-cli/` — replaced by TS CLI
- Generic pipeline YAML format — replaced by graph schema

## What Gets Kept

- `rust/orchestra-core/src/fractal/decompose.rs` — TF-IDF, toposort, cycles, budget, complexity
- `rust/orchestra-core/src/fractal/schedule.rs` — wave scheduling
- `rust/orchestra-core/src/fractal/reunify.rs` — merge conflict detection
- `rust/orchestra-core/src/swarm/mod.rs` — Jaccard similarity
- `skills/` — AI agent prompts (unchanged)
- `schemas/` — structured output schemas (unchanged)

## Risks & Mitigations

| Risk | Mitigation |
|------|-----------|
| Rust binary distribution complexity | Offer pure-TS fallback for core algorithms; Rust is optimization |
| Graph schema expressiveness gaps | Start by converting existing patterns; extend schema as needed |
| Losing type safety at Rust boundary | Zod schemas for runtime validation of JSON-in/JSON-out |
| Recursive subgraphs (fractal) | Allow nodes to reference other graph definitions |
