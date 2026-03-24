# Eval: Concurrent runs don't interfere

Launch two factory runs in separate sessions (or with a time gap ensuring
different timestamps). Verify:

1. Each run generates a unique `factory-{timestamp}` work ID.
2. Each run creates its own branch (`factory/factory-{ts1}` vs `factory/factory-{ts2}`).
3. Each run writes to its own manifest directory (`.factory/factory-{ts1}/` vs `.factory/factory-{ts2}/`).
4. Neither run's commits appear on the other's branch.
