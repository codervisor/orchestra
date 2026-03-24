---
status: planned
created: 2026-03-11
priority: critical
tags:
- vision
- strategy
- factory
- assembly-line
- core-objective
parent: null
depends_on: []
created_at: 2026-03-11T00:00:00Z
updated_at: 2026-03-11T00:00:00Z
---

# The Coding Factory — Ford Assembly Line for the AI Era

## Core Objective (System Prompt)

> **Synodic builds the software production line for the AI era.**
>
> Just as Ford's assembly line transformed automobiles from artisanal one-offs
> into predictable, high-throughput, continuously-improving industrial output,
> Synodic transforms software from craft-mode (one developer, one task, one tool)
> into **factory-mode**: heterogeneous AI agents organized into assembly-line
> stations that transform requirements into shipped, tested, production-ready
> code at industrial scale and predictable quality.
>
> The measure of success is not how smart any single agent is — it's
> **throughput, cycle time, defect rate, and cost per unit of shipped software.**

---

## The Ford Parallel

Ford didn't invent the automobile. He invented the **production system** that
made automobiles accessible at scale. The key innovations weren't in the
engine — they were in the factory:

| Ford's Innovation | Synodic's Equivalent |
|---|---|
| **Division of labor** — complex car-building decomposed into simple, repeatable steps | **Agent specialization** — each station staffed by the best-fit agent (explorer, implementer, reviewer, tester, deployer) |
| **Moving assembly line** — work flows forward continuously; workers stay at their stations | **Task conveyor** — work items flow through pipeline stages automatically; agents don't context-switch |
| **Interchangeable parts** — any Model T part fits any Model T | **Tool-agnostic stations** — any conformant AI (Claude, Codex, Gemini, Copilot) can staff any role |
| **Standardized stations** — clear inputs, tooling, and outputs at each position | **Pipeline stages** — defined input/output contracts, quality gates, and SLAs per stage |
| **Quality inspection** — dedicated inspectors at key points; defective parts rejected before assembly | **Adversarial review gates** — generator/critic loops, automated validation, and human-in-the-loop at defined checkpoints |
| **Time studies** — measure every operation; eliminate waste | **Production metrics** — throughput (PRs/day), cycle time (spec→ship), defect rate (rework %), cost per unit ($/PR) |
| **Continuous improvement (Kaizen)** — every worker can stop the line; feedback loops tighten process | **Self-improving factory** — production data feeds back to optimize coordination patterns, agent routing, and cost |
| **Vertical integration** — Ford owned the steel mills, glass plants, rubber plantations | **End-to-end ownership** — from requirement intake through deployment, monitoring, and maintenance |
| **Economies of scale** — unit cost drops as volume increases | **Nemosis distillation** — frontier models teach cheaper models; cost per unit decreases with fleet experience |

---

## The Production Line — Seven Stations

```
 ╔═══════════╗    ╔═══════════╗    ╔═══════════╗    ╔═══════════╗
 ║  STATION 1║    ║  STATION 2║    ║  STATION 3║    ║  STATION 4║
 ║   INTAKE  ║───→║  DESIGN   ║───→║   BUILD   ║───→║  INSPECT  ║
 ║           ║    ║           ║    ║           ║    ║           ║
 ║ Requirement║   ║ Spec →    ║    ║ Code →    ║    ║ Adversarial║
 ║ → Spec    ║    ║ Blueprint ║    ║ Implement ║    ║ Review +  ║
 ║           ║    ║           ║    ║           ║    ║ Auto-test ║
 ╚═══════════╝    ╚═══════════╝    ╚═══════════╝    ╚═══════════╝
                                                          │
 ╔═══════════╗    ╔═══════════╗    ╔═══════════╗          │
 ║  STATION 7║    ║  STATION 6║    ║  STATION 5║          │
 ║  MAINTAIN ║←───║  DEPLOY   ║←───║  HARDEN   ║←─────────╘
 ║           ║    ║           ║    ║           ║
 ║ Stigmergic║    ║ Ship →    ║    ║ Speculative║
 ║ Monitor + ║    ║ Release + ║    ║ Swarm →   ║
 ║ Evolve    ║    ║ Rollback  ║    ║ Fuzz/Perf ║
 ╚═══════════╝    ╚═══════════╝    ╚═══════════╝
```

### Station 1 — INTAKE (Requirement → Spec)
- **Input:** Issue, feature request, bug report, user story, or verbal description
- **Process:** Decompose into atomic units; classify complexity; estimate cost
- **Coordination:** Fractal Decomposition (spec 027) — recursively split until each unit fits one agent
- **Output:** LeanSpec-formatted spec with acceptance criteria, tagged with cost estimate
- **Quality gate:** Spec review — is it atomic? testable? unambiguous?

