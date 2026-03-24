---
status: complete
created: 2026-03-23
priority: medium
tags:
- harness
- migration
- skills
depends_on:
- '061'
- '062'
- '063'
- '064'
parent: 058-code-harness-orchestration
created_at: 2026-03-23T00:54:01.356579130Z
updated_at: 2026-03-23T01:18:06.830542654Z
---

# Skill Migration: SKILL.md to Pipeline YAML Shim Layer

## Overview

Migration path from current SKILL.md-based orchestration to pipeline YAML execution. After migration, SKILL.md files become thin shims that invoke `synodic harness run --pipeline <name>`.

## Design

### Migration mapping

| Current | After migration |
|---------|----------------|
| `skills/factory/SKILL.md` (~500 lines of orchestration) | `synodic harness run --pipeline factory --spec <path>` |
| `skills/fractal/SKILL.md` (~600 lines) | `synodic harness run --pipeline fractal --spec <path>` |
| `skills/swarm/SKILL.md` (~500 lines) | `synodic harness run --pipeline swarm --spec <path>` |
| `skills/adversarial/SKILL.md` (~500 lines) | `synodic harness run --pipeline adversarial --spec <path>` |

### What moves where

| Content | From | To |
|---------|------|----|
| Orchestration logic | SKILL.md prose | Pipeline YAML steps |
| Prompt templates | Inline in SKILL.md | `skills/{name}/prompts/*.md` |
| Output schemas | Implicit in SKILL.md | `schemas/*.json` |
| Algorithmic commands | Described in SKILL.md | `synodic fractal/swarm` CLI |
| Governance logging | SKILL.md instructions | Automatic via runtime middleware |

### Shim format

Post-migration SKILL.md files contain only:
1. Skill description (for `/help` and discovery)
2. Usage examples
3. A single execution line: `synodic harness run --pipeline <name> --spec ${spec.path}`

### Migration strategy

Incremental, one skill at a time. Factory first (simplest linear pipeline), then adversarial, then fractal, then swarm (most complex).

### Rollback

Keep original SKILL.md content in git history. Shim can be reverted to full SKILL.md if pipeline engine has issues.

## Plan

- [ ] Migrate factory SKILL.md → shim + factory.yml (simplest pipeline)
- [ ] Migrate adversarial SKILL.md → shim + adversarial.yml
- [ ] Migrate fractal SKILL.md → shim + fractal.yml
- [ ] Migrate swarm SKILL.md → shim + swarm.yml
- [ ] Extract prompt templates from each SKILL.md into `skills/{name}/prompts/`
- [ ] Verify each migrated skill produces equivalent governance log output
- [ ] Update CLAUDE.md skill table to reference pipeline execution

## Test

- [ ] Each shim invokes `synodic harness run` correctly
- [ ] Prompt templates render identically to SKILL.md inline versions
- [ ] Governance JSONL output format unchanged after migration
- [ ] `/factory run`, `/fractal decompose`, `/swarm run`, `/adversarial run` still work
- [ ] Rollback: reverting shim to full SKILL.md restores original behavior
