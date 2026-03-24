# Eval: Budget Enforcement

## Setup

Load the fractal SKILL.md into the agent's context. Override config with a tight budget:

```yaml
fractal:
  max_total_nodes: 6
  max_depth: 3
  max_children: 5
```

## Prompt

```
You have the fractal decomposition skill loaded. Config override:
max_total_nodes=6, max_depth=3.

/fractal decompose "Build a complete e-commerce platform with: product catalog,
shopping cart, checkout with payment processing, user accounts, order tracking,
inventory management, admin dashboard, email notifications, search with filters,
and analytics reporting"

Follow the full orchestration protocol. The task is intentionally too large for
the budget — the skill must enforce the node cap.
```

## Expected structure

1. **First level**: creates some children (e.g., 4-5)
2. **Budget hit**: remaining children forced to LEAF even if complex
3. **manifest.json** records budget enforcement event
4. **Total nodes <= 6** in the final tree
5. **All nodes still solved** (forced leaves get solved directly)

## Anti-signal

- Tree exceeds max_total_nodes
- Budget ignored — all nodes split freely
- Forced leaves are abandoned (not solved)
- No indication in manifest that budget was enforced
