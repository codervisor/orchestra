---
status: planned
created: '2026-03-12'
tags:
  - testing
  - factory
  - validation
priority: high
created_at: '2026-03-12T07:04:01.332164054+00:00'
---

# Factory Test Harness — End-to-End Pipeline Validation

> **Status**: planned · **Priority**: high · **Created**: 2026-03-12

## Overview

End-to-end validation harness for the Synodic factory pipeline (BUILD → INSPECT → rework loop). Runs real specs through real Claude calls against example code projects, with file-based observability for human verification.

### Goals

- Validate the factory pipeline works on small-to-medium use cases with real Claude runs
- Provide simple observability so a verifier can see what happened
- Graduate from tiny deterministic fixtures to realistic small features

### Non-Goals

- Unit testing internal engine logic (separate concern)
- Automated CI integration (future work)
- Dashboards or databases for observability

## Design

### Structure

```
tests/
  fixtures/
    01-add-fibonacci/
      repo/                 ← starting codebase (Rust lib, will be git-initialized by runner)
      spec/README.md        ← spec the factory will execute (copied into temp repo)
      verify.sh             ← post-run correctness checks
    02-add-cli-flag/
      repo/                 ← starting codebase (clap-based CLI tool)
      spec/README.md
      verify.sh
    03-fix-bug/
      repo/                 ← starting codebase (string utility with deliberate bug)
      spec/README.md
      verify.sh
  run-factory-tests.sh      ← orchestrator script
```

### Test Fixtures

Each fixture contains three things:
- `repo/` — the starting source code (not a git repo; the runner initializes git)
- `spec/README.md` — a LeanSpec-format spec that the factory will execute
- `verify.sh` — a script that checks the result

#### Fixture 01 — `add-fibonacci` (tiny, deterministic)

**Starting repo:** A bare Rust lib crate with `lib.rs` containing a single `pub fn add(a: i32, b: i32) -> i32`.

**Spec:** "Add a `fibonacci(n: u32) -> u64` function and unit tests for it."

**Verification:**
- `lib.rs` contains `fn fibonacci`
- `cargo test` passes in the result
- At least 3 test cases exist

#### Fixture 02 — `add-cli-flag` (small, realistic)

**Starting repo:** A tiny CLI tool (clap-based) that reads a file and prints its line count.

**Spec:** "Add a `--words` flag that prints word count instead of line count."

**Verification:**
- `--words` flag exists in help output
- Running with `--words` on a known file produces correct count
- Existing line-count behavior unchanged

#### Fixture 03 — `fix-bug` (small, diagnostic)

