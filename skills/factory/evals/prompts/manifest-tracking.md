# Eval: Manifest records full attempt history

After a factory run completes, read `.factory/{work-id}/manifest.json` and
validate against `references/manifest.schema.json`:

1. Top-level fields: `id`, `spec`, `status`, `branch`, `attempts`, `metrics`.
2. Each attempt entry has `build` (with `files_changed`, `tests`, `commit`) and `inspect` (with `verdict`, `summary`).
3. `metrics.cycle_time_seconds` is a positive number.
4. `metrics.total_attempts` matches the length of `attempts`.
5. `metrics.first_pass_yield` is `true` only if `attempts[0].inspect.verdict === "approve"`.
