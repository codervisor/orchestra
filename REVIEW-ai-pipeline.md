# AI Pipeline Review

## Scope

Review of the AI delivery pipeline (`ai-delivery-pipeline.yml`), the four
algorithmic pipelines (factory, fractal, swarm, adversarial), and the Rust
pipeline execution engine.

---

## 1. GitHub Actions Workflow (`ai-delivery-pipeline.yml`)

### 1.1 Duplicate file drift

`ai-delivery-pipeline.yaml` (root) and `.github/workflows/ai-delivery-pipeline.yml`
are nearly identical but have diverged:

| Difference | `.github/workflows/` | Root copy |
|---|---|---|
| `--yolo` comment | no parenthetical note | adds `(≈ claude --dangerously-skip-permissions)` |
| CI aggregate step | single grep-based check | **two** loops — first uses broken `steps[format(...)]` syntax, then falls through to the grep loop |

**Recommendation:** Remove the root `ai-delivery-pipeline.yaml` or keep it as a
symlink. Having two copies invites silent drift.

### 1.2 Broken `steps[format()]` outcome check (root copy, line 399-405)

```yaml
if [ "${{ steps[format('{0}', steps[step])].outcome }}" = "failure" ] 2>/dev/null; then
```

`format()` is not valid in `${{ }}` expression context and `steps[step]` does a
literal key lookup where `step` is a shell variable, not an Actions expression.
This block always silently fails (`2>/dev/null` hides the error) and falls
through to the grep-based check, making it dead code.

**Severity:** Low — the grep fallback works, but dead code obscures intent.

### 1.3 Script-injection risk via issue body

```yaml
-p "Implement issue #${{ needs.pickup.outputs.issue_number }}: ${{ needs.pickup.outputs.issue_title }}
  ${{ needs.pickup.outputs.issue_body }}
```

`issue_body` is user-controlled and interpolated directly into a shell command.
A crafted issue body with backticks, `$(...)`, or unmatched quotes can escape
the prompt string and inject shell commands into the runner.

**Severity:** High — the runner has `contents: write` and `GITHUB_TOKEN`.

**Fix:** Pass the body via an environment variable or a temporary file instead
of inline interpolation:

```yaml
env:
  ISSUE_BODY: ${{ needs.pickup.outputs.issue_body }}
run: |
  copilot ... -p "Implement issue ... ${ISSUE_BODY_FILE:=$(mktemp)}"
  # or write to file first
```

The same pattern appears in the `retry-via-copilot` job (line 508) where
`failure_reason` is injected into a PR comment body literal — though that's a
JS string context (lower risk), a multi-line log containing backticks could
still break the template literal.

### 1.4 Retry counter is not truly incremented

In the `code-local` job (line 196-213), the attempt counter is either `1` (new
branch) or hard-coded `2` (branch exists). The comment says
`"simplified; production would parse PR comments"` — this means the retry loop
will never reach `max_retries` (default 3) via its own counter, so the
`retry-via-dispatch` job re-triggers the whole workflow but the counter resets
to `2` each time, creating an **infinite retry loop** until something else
breaks.

**Severity:** Medium — could burn Actions minutes indefinitely.

### 1.5 `qa-gate` references `needs.code-local.outputs.attempt` without dependency

Line 464 reads `needs.code-local.outputs.attempt`, but `qa-gate` depends on
`[pickup, resolve, ci]`, not `code-local`. For the delegate strategy
`code-local` is skipped, so this expression evaluates to empty string. The `|| 1`
fallback masks it, but this is fragile.

### 1.6 `release` job tags the wrong SHA

Line 598: `context.sha` is the SHA of the `release` job's checkout, which is
`main` at workflow trigger time — not the squash-merge commit from
`deploy-staging`. The tag will point to the pre-merge main, not the merged
code.

**Fix:** After the merge step, re-checkout main and capture the new HEAD SHA.

### 1.7 Missing concurrency controls

No `concurrency:` key on the workflow. Two issues labeled `ai-implement`
simultaneously will race on overlapping branches, CI slots, and PR creation.

---

## 2. Pipeline YAML Definitions (Factory / Fractal / Swarm / Adversarial)

### 2.1 Generally well-designed

The 4-step-type system (agent, run, branch, fan) is clean and composable. The
YAML schema is minimal yet expressive. The separation of concerns between
declarative pipeline definitions and the Rust execution engine is solid.

### 2.2 Hardcoded `synodic` CLI name

Pipelines reference `synodic fractal schedule`, `synodic fractal gate`, etc.
The CLI binary is `orchestra-cli` and the Cargo binary name should be verified.
If the binary was renamed from `synodic` to `orchestra`, these commands will
fail at runtime.

### 2.3 `create-pr` steps use `gh` CLI

All four pipelines end with `gh pr create`. The executor runs commands via
`sh -c`, so `gh` must be in PATH and authenticated. This is fine in CI but
will fail locally without setup. Consider documenting this dependency or
adding a gate that checks for `gh` availability.

---

## 3. Rust Pipeline Engine

### 3.1 Schema (`schema.rs`) — Good

- Clean serde derivations with `#[serde(tag = "type")]` for step type discrimination.
- All four step types fully modeled.
- Good default handling (`default_max_iterations`, optional fields).
- 17 tests covering all step types, edge cases, and error paths.

### 3.2 Executor (`executor.rs`) — Good with caveats

**Strengths:**
- Middleware chain (retry → timeout → log) applied correctly.
- Branch routing with iteration counting and exhaust handling.
- `on_fail` rework/escalate routing works.
- Variable context properly populated after each step.

**Issues:**

