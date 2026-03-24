# Eval: Rework limit enforced at 3 attempts

Simulate or trigger a scenario where INSPECT returns `VERDICT: REWORK` on
all 3 attempts. Verify that:

1. The factory does NOT attempt a 4th build.
2. The manifest `status` is `"escalated"`.
3. No PR is created.
4. The user receives escalation output with the last rework items.
