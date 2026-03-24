---
status: complete
created: 2026-03-18
priority: high
tags:
- fractal
- algorithms
- performance
- determinism
created_at: 2026-03-19T05:25:30.884614458Z
updated_at: 2026-03-19T05:25:30.884614458Z
completed_at: 2026-03-19T05:25:30.884614458Z
transitions:
- status: complete
  at: 2026-03-19T05:25:30.884614458Z
---

# 050 — Fractal Algorithmic Spine

> Replace AI subagent calls with classical algorithms wherever the operation
> is structurally decidable, reserving AI exclusively for semantic
> input→output transformation.

## Problem

Fractal currently uses AI subagents for five operations:

1. **Decompose** — decide LEAF vs SPLIT, produce child specs
2. **Solve** — implement a leaf sub-problem
3. **Reunify** — merge child solutions into parent
4. **Reunify Rework** — detect and resolve conflicts
5. **Prune** — identify redundant nodes

Of these, only **Decompose (partially)** and **Solve** genuinely require semantic
understanding. The rest — tree scheduling, code merging, conflict detection,
redundancy analysis — have well-known algorithmic solutions that are faster,
cheaper, deterministic, and reproducible.

## Principle

> **AI for semantics, algorithms for structure.**
>
> If the operation can be expressed as a function over syntax trees, diffs,
> dependency graphs, or set operations — it should be an algorithm, not
> a subagent call.

## Current vs. Proposed

| Operation | Current | Proposed Algorithm | Classical Analog |
|-----------|---------|-------------------|------------------|
| Leaf detection | AI decides LEAF/SPLIT | AI decides, but with **complexity scoring** as prior | Decision tree feature importance |
| Decompose gate | Jaccard similarity (basic) | **TF-IDF weighted cosine similarity** + dependency graph cycle detection | Information retrieval + topological sort |
| Solve scheduling | All parallel or all sequential | **DAG-based critical path scheduling** | MapReduce shuffle, job scheduling |
| Code reunification | AI subagent merges | **3-way merge** (git merge-tree) + AST conflict detection | MergeSort merge step |
| Conflict detection | AI reads and flags | **AST diff** + interface signature matching | Graph coloring / constraint satisfaction |
| Redundancy pruning | AI reviews tree | **Diff similarity** (Levenshtein on outputs) + set cover | Set cover problem |
| Budget allocation | Greedy (first-come) | **Knapsack** or proportional allocation by estimated complexity | Dynamic programming |

## Detailed Design

### 1. Complexity Scoring (pre-Decompose)

Before asking AI to decompose, compute a **complexity score** deterministically:

```python
def complexity_score(spec_text: str, repo_context: dict) -> float:
    """Score 0.0 (trivial) to 1.0 (very complex)."""
    signals = {
        "line_count": len(spec_text.splitlines()) / 200,        # normalized
        "term_diversity": len(extract_terms(spec_text)) / 100,  # from existing gate
        "file_fan_out": estimate_files_touched(spec_text, repo_context) / 20,
        "cross_cutting_terms": count_cross_cutting(spec_text) / 10,
    }
    return min(1.0, sum(signals.values()) / len(signals))
```

- If score < `min_complexity` threshold → auto-LEAF, skip AI decompose entirely
- If score > threshold → pass score to AI as a prior ("complexity: 0.73")
- **Algorithm**: weighted feature scoring (same as decision tree split criterion)

### 2. Enhanced Decompose Gate (TF-IDF + DAG)

Replace raw Jaccard with TF-IDF weighted cosine similarity:

```python
from collections import Counter
import math

def tfidf_similarity(scope_a: str, scope_b: str, corpus: list[str]) -> float:
    """TF-IDF weighted cosine similarity between two scopes."""
    # Document frequency across all children scopes
    df = Counter()
    for doc in corpus:
        df.update(set(extract_terms(doc)))

    def tfidf_vec(text):
        terms = extract_terms(text)
        tf = Counter(terms)
        return {t: (tf[t] / len(terms)) * math.log(len(corpus) / (df[t] + 1))
                for t in terms}

    vec_a, vec_b = tfidf_vec(scope_a), tfidf_vec(scope_b)
    common = set(vec_a) & set(vec_b)
    if not common:
        return 0.0
    dot = sum(vec_a[t] * vec_b[t] for t in common)
    mag_a = math.sqrt(sum(v**2 for v in vec_a.values()))
    mag_b = math.sqrt(sum(v**2 for v in vec_b.values()))
    return dot / (mag_a * mag_b) if mag_a and mag_b else 0.0
```