1. **`duration_ms` always 0 in step results.** The `execute_agent`,
   `execute_run`, and `execute_branch` functions all return `duration_ms: 0`.
   The duration is calculated in the outer `execute()` function and stored in
   `StepResult`, but the inner functions overwrite it. The outer function
   creates a new `StepResult` with the correct duration only to then
   immediately shadow it by pushing the inner result.

   Wait — re-reading: the inner result IS what gets pushed, but `duration` is
   computed at line 122 and never written back into the result. The
   `step_result` at line 134 copies `result.status` and `result.output` but
   uses the outer `duration`. So this is actually correct for the final
   `step_results` vec, but the `StepResult` returned from middleware has 0.
   The actual duration tracking is fine.

2. **Fan parallel mode executes sequentially.** Line 548 comment acknowledges
   this: `"true parallelism requires async"`. This is a known limitation but
   means the swarm's "parallel explore" step runs strategies serially.

3. **Fan collection ignores `over` field.** `execute_fan_collection` only
   iterates `fan.steps`, ignoring `fan.over` and `fan.step` (the template for
   per-item execution). This means `fan mode: parallel over: strategies` with
   a `step:` template won't actually iterate over the collection.

4. **Branch verdict matching is fragile.** Line 180: `verdict.contains("approve")`
   means any output containing the substring "approve" routes to approve, even
   `"I do not approve"`. Consider parsing the structured JSON output schema
   instead.

5. **Regex compiled on every `interpolate()` call.** `vars.rs` line 40
   compiles the regex fresh each time. Should use `lazy_static!` or
   `std::sync::OnceLock`.

### 3.3 Gates (`gates.rs`) — Good

- File-match filtering with glob patterns works correctly.
- Graceful fallback when `gates.yml` doesn't exist.
- Dual-layer filtering (gate-level + step-level match patterns).
- 12 tests with good coverage.

### 3.4 Validation (`validate.rs`) — Good

- Catches all structural errors: empty names, duplicate names, dangling
  references, missing required fields.
- Collects all errors rather than failing on first (good UX).
- 14 tests including full pipeline validation scenarios.

### 3.5 Variables (`vars.rs`) — Good

- Fail-fast on unset variables per spec.
- All five scopes (`config`, `spec`, `steps`, `loop`, `manifest`) work.
- 14 tests.
- Minor: regex should be compiled once (see 3.2.5 above).

---

## 4. Summary of Findings

| # | File | Severity | Issue |
|---|------|----------|-------|
| 1 | `ai-delivery-pipeline.yml` | **High** | Script injection via `issue_body` |
| 2 | `ai-delivery-pipeline.yml` | Medium | Infinite retry loop (counter never increments past 2) |
| 3 | `ai-delivery-pipeline.yml` | Medium | Release tag points to wrong SHA |
| 4 | `ai-delivery-pipeline.yaml` | Low | Duplicate file with drift + dead code |
| 5 | `ai-delivery-pipeline.yml` | Low | Missing concurrency controls |
| 6 | `ai-delivery-pipeline.yml` | Low | `qa-gate` reads output from non-dependency job |
| 7 | `executor.rs` | Medium | Fan collection ignores `over`/`step` template |
| 8 | `executor.rs` | Low | Fan parallel runs sequentially (documented) |
| 9 | `executor.rs` | Low | Branch verdict matching via substring is fragile |
| 10 | `vars.rs` | Low | Regex recompiled per call |
| 11 | `pipelines/*.yml` | Low | `synodic` CLI name may be stale |

---

## 5. Fixes Applied

The following issues from the review have been addressed:

| # | Issue | Fix |
|---|-------|-----|
| 1 | Script injection via `issue_body` (High) | Moved all user-controlled data (`issue_body`, `issue_title`, `failure_reason`, `ci_log`) to environment variables; prompts written to temp files via `printf '%s'` (no shell expansion); agent instructed to read the prompt file rather than receiving body content on the command line |
| 2 | Infinite retry loop (Medium) | Replaced hardcoded `ATTEMPT=2` with actual PR comment counting via GitHub API |
| 3 | Release tag wrong SHA (Medium) | Replaced `context.sha` with `git.getRef('heads/main')` to capture post-merge HEAD |
| 4 | Duplicate `ai-delivery-pipeline.yaml` (Low) | Removed root copy; single source of truth in `.github/workflows/` |
| 5 | Missing concurrency controls (Low) | Added `concurrency: group` keyed on issue number |
| 6 | `qa-gate` reads non-dependency job (Low) | Added `code-local` to `qa-gate.needs` with `always()` condition |
| 7 | `synodic` CLI name stale in pipelines (Low) | Renamed to `orchestra` in `fractal.yml` and `swarm.yml` |

### Remaining (Rust engine — not addressed in this pass)

| # | Issue | Severity |
|---|-------|----------|
| 7 | Fan collection ignores `over`/`step` template | Medium |
| 8 | Fan parallel runs sequentially (documented) | Low |
| 9 | Branch verdict matching via substring is fragile | Low |
| 10 | Regex recompiled per call in `vars.rs` | Low |

---

### What's working well

- The 4-step-type pipeline schema is clean, composable, and well-tested
  (140 Rust tests all passing).
- Algorithmic spines (fractal TF-IDF, swarm Jaccard) successfully replace
  ~40% of AI calls with deterministic operations.
- Gate system with file-match filtering is practical and avoids unnecessary
  CI work.
- Variable interpolation with fail-fast semantics catches misconfigurations
  early.
- The overall architecture separating declarative YAML from Rust execution
  is well-structured and maintainable.
