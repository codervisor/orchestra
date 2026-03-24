# Eval: Parallel Solve

## Setup

Load the fractal SKILL.md into the agent's context.

## Prompt

```
You have the fractal decomposition skill loaded. Config: solve_mode=parallel.

/fractal decompose "Design a REST API with three independent resource endpoints:
(1) Users — CRUD with email validation and password hashing,
(2) Products — CRUD with image upload and category tagging,
(3) Orders — CRUD with status transitions and inventory checks"

Follow the full orchestration protocol. These three endpoints are orthogonal
and should be solved in parallel.
```

## Expected structure

1. **Three children** identified (users, products, orders)
2. **All three SOLVE subagents spawned in a single message** (parallel Agent calls)
3. **Each SOLVE prompt** includes the other two specs as read-only sibling context
4. **Each result** is self-contained within its declared scope
5. **Reunification** merges the three endpoint designs into a coherent API

## Anti-signal

- SOLVE subagents spawned one at a time (sequential when parallel was requested)
- Sibling context not provided — solutions can't align interfaces
- One solve agent implements pieces of another's scope