Add **dependency cycle detection** on declared inputs/outputs:

```python
def detect_cycles(children: list[dict]) -> list[dict]:
    """Topological sort to detect circular dependencies."""
    # Build adjacency from inputs → outputs
    graph = {}
    output_map = {}
    for c in children:
        graph[c["slug"]] = []
        for term in extract_terms(c.get("outputs", "")):
            output_map[term] = c["slug"]

    for c in children:
        for term in extract_terms(c.get("inputs", "")):
            if term in output_map and output_map[term] != c["slug"]:
                graph[c["slug"]].append(output_map[term])

    # Kahn's algorithm for cycle detection
    in_degree = {k: 0 for k in graph}
    for node in graph:
        for dep in graph[node]:
            if dep in in_degree:
                in_degree[dep] += 1

    queue = [n for n, d in in_degree.items() if d == 0]
    visited = 0
    while queue:
        node = queue.pop(0)
        visited += 1
        for dep in graph.get(node, []):
            in_degree[dep] -= 1
            if in_degree[dep] == 0:
                queue.append(dep)

    if visited < len(graph):
        return [{"category": "cycle", "description": "Circular dependency detected among children"}]
    return []
```

### 3. DAG-Based Solve Scheduling

Replace the binary parallel/sequential choice with **critical path scheduling**:

```python
def schedule_solves(leaves: list[dict]) -> list[list[str]]:
    """Return solve waves — each wave can run in parallel.

    Uses topological sort on the dependency DAG.
    Like MapReduce shuffle: group independent tasks into parallel waves.
    """
    # Build dependency graph from inputs/outputs
    deps = build_dependency_graph(leaves)

    # Topological sort into waves (BFS layers)
    in_degree = {leaf["slug"]: 0 for leaf in leaves}
    for slug, dep_list in deps.items():
        for dep in dep_list:
            in_degree[slug] += 1

    waves = []
    ready = [s for s, d in in_degree.items() if d == 0]

    while ready:
        waves.append(ready[:])
        next_ready = []
        for slug in ready:
            for dependent in find_dependents(slug, deps):
                in_degree[dependent] -= 1
                if in_degree[dependent] == 0:
                    next_ready.append(dependent)
        ready = next_ready

    return waves
```

This gives us three behaviors from one algorithm:
- **All independent** → single wave (= current parallel mode)
- **All sequential** → one per wave (= current sequential mode)
- **Mixed** → multiple waves with maximal parallelism (new capability)

### 4. Algorithmic Code Reunification

For `output_mode: code`, replace the AI reunify subagent with **git merge-tree**:

```bash
# Three-way merge: base (pre-fractal) vs child branches
git merge-tree $(git merge-base HEAD $child_branch) HEAD $child_branch
```

