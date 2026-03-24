# Eval: Leaf Detection — Simple Tasks Not Split

## Setup

Load the fractal SKILL.md into the agent's context before giving this prompt.

## Prompt

```
You have the fractal decomposition skill loaded. Use it to decompose:

/fractal decompose "Add a --verbose flag to the CLI that enables debug logging to stderr"

Follow the full orchestration protocol from the SKILL.md.
```

## Expected structure

1. **DECOMPOSE VERDICT: LEAF** — this task is too simple to split
2. **Direct solve** — task handled in a single SOLVE pass
3. **No child directories** created under tree/
4. **manifest.json** showing tree_depth: 1, leaf_nodes: 1, total_nodes: 1

## Anti-signal

- Agent splits a trivial task into unnecessary sub-problems (over-decomposition)
- Agent creates child specs for "parse flag" and "add logging" separately
- No LEAF verdict — everything gets split regardless of complexity
