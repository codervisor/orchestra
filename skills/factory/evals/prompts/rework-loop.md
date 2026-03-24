# Eval: Rework loop fires on deliberate bug

Run the factory skill on a spec. After the first BUILD, manually verify that
INSPECT identifies at least one issue and returns `VERDICT: REWORK`.

Then verify that the factory re-invokes BUILD with the rework items and that
the second attempt addresses the feedback.

Check the manifest for:
- `attempts[0].inspect.verdict` === `"rework"`
- `attempts[0].inspect.items` is non-empty
- `attempts[1]` exists and addresses the feedback
