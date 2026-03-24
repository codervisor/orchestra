use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::pipeline::gates;
use crate::pipeline::schema::*;
use crate::pipeline::vars::VarContext;

// ---------------------------------------------------------------------------
// Pipeline executor — runs pipelines step by step per spec 061
// ---------------------------------------------------------------------------

/// Result of executing a single step.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StepResult {
    pub name: String,
    pub status: StepStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StepStatus {
    Passed,
    Failed,
    Skipped,
}

/// Result of executing a full pipeline.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PipelineResult {
    pub pipeline: String,
    pub status: String,
    pub steps: Vec<StepResult>,
    pub duration_ms: u64,
}

/// Configuration for a pipeline execution.
pub struct ExecConfig {
    pub pipeline_path: PathBuf,
    pub spec_path: Option<String>,
    pub repo_root: PathBuf,
    pub harness_dir: PathBuf,
    pub dry_run: bool,
    pub json_output: bool,
}

/// Execute a pipeline.
pub fn execute(config: &ExecConfig) -> Result<PipelineResult> {
    let pipeline = Pipeline::from_file(&config.pipeline_path)?;

    // Pre-execution validation.
    let errors = pipeline.validate();
    if !errors.is_empty() {
        anyhow::bail!(
            "pipeline validation failed:\n  {}",
            errors.join("\n  ")
        );
    }

    let start = std::time::Instant::now();
    let mut ctx = VarContext::new();
    let mut step_results: Vec<StepResult> = Vec::new();

    // Populate config scope.
    for (k, v) in &pipeline.config {
        ctx.set(
            &format!("config.{}", k),
            &serde_yaml::to_string(v).unwrap_or_default().trim().to_string(),
        );
    }

    // Populate spec scope.
    if let Some(spec_path) = &config.spec_path {
        ctx.set("spec.path", spec_path);
    }

    if config.dry_run {
        eprintln!("[dry-run] Pipeline: {}", pipeline.name);
        for step in &pipeline.steps {
            eprintln!("[dry-run]   Step: {} (type: {:?})", step.name, step_type_name(&step.kind));
        }
        return Ok(PipelineResult {
            pipeline: pipeline.name.clone(),
            status: "dry-run".to_string(),
            steps: vec![],
            duration_ms: start.elapsed().as_millis() as u64,
        });
    }

    // Sequential step execution with branch/fan routing.
    let mut i = 0;
    let mut branch_iterations: HashMap<String, u32> = HashMap::new();

    while i < pipeline.steps.len() {
        let step = &pipeline.steps[i];

        // Check condition.
        if let Some(cond) = &step.condition {
            if let Some(val) = ctx.get(cond) {
                if val == "false" || val == "0" || val.is_empty() {
                    step_results.push(StepResult {
                        name: step.name.clone(),
                        status: StepStatus::Skipped,
                        output: Some("condition not met".to_string()),
                        exit_code: None,
                        duration_ms: 0,
                    });
                    i += 1;
                    continue;
                }
            }
        }

        let step_start = std::time::Instant::now();
        let result = execute_step_with_middleware(step, &ctx, config)?;
        let duration = step_start.elapsed().as_millis() as u64;

        // Update step output in context.
        if let Some(output) = &result.output {
            ctx.set(&format!("steps.{}.output", step.name), output);
        }
        match &result.status {
            StepStatus::Passed => ctx.set(&format!("steps.{}.status", step.name), "passed"),
            StepStatus::Failed => ctx.set(&format!("steps.{}.status", step.name), "failed"),
            StepStatus::Skipped => ctx.set(&format!("steps.{}.status", step.name), "skipped"),
        }

        let step_result = StepResult {
            name: step.name.clone(),
            status: result.status.clone(),
            output: result.output.clone(),
            exit_code: result.exit_code,
            duration_ms: duration,
        };

        // Handle branch routing.
        if let StepKind::Branch(b) = &step.kind {
            let count = branch_iterations.entry(step.name.clone()).or_insert(0);
            *count += 1;

            if *count > b.max_iterations {
                // Exhaust.
                step_results.push(step_result.clone());
                if let Some(exhaust) = &b.exhaust {
                    if exhaust == "escalate" {
                        return Ok(PipelineResult {
                            pipeline: pipeline.name.clone(),
                            status: "escalated".to_string(),
                            steps: step_results,
                            duration_ms: start.elapsed().as_millis() as u64,
                        });
                    }
                    if let Some(pos) = pipeline.steps.iter().position(|s| s.name == *exhaust) {
                        i = pos;
                        continue;
                    }
                }
                return Ok(PipelineResult {
                    pipeline: pipeline.name.clone(),
                    status: "exhausted".to_string(),
                    steps: step_results,
                    duration_ms: start.elapsed().as_millis() as u64,
                });
            }

            // Route based on verdict.
            let verdict = result
                .output
                .as_deref()
                .unwrap_or("")
                .trim()
                .to_lowercase();

            let target = if verdict.contains("approve") || verdict.contains("pass") {
                &b.approve
            } else {
                &b.rework
            };

            if let Some(pos) = pipeline.steps.iter().position(|s| s.name == *target) {
                step_results.push(step_result.clone());
                i = pos;
                continue;
            }
        }

        // Handle on_fail rework routing.
        if matches!(result.status, StepStatus::Failed) {
            if let Some(on_fail) = &step.on_fail {
                if let Some(target) =
                    on_fail.strip_prefix("rework(").and_then(|s| s.strip_suffix(')'))
                {
                    if let Some(pos) = pipeline.steps.iter().position(|s| s.name == target) {
                        step_results.push(step_result.clone());
                        i = pos;
                        continue;
                    }
                }
                if on_fail == "escalate" {
                    step_results.push(step_result.clone());
                    return Ok(PipelineResult {
                        pipeline: pipeline.name.clone(),
                        status: "escalated".to_string(),
                        steps: step_results,
                        duration_ms: start.elapsed().as_millis() as u64,
                    });
                }
            }
        }

        step_results.push(step_result);
        i += 1;
    }

    Ok(PipelineResult {
        pipeline: pipeline.name.clone(),
        status: "completed".to_string(),
        steps: step_results,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

/// Execute a step with middleware (retry, timeout, logging).
fn execute_step_with_middleware(
    step: &Step,
    ctx: &VarContext,
    config: &ExecConfig,
) -> Result<StepResult> {
    let max_attempts = step.retry.unwrap_or(0) + 1;
    let mut last_result = None;

    for attempt in 0..max_attempts {
        if attempt > 0 {
            if let Some(log_path) = &step.log {
                let _ = append_log(
                    log_path,
                    &format!("step '{}' attempt {}/{}", step.name, attempt + 1, max_attempts),
                );
            }
        }

        let result = execute_step_inner(step, ctx, config)?;

        if matches!(result.status, StepStatus::Passed) {
            return Ok(result);
        }

        last_result = Some(result);
    }

    Ok(last_result.unwrap())
}

/// Execute a single step (no middleware).
fn execute_step_inner(step: &Step, ctx: &VarContext, config: &ExecConfig) -> Result<StepResult> {
    match &step.kind {
        StepKind::Agent(a) => execute_agent(step, a, ctx, config),
        StepKind::Run(r) => execute_run(step, r, ctx, config),
        StepKind::Branch(b) => execute_branch(step, b, ctx),
        StepKind::Fan(f) => execute_fan(step, f, ctx, config),
    }
}

/// Execute an agent step (`claude -p`).
fn execute_agent(
    step: &Step,
    agent: &AgentStep,
    ctx: &VarContext,
    config: &ExecConfig,
) -> Result<StepResult> {
    // Resolve prompt path and read template.
    let prompt_path = config.repo_root.join(&agent.prompt);
    let prompt_template = if prompt_path.exists() {
        std::fs::read_to_string(&prompt_path)
            .with_context(|| format!("reading prompt: {}", prompt_path.display()))?
    } else {
        agent.prompt.clone()
    };

    // Interpolate variables in prompt.
    let prompt = ctx.interpolate(&prompt_template).map_err(|e| {
        anyhow::anyhow!("step '{}': variable interpolation failed: {}", step.name, e)
    })?;

    // Inject context variables.
    let mut full_prompt = prompt;
    for (key, var_ref) in &agent.context {
        if let Ok(val) = ctx.interpolate(var_ref) {
            full_prompt.push_str(&format!("\n\n## {}\n{}", key, val));
        }
    }

    // Build claude -p command.
    let mut cmd = Command::new("claude");
    cmd.arg("-p").arg(&full_prompt);

    if let Some(max_turns) = agent.max_turns {
        cmd.arg("--max-turns").arg(max_turns.to_string());
    }

    if !agent.tools.is_empty() {
        cmd.arg("--allowedTools")
            .arg(agent.tools.join(","));
    }

    if let Some(schema_path) = &agent.output_schema {
        let full_path = config.repo_root.join(schema_path);
        if full_path.exists() {
            cmd.arg("--output-format").arg("json");
        }
    }

    cmd.current_dir(&config.repo_root);

    let output = cmd.output().with_context(|| {
        format!("step '{}': failed to execute claude -p", step.name)
    })?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let exit_code = output.status.code().unwrap_or(1);

    Ok(StepResult {
        name: step.name.clone(),
        status: if output.status.success() {
            StepStatus::Passed
        } else {
            StepStatus::Failed
        },
        output: Some(stdout),
        exit_code: Some(exit_code),
        duration_ms: 0,
    })
}

/// Execute a run step (command execution + gate checking).
fn execute_run(
    step: &Step,
    run: &RunStep,
    ctx: &VarContext,
    config: &ExecConfig,
) -> Result<StepResult> {
    // If check is specified, run gate groups.
    if !run.check.is_empty() {
        let gate_result =
            gates::run_gate_groups(&run.check, &run.match_patterns, &config.harness_dir, &config.repo_root)?;
        return Ok(StepResult {
            name: step.name.clone(),
            status: if gate_result.passed {
                StepStatus::Passed
            } else {
                StepStatus::Failed
            },
            output: Some(serde_json::to_string(&gate_result)?),
            exit_code: Some(if gate_result.passed { 0 } else { 1 }),
            duration_ms: 0,
        });
    }

    // Direct command execution.
    if let Some(command) = &run.command {
        let interpolated = ctx.interpolate(command).map_err(|e| {
            anyhow::anyhow!("step '{}': variable interpolation failed: {}", step.name, e)
        })?;

        // Polling mode.
        if let Some(poll) = &run.poll {
            return execute_poll(step, &interpolated, poll, &config.repo_root);
        }

        let output = Command::new("sh")
            .arg("-c")
            .arg(&interpolated)
            .current_dir(&config.repo_root)
            .output()
            .with_context(|| format!("step '{}': failed to execute command", step.name))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let combined = if stderr.is_empty() {
            stdout
        } else {
            format!("{}\n{}", stdout, stderr)
        };

        return Ok(StepResult {
            name: step.name.clone(),
            status: if output.status.success() {
                StepStatus::Passed
            } else {
                StepStatus::Failed
            },
            output: Some(combined),
            exit_code: output.status.code(),
            duration_ms: 0,
        });
    }

    anyhow::bail!("step '{}': run step has no command or check", step.name)
}

/// Execute a run step with polling.
fn execute_poll(
    step: &Step,
    command: &str,
    poll: &PollConfig,
    repo_root: &Path,
) -> Result<StepResult> {
    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_millis(poll.timeout);
    let interval = std::time::Duration::from_millis(poll.interval);

    loop {
        let output = Command::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(repo_root)
            .output()?;

        if output.status.success() {
            return Ok(StepResult {
                name: step.name.clone(),
                status: StepStatus::Passed,
                output: Some(String::from_utf8_lossy(&output.stdout).to_string()),
                exit_code: Some(0),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }

        if start.elapsed() >= timeout {
            return Ok(StepResult {
                name: step.name.clone(),
                status: StepStatus::Failed,
                output: Some(format!(
                    "poll timeout after {}ms",
                    start.elapsed().as_millis()
                )),
                exit_code: Some(1),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }

        std::thread::sleep(interval);
    }
}

/// Execute a branch step (verdict routing).
fn execute_branch(step: &Step, branch: &BranchStep, ctx: &VarContext) -> Result<StepResult> {
    let value = ctx
        .get(&branch.input)
        .unwrap_or("unknown")
        .to_string();

    Ok(StepResult {
        name: step.name.clone(),
        status: StepStatus::Passed,
        output: Some(value),
        exit_code: Some(0),
        duration_ms: 0,
    })
}

/// Execute a fan step (parallel/sequential/loop).
fn execute_fan(
    step: &Step,
    fan: &FanStep,
    ctx: &VarContext,
    config: &ExecConfig,
) -> Result<StepResult> {
    match fan.mode {
        FanMode::Loop => execute_fan_loop(step, fan, ctx, config),
        FanMode::Parallel | FanMode::Sequential => {
            execute_fan_collection(step, fan, ctx, config)
        }
    }
}

/// Execute fan in loop mode.
fn execute_fan_loop(
    step: &Step,
    fan: &FanStep,
    ctx: &VarContext,
    config: &ExecConfig,
) -> Result<StepResult> {
    let max_iter = fan.max_iterations.unwrap_or(10);
    let mut consecutive_clean = 0u32;
    let required_clean = fan
        .termination
        .as_ref()
        .and_then(|t| t.consecutive_clean)
        .unwrap_or(u32::MAX);

    for iteration in 0..max_iter {
        let mut loop_ctx = VarContext::new();
        // Copy parent context values.
        loop_ctx.set("loop.iteration", &iteration.to_string());

        // Execute inner steps.
        let mut all_passed = true;
        for inner_step in &fan.steps {
            let result = execute_step_with_middleware(inner_step, ctx, config)?;
            if !matches!(result.status, StepStatus::Passed) {
                all_passed = false;
            }
        }

        if all_passed {
            consecutive_clean += 1;
            if consecutive_clean >= required_clean {
                return Ok(StepResult {
                    name: step.name.clone(),
                    status: StepStatus::Passed,
                    output: Some(format!(
                        "terminated after {} iterations ({} consecutive clean)",
                        iteration + 1,
                        consecutive_clean
                    )),
                    exit_code: Some(0),
                    duration_ms: 0,
                });
            }
        } else {
            consecutive_clean = 0;
        }
    }

    Ok(StepResult {
        name: step.name.clone(),
        status: StepStatus::Passed,
        output: Some(format!("completed {} iterations", max_iter)),
        exit_code: Some(0),
        duration_ms: 0,
    })
}

/// Execute fan in parallel/sequential mode over a collection.
fn execute_fan_collection(
    step: &Step,
    fan: &FanStep,
    ctx: &VarContext,
    config: &ExecConfig,
) -> Result<StepResult> {
    // For now, execute inner steps sequentially (true parallelism requires async).
    let mut results = Vec::new();

    if !fan.steps.is_empty() {
        for inner_step in &fan.steps {
            let result = execute_step_with_middleware(inner_step, ctx, config)?;
            results.push(result);
        }
    }

    let all_passed = results.iter().all(|r| matches!(r.status, StepStatus::Passed));

    Ok(StepResult {
        name: step.name.clone(),
        status: if all_passed {
            StepStatus::Passed
        } else {
            StepStatus::Failed
        },
        output: Some(serde_json::to_string(&results)?),
        exit_code: Some(if all_passed { 0 } else { 1 }),
        duration_ms: 0,
    })
}

fn step_type_name(kind: &StepKind) -> &'static str {
    match kind {
        StepKind::Agent(_) => "agent",
        StepKind::Run(_) => "run",
        StepKind::Branch(_) => "branch",
        StepKind::Fan(_) => "fan",
    }
}

fn append_log(path: &str, message: &str) -> Result<()> {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(file, "[{}] {}", chrono::Utc::now().to_rfc3339(), message)?;
    Ok(())
}
