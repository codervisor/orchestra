# Eval: End-to-end Design Decomposition

## Setup

Load the fractal SKILL.md into the agent's context before giving this prompt.

## Prompt

```
You have the fractal decomposition skill loaded. Use it to decompose the
following system design task:

/fractal decompose "Design a real-time collaborative document editor with
the following requirements: (1) multi-user concurrent editing with conflict
resolution, (2) rich text formatting with extensible block types, (3) offline
support with sync-on-reconnect, (4) access control with sharing permissions,
(5) version history with diff viewing"

Follow the full orchestration protocol from the SKILL.md.
```

## Expected structure

1. **Decomposition tree** with at least 3 top-level children (e.g., CRDT/OT engine, rich text model, sync/offline)
2. **Depth >= 2** — at least one child further decomposed (e.g., CRDT splits into conflict resolution + operation log)
3. **All leaf nodes solved** with result.md files
4. **Reunification** producing a coherent design that integrates all sub-solutions
5. **output.md** with the final unified design document
6. **manifest.json** tracking the full tree and metrics

## Anti-signal

- Agent writes one monolithic design document (no decomposition)
- Sub-specs have overlapping scopes (e.g., two children both handle "sync")
- No reunification — sub-solutions just concatenated
- Leaf solutions reference or depend on implementation details of siblings