### Station 2 — DESIGN (Spec → Blueprint)
- **Input:** Atomic spec
- **Process:** Explore approaches; identify affected files; design API/interface changes
- **Coordination:** Speculative Swarm (spec 025) — fork 2–4 agents to explore different approaches; converge on best
- **Output:** Implementation blueprint (files to change, approach, edge cases, test plan)
- **Quality gate:** Blueprint review — does it address all acceptance criteria? Are edge cases covered?

### Station 3 — BUILD (Blueprint → Code)
- **Input:** Approved blueprint
- **Process:** Implement the change, write tests, create PR
- **Coordination:** Hierarchical (spec 030) — lead implementer delegates to specialists (frontend, backend, infra)
- **Output:** PR with passing CI, tests, and documentation
- **Quality gate:** CI green, coverage threshold met

### Station 4 — INSPECT (Code → Reviewed Code)
- **Input:** PR
- **Process:** Multi-dimensional review — correctness, security, performance, style, maintainability
- **Coordination:** Generative-Adversarial (spec 028) — reviewer agents find flaws, author agent fixes; iterate until convergence
- **Output:** Approved PR or specific rework items sent back to Station 3
- **Quality gate:** All review dimensions pass; no open threads

### Station 5 — HARDEN (Reviewed Code → Battle-Tested Code)
- **Input:** Approved PR
- **Process:** Stress testing, fuzzing, performance benchmarking, integration testing
- **Coordination:** Speculative Swarm — multiple agents attack from different angles (security, load, edge cases)
- **Output:** Hardening report; performance baselines; security attestation
- **Quality gate:** No P0/P1 findings; perf within SLA

### Station 6 — DEPLOY (Tested Code → Production)
- **Input:** Hardened PR
- **Process:** Merge, release, canary deploy, monitor
- **Coordination:** Pipeline (spec 030) — sequential deployment stages with rollback triggers
- **Output:** Live in production with monitoring
- **Quality gate:** Error rate within threshold; no rollback triggered

### Station 7 — MAINTAIN (Production → Continuous Health)
- **Input:** Deployed system
- **Process:** Monitor health, detect drift, proactive maintenance, dependency updates
- **Coordination:** Stigmergic (spec 029) — agents observe production artifacts (logs, metrics, alerts) and self-organize maintenance tasks
- **Output:** Maintenance tasks fed back to Station 1 (loop)
- **Quality gate:** SLA compliance; tech debt budget

---

## Factory Metrics — The Dashboard

A factory is measured, not felt. The core metrics:

| Metric | Definition | Target |
|---|---|---|
| **Throughput** | Shipped units (PRs merged) per day | Continuous improvement |
| **Cycle time** | Wall-clock time from intake (Station 1) to production (Station 6) | < 4 hours for atomic units |
| **Lead time** | Time from requirement creation to production | < 24 hours for standard work |
| **Defect rate** | % of shipped units requiring rework post-deploy | < 5% |
| **First-pass yield** | % of units passing all stations without rework loops | > 80% |
| **Cost per unit** | Total agent compute cost per shipped PR | Decreasing quarter over quarter |
| **Agent utilization** | % of time agents are productive vs idle/waiting | > 70% |
| **WIP count** | Work items currently in flight across all stations | Bounded by Kanban limits |
| **Station dwell time** | Time spent at each station | Balanced (no station > 2x average) |

---

## The One-Line Test

If we can't explain it in one line to a factory worker, the abstraction is wrong:

> **"Requirements go in one end, shipped code comes out the other, and every station in between is staffed by AI agents that never sleep, never forget, and get cheaper every month."**

---

## Relationship to Existing Vision

The current vision — "Docker Compose for AI agents" — describes the **execution substrate** (Layer 1). It's necessary but not sufficient.

The factory vision subsumes and extends it:

```
Docker Compose for AI agents    →  The factory floor
+ Coordination primitives       →  The machines
+ Pipeline/Station abstraction  →  The assembly line
+ Quality gates                 →  The inspection system
+ Production metrics            →  The time studies
+ Self-improvement              →  The process engineering dept
+ Cost optimization             →  The economies of scale
────────────────────────────────────────────────────
= The Coding Factory
```

**"Docker Compose for AI agents" is how we build the floor. "The Coding Factory" is what we build on it.**

---

## Implementation

The MVP (spec 044) proves the core two-station pipeline (BUILD → INSPECT) as a
Claude Code skill. See [044-factory-skill-mvp](../044-factory-skill-mvp/README.md)
for the implementation spec.