Process:
1. Each SOLVE subagent works in an isolated worktree (already the case)
2. Each produces a branch with commits prefixed `fractal({slug}):`
3. **Reunification = sequential git merge**, not AI:
   - Sort children by dependency order (from Step 3's DAG)
   - Merge each child branch into the integration branch
   - If `git merge-tree` reports conflicts → those are the CONFLICT items
   - No AI needed for conflict *detection*

AI is only needed for conflict *resolution* when:
- Two children modified the same function with incompatible logic
- Interface contracts don't align semantically (types match but meaning differs)

```python
def algorithmic_reunify(children_branches: list[str], base: str) -> ReunifyResult:
    """Merge child branches using git, fall back to AI only on semantic conflicts."""
    integration = create_branch(f"fractal/reunify-{node_slug}")
    conflicts = []

    for branch in dependency_sorted(children_branches):
        result = git_merge_tree(base, integration, branch)
        if result.clean:
            fast_forward(integration, branch)
        else:
            # Classify conflicts
            for conflict in result.conflicts:
                if is_textual_conflict(conflict):
                    # Try automatic resolution strategies
                    resolved = try_auto_resolve(conflict)  # e.g., take-both for additive changes
                    if resolved:
                        apply_resolution(conflict, resolved)
                        continue
                conflicts.append(conflict)

    if conflicts:
        # ONLY now invoke AI — and only for the specific conflicts
        return ReunifyResult(status="CONFLICT", conflicts=conflicts)
    return ReunifyResult(status="MERGED")
```

### 5. AST-Based Conflict Detection

For interface mismatches, use **AST parsing** instead of AI:

```python
def detect_interface_conflicts(child_results: list[dict]) -> list[dict]:
    """Parse exported interfaces from each child, check compatibility."""
    exports = {}
    for child in child_results:
        for file in child["files"]:
            tree = parse_ast(file)  # tree-sitter or syn for Rust
            for export in extract_exports(tree):
                key = (export.name, export.kind)  # e.g., ("UserAuth", "struct")
                if key in exports:
                    existing = exports[key]
                    if not signatures_compatible(existing, export):
                        yield {
                            "category": "interface",
                            "children": [existing.source, child["slug"]],
                            "description": f"Incompatible signatures for {export.name}"
                        }
                exports[key] = export
```

This catches:
- **[interface]** — signature mismatches (deterministic via AST)
- **[boundary]** — child modified files outside its scope (deterministic via file path check)
- **[redundancy]** — multiple children export the same symbol (deterministic via symbol table)
- **[gap]** — declared output not found in exports (deterministic via contract check)

Only **[interface] semantic mismatches** (types match, meaning doesn't) need AI.

### 6. Algorithmic Pruning

Replace AI pruning with **diff-based redundancy detection**:

```python
def detect_redundant_nodes(tree: dict) -> list[str]:
    """Find nodes whose output is a subset of a sibling's output.

    Uses normalized Levenshtein similarity on outputs.
    Equivalent to set cover: find minimal set of nodes that covers all outputs.
    """
    prunable = []
    for node in tree.siblings():
        for sibling in tree.siblings_of(node):
            if node == sibling:
                continue
            # For code: diff-based (are all of node's changes contained in sibling's?)
            if output_is_subset(node.result, sibling.result):
                prunable.append(node.slug)
    return prunable

def output_is_subset(a: str, b: str) -> bool:
    """Check if output A is fully contained within output B."""
    # For code mode: check if all files changed by A are also changed by B
    # with compatible changes (using git diff --stat)
    a_files = set(changed_files(a))
    b_files = set(changed_files(b))
    return a_files.issubset(b_files)
```

### 7. Budget Allocation (Knapsack)

Replace greedy budget enforcement with **proportional allocation**:

```python
def allocate_budget(node: dict, remaining_budget: int, children_scores: list[float]) -> list[int]:
    """Allocate node budget to children proportional to complexity.

    Simple proportional allocation (avoids full knapsack complexity).
    Each child gets at least 1 node. Remaining distributed by score.
    """
    n = len(children_scores)
    if remaining_budget <= n:
        return [1] * min(n, remaining_budget)

    base = [1] * n  # everyone gets at least 1
    remaining = remaining_budget - n
    total_score = sum(children_scores)

    if total_score == 0:
        # Equal distribution
        extra = [remaining // n] * n
    else:
        # Proportional to complexity
        extra = [int(remaining * (s / total_score)) for s in children_scores]

    return [b + e for b, e in zip(base, extra)]
```

## Architecture: The Algorithmic Spine

```
                        ┌─────────────┐
                        │  INITIALIZE  │  deterministic
                        └──────┬──────┘
                               │
                    ┌──────────▼──────────┐
                    │  COMPLEXITY SCORE   │  algorithm (feature scoring)
                    └──────────┬──────────┘
                               │
              ┌────────────────▼────────────────┐
              │  score < threshold?              │
              │  YES → auto-LEAF (skip AI)       │  decision tree
              │  NO  → AI DECOMPOSE              │  ← AI (semantic)
              └────────────────┬────────────────┘
                               │
                    ┌──────────▼──────────┐
                    │   DECOMPOSE GATE    │  algorithm (TF-IDF + toposort)
                    │   + CYCLE DETECT    │
                    │   + BUDGET ALLOC    │  algorithm (proportional/knapsack)
                    └──────────┬──────────┘
                               │
                    ┌──────────▼──────────┐
                    │  SOLVE SCHEDULING   │  algorithm (DAG critical path)
                    └──────────┬──────────┘
                               │
              wave 1: ┌────┬────┬────┐
                      │ AI │ AI │ AI │    ← AI (semantic — the real work)
              wave 2: └────┴──┬─┴────┘
                      ┌───┬───┘
                      │AI │ AI│           ← AI (only for dependent leaves)
                      └───┴───┘
                               │
                    ┌──────────▼──────────┐
                    │   GIT MERGE-TREE    │  algorithm (3-way merge)
                    │   + AST CONFLICT    │  algorithm (symbol table diff)
                    │   + SCOPE CHECK     │  algorithm (file path sets)
                    └──────────┬──────────┘
                               │
              ┌────────────────▼────────────────┐
              │  Conflicts remaining?            │
              │  Textual only → auto-resolve     │  algorithm (merge strategies)
              │  Semantic → AI RESOLVE           │  ← AI (only for ambiguous cases)
              └────────────────┬────────────────┘
                               │
                    ┌──────────▼──────────┐
                    │  REDUNDANCY PRUNE   │  algorithm (diff subset check)
                    └──────────┬──────────┘
                               │
                    ┌──────────▼──────────┐
                    │     FINALIZE        │  deterministic
                    └─────────────────────┘
```

## AI Call Reduction

| Scenario (20-node tree, 12 leaves) | Current AI Calls | Proposed AI Calls |
|-------------------------------------|-----------------|-------------------|
| Decompose (8 internal nodes) | 8 | 5-7 (auto-LEAF skips trivial) |
| Solve (12 leaves) | 12 | 12 (unchanged — this is the work) |
| Reunify (8 non-leaf nodes) | 8 | 0-2 (only semantic conflicts) |
| Reunify Rework | 0-8 | 0-2 (fewer conflicts reach AI) |
| Prune | 1 | 0 (algorithmic) |
| **Total** | **29-37** | **17-23** (~40% reduction) |

The savings compound: fewer AI calls → fewer retries → fewer rework loops.

## Implementation

All algorithms are implemented in Rust as `synodic fractal` CLI subcommands
(`cli/src/fractal/`), using the `rust-tfidf` crate for proper TF-IDF computation.

| Module | CLI Command | Algorithms |
|--------|-------------|------------|
| `fractal/decompose.rs` | `synodic fractal gate` | TF-IDF cosine (via `rust-tfidf`), Jaccard pre-filter, Kahn's toposort, complexity scoring, budget allocation |
| `fractal/schedule.rs` | `synodic fractal schedule` | BFS layer decomposition, critical path DP |
| `fractal/reunify.rs` | `synodic fractal reunify` | git merge-tree 3-way merge, set intersection conflict detection |
| `fractal/prune.rs` | `synodic fractal prune` | Greedy set cover, subset detection |
| `fractal/mod.rs` | (shared) | Term extraction, Jaccard similarity, type definitions |
| `cmd/fractal.rs` | (CLI dispatch) | stdin/file JSON input, pretty JSON output |

Tests: 28 fractal-specific tests covering orthogonality detection, cycle detection,
wave scheduling, diamond dependencies, conflict detection, set cover, and budget allocation.

## Algorithm Reference

| Algorithm | Used For | Complexity |
|-----------|----------|------------|
| TF-IDF + Cosine Similarity | Orthogonality check | O(n² · m) where n=children, m=terms |
| Kahn's Topological Sort | Cycle detection + solve ordering | O(V + E) |
| 3-Way Merge (git merge-tree) | Code reunification | O(n) per file |
| Jaccard Similarity | Quick pre-filter (kept) | O(m) |
| Proportional Allocation | Budget distribution | O(n) |
| Levenshtein / Diff | Redundancy detection | O(n · m) per pair |
| BFS Layer Decomposition | Parallel wave scheduling | O(V + E) |

All are polynomial, most are linear. The entire algorithmic spine adds negligible
overhead compared to a single AI subagent call.