**Starting repo:** A small string utility crate with a `slug()` function that has a deliberate bug (doesn't handle consecutive hyphens).

**Spec:** "Fix the bug where `slug("hello---world")` returns `"hello---world"` instead of `"hello-world"`. Add a regression test."

**Verification:**
- `cargo test` passes
- Regression test exists (grep for the test function)
- `slug("hello---world")` returns `"hello-world"` (verified via `cargo test`)

### Test Runner

`tests/run-factory-tests.sh` — a shell script that orchestrates fixture runs.

#### Prerequisites

The runner builds the `synodic` binary before running fixtures:

```bash
cargo build -p syn-cli
```

Debug build is fine — these are validation runs, not benchmarks. It uses `./target/debug/synodic` for all runs — no PATH dependency.

#### Behavior

For each fixture:

1. **Prepare temp repo:**
   - Create `<output-dir>/<fixture-name>/repo/`
   - Copy fixture's `repo/` contents into it
   - Copy fixture's `spec/` directory into `<temp-repo>/spec/`
   - `git init`, `git checkout -b main`, `git add .`, `git commit -m "initial"`
   - This ensures the temp repo has a `main` branch (required by `station.rs` which diffs against `main`)

2. **Run the factory:**
   - Execute `synodic run spec/` from within the temp repo directory
   - Tee stderr to `<output-dir>/<fixture-name>.log`
   - Record wall-clock time via `time` or `$SECONDS`

3. **Handle factory outcome:**
   - If `synodic run` exits 0: proceed to branch checkout and verification
   - If `synodic run` exits non-zero (escalation, agent crash): mark fixture as FAIL with reason "factory-error", record the error, skip `verify.sh`, continue to next fixture
   - Note: the CLI always exits 1 for any failure — escalation vs. crash are distinguished by the error message in the log, not the exit code

4. **Checkout factory branch before verification:**
   - The CLI switches back to `main` after a successful run (main.rs line 102), so the temp repo will be on `main`, not the factory branch
   - The factory branch name is non-deterministic (`factory/work-<uuid>`)
   - The runner discovers it by globbing: `<temp-repo>/.factory/*/manifest.json`, then reading the `branch` field with `jq -r .branch`
   - Run `git checkout <branch>` in the temp repo so that `verify.sh` tests the factory's output, not the original code

5. **Run verification:**
   - Execute `verify.sh <path-to-temp-repo>` (the temp repo path is passed as `$1`)
   - Exit code 0 = PASS, non-zero = FAIL (reason: "verify-failed")
   - Capture stdout of `verify.sh` for the summary report

6. **Extract metrics from manifest:**
   - The manifest was already located in step 4
   - Read `metrics.total_tokens` → Tokens column
   - Read `metrics.rework_count` → Reworks column
   - Read `metrics.first_pass_yield` → FPY column (true → YES, false → NO, null → N/A)
   - If manifest doesn't exist (factory crashed early), show "—" for all metrics

#### Interface

```bash
./tests/run-factory-tests.sh                    # run all fixtures
./tests/run-factory-tests.sh 01-add-fibonacci    # run one fixture
```

#### Exit Code

- Exit 0 if all fixtures pass
- Exit 1 if any fixture fails
- Always runs all fixtures (does not abort on first failure)

#### Output

Prints a summary table to stdout after all runs. A fixture can FAIL for two reasons:
- `factory-error` — `synodic run` exited non-zero (escalation or crash). `verify.sh` was not run.
- `verify-failed` — factory succeeded but `verify.sh` returned non-zero. See verification output in `results.md`.

### Observability

Three layers, all file-based.

**1. Per-fixture artifacts** — The temp repo IS the artifact directory. The engine creates `.factory/` inside the temp repo during the run, so artifacts naturally live alongside the final code.

```
/tmp/synodic-test-<timestamp>/
  01-add-fibonacci/
    repo/
      src/lib.rs                   ← the code the factory produced
      .factory/<work-id>/
        manifest.json              ← full work item state + history + metrics
        build-report-attempt-1.json
        review-report-attempt-1.json
      spec/README.md               ← the spec that was executed
```

**2. Run logs** — Stderr output captured per fixture as `<output-dir>/<fixture-name>.log`. Contains chronological `[build]`, `[inspect]`, `[conveyor]` log lines emitted by the engine.

**3. Summary report** — A single `results.md` at the test output root with the summary table and per-fixture details (verification output, failure reason, path to artifacts).

**Cleanup** — Temp directories under `/tmp/synodic-test-*` are NOT automatically cleaned up. The verifier deletes them manually when done.

### verify.sh Contract

- **Input:** Receives the temp repo path as `$1`
- **Exit code:** 0 = all checks pass, non-zero = at least one check failed
- **Stdout:** Human-readable check results, one line per check (e.g., `PASS: lib.rs contains fn fibonacci`, `FAIL: cargo test failed`)
- **Stderr:** Ignored (may contain build output from cargo)
- **Idempotent:** Can be re-run against the same repo

### Key Decisions

- **Real Claude calls:** These tests cost real tokens. They are not run in CI automatically. A human triggers them intentionally.
- **Isolated temp dirs:** Each fixture runs in a fresh copy so failures don't contaminate each other or the real repo.
- **Shell-based:** The runner and verifiers are shell scripts. Simple to read, simple to debug, no framework overhead.
- **Graduated complexity:** Start with trivially verifiable fixtures, build confidence, then add harder ones.
- **Build before run:** The runner builds `synodic` from source rather than expecting it on PATH.

## Plan

- [ ] Create test fixture 01-add-fibonacci (repo + spec + verify.sh)
- [ ] Create test fixture 02-add-cli-flag (repo + spec + verify.sh)
- [ ] Create test fixture 03-fix-bug (repo + spec + verify.sh)
- [ ] Implement run-factory-tests.sh orchestrator
- [ ] Run all fixtures and validate results

## Test

- [ ] `./tests/run-factory-tests.sh` executes without script errors
- [ ] Each fixture's verify.sh correctly detects pass/fail
- [ ] Summary table and results.md are generated correctly
- [ ] Artifacts are preserved in temp directory for inspection

## Notes

- Future extensions: dogfood mode (run Synodic's own specs), Cargo `#[ignore]` integration tests, cost tracking across runs
