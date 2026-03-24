---
status: complete
created: 2026-03-23
priority: high
tags:
- harness
- pipeline
- skills
depends_on:
- '061'
- '062'
parent: 058-code-harness-orchestration
created_at: 2026-03-23T00:53:28.641724329Z
updated_at: 2026-03-23T01:18:06.814385523Z
---

# Pipeline Definitions: Factory, Fractal, Swarm, and Adversarial YAMLs

## Overview

Four declarative pipeline YAML files that encode the factory, fractal, swarm, and adversarial skill topologies using the 4-type step system from spec 061. Includes prompt templates and output schemas referenced by each pipeline.

## Design

### Pipeline files

| Pipeline | File | Topology |
|----------|------|----------|
| Factory | `.harness/pipelines/factory.yml` | Linear: build → gate → inspect → branch → PR |
| Fractal | `.harness/pipelines/fractal.yml` | Recursive: complexity-check → decompose(fan/loop) → schedule → solve(fan/parallel) → reunify(fan/loop) → PR |
| Swarm | `.harness/pipelines/swarm.yml` | Divergent: strategize → explore(fan/loop with checkpoint+prune) → merge → PR |
| Adversarial | `.harness/pipelines/adversarial.yml` | Escalating: generate → gate → attack → termination(fan/loop) → PR |

### Prompt templates

Each pipeline references prompt templates at `skills/{name}/prompts/*.md`. Templates receive context via the runtime's variable interpolation (`${spec.*}`, `${manifest.*}`, `${steps.{name}.*}`).

### Output schemas

Each agent step declares an `output_schema` pointing to `schemas/*.json`. These are JSON Schema files that constrain agent structured output.

### Context passing (no session continuity)

Per spec 061's design, agent steps that need prior context (e.g., factory's `ci-fix` needing build context) receive it via `context` maps — not session IDs. The runtime injects relevant prior step outputs into the prompt.

```yaml
- name: ci-fix
  type: agent
  prompt: skills/factory/prompts/ci-fix.md
  context:
    build_diff: ${steps.build.diff}
    error_output: ${steps.gates.failures}
    manifest: ${manifest.summary}
```

### Adversarial escalation ladder

The adversarial pipeline's critic mode escalation is data in the pipeline config, not a variable filter:

```yaml
config:
  critic_modes: [syntax-and-types, edge-cases, concurrency-safety, adversarial-inputs, semantic-analysis]
steps:
  - name: adversarial-loop
    type: fan
    mode: loop
    context:
      critic_mode_index: loop.iteration    # runtime resolves to config.critic_modes[N]
```

## Plan

- [ ] Write `factory.yml` pipeline using 4-type step system
- [ ] Write `fractal.yml` pipeline using 4-type step system
- [ ] Write `swarm.yml` pipeline using 4-type step system
- [ ] Write `adversarial.yml` pipeline using 4-type step system
- [ ] Create prompt templates for each pipeline's agent steps
- [ ] Create JSON Schema files for each agent step's output
- [ ] Validate all pipelines against spec 061's YAML schema
- [ ] Document context passing patterns for each pipeline

## Test

- [ ] All 4 pipelines pass `synodic harness validate`
- [ ] Variable references in pipelines resolve against known scopes
- [ ] Output schemas are valid JSON Schema
- [ ] Prompt templates render with sample context data
- [ ] Each pipeline encodes the same semantics as its SKILL.md predecessor
