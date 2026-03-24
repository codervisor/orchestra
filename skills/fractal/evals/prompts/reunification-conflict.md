# Eval: Reunification Conflict Resolution

## Setup

Load the fractal SKILL.md into the agent's context.

## Prompt

```
You have the fractal decomposition skill loaded.

/fractal decompose "Design a data pipeline with:
(1) An ingestion layer that accepts CSV and JSON, normalizes to a common internal format,
(2) A transformation layer that applies validation rules and enrichment from external APIs,
(3) A storage layer that writes to both PostgreSQL (structured) and S3 (raw archives)"

After the SOLVE phase, the reunification step should identify that:
- The ingestion layer's "common internal format" must match what the transformation
  layer expects as input
- The transformation layer's output schema must match what the storage layer writes

If the solve subagents chose incompatible formats, the REUNIFY step must detect and
resolve the interface mismatch.

Follow the full orchestration protocol.
```

## Expected structure

1. **Three children** solving ingestion, transformation, storage independently
2. **Interface definitions** in each SOLVE REPORT (input format, output format)
3. **REUNIFY detects** at least one interface mismatch between adjacent stages
4. **REUNIFY REPORT** has CONFLICTS field listing the mismatch(es)
5. **RESOLUTION** field explains the chosen common format
6. **STATUS: MERGED** — conflicts are resolved, not left open
7. **Final design** has consistent data formats flowing through all three stages

## Anti-signal

- REUNIFY claims "no conflicts" when solve results used different data formats
- Conflicts listed but not resolved (STATUS: CONFLICT without resolution)
- Agent pre-coordinates formats during decompose phase (defeats the test)
- Interface mismatches silently ignored
