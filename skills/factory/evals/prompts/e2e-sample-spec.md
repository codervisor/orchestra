# Eval: End-to-end on sample spec

Run the factory skill on the sample fixture spec:

```
/factory run skills/factory/fixtures/sample-spec/README.md
```

Verify:
1. A `factory/{work-id}` branch is created with implementation commits.
2. `greet.sh` is created and works (`bash greet.sh Alice` → `Hello, Alice!`).
3. `.factory/{work-id}/manifest.json` exists and has valid structure.
4. INSPECT returned a verdict.
5. If approved, a PR was created.
