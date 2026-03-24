# Eval: Code Task Decomposition

## Setup

Load the fractal SKILL.md into the agent's context. Set output_mode=code.

## Prompt

```
You have the fractal decomposition skill loaded. Config: output_mode=code.

/fractal decompose "Add a configuration file parser to the Rust CLI that:
(1) Supports TOML, JSON, and YAML formats (detect by file extension),
(2) Implements environment variable interpolation (${VAR} syntax),
(3) Validates parsed config against a JSON Schema,
(4) Provides helpful error messages with line numbers on parse failure"

Follow the full orchestration protocol. Each leaf should produce working
Rust code committed to an isolated worktree.
```

## Expected structure

1. **Decomposition** into at least 3 sub-problems (format parsing, env interpolation, schema validation)
2. **SOLVE subagents use isolation: worktree** for code isolation
3. **Each leaf commits** with `fractal({slug}):` prefix
4. **REUNIFY integrates** the code — resolves import paths, shared types, public API
5. **Final code compiles** — `cargo build` succeeds after reunification
6. **output.md** lists all files changed with summary

## Anti-signal

- SOLVE subagents don't use worktree isolation (code conflicts)
- Leaves produce code that only works when copy-pasted together (no interfaces)
- Reunification is just file concatenation without integration
- No commit prefix convention followed
