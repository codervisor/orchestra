# Eval: INSPECT has fresh context (no builder bias)

Verify that the INSPECT subagent is spawned independently of BUILD:

1. The INSPECT `Agent()` call does NOT use `resume` with the BUILD agent ID.
2. The INSPECT prompt contains only the spec content and a reference to the
   diff (branch name) — not the BUILD conversation or reasoning.
3. INSPECT does NOT use `isolation: worktree` (it's read-only).
4. The INSPECT subagent must read the diff itself via git commands.
