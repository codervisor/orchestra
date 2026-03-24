use crate::pipeline::schema::*;
use std::collections::HashSet;

// ---------------------------------------------------------------------------
// Pipeline validation — pre-execution schema checking per spec 061
// ---------------------------------------------------------------------------

/// Validate a pipeline and return a list of errors (empty = valid).
pub fn validate_pipeline(pipeline: &Pipeline) -> Vec<String> {
    let mut errors = Vec::new();

    if pipeline.name.is_empty() {
        errors.push("pipeline name is required".to_string());
    }

    if pipeline.steps.is_empty() {
        errors.push("pipeline must have at least one step".to_string());
    }

    // Check for duplicate step names.
    let mut seen = HashSet::new();
    for step in &pipeline.steps {
        if step.name.is_empty() {
            errors.push("step name is required".to_string());
        } else if !seen.insert(&step.name) {
            errors.push(format!("duplicate step name: {}", step.name));
        }
    }

    // Collect all step names for reference validation.
    let step_names: HashSet<&str> = pipeline.steps.iter().map(|s| s.name.as_str()).collect();

    for step in &pipeline.steps {
        validate_step(step, &step_names, &mut errors);
    }

    errors
}

fn validate_step(step: &Step, step_names: &HashSet<&str>, errors: &mut Vec<String>) {
    match &step.kind {
        StepKind::Agent(a) => {
            if a.prompt.is_empty() {
                errors.push(format!("step '{}': agent prompt is required", step.name));
            }
        }
        StepKind::Run(r) => {
            if r.command.is_none() && r.check.is_empty() {
                errors.push(format!(
                    "step '{}': run step needs either command or check",
                    step.name
                ));
            }
        }
        StepKind::Branch(b) => {
            if b.input.is_empty() {
                errors.push(format!("step '{}': branch input is required", step.name));
            }
            if !step_names.contains(b.approve.as_str()) {
                errors.push(format!(
                    "step '{}': approve target '{}' not found",
                    step.name, b.approve
                ));
            }
            if !step_names.contains(b.rework.as_str()) {
                errors.push(format!(
                    "step '{}': rework target '{}' not found",
                    step.name, b.rework
                ));
            }
            if b.max_iterations == 0 {
                errors.push(format!(
                    "step '{}': max_iterations must be > 0",
                    step.name
                ));
            }
        }
        StepKind::Fan(f) => {
            match f.mode {
                FanMode::Parallel | FanMode::Sequential => {
                    if f.over.is_none() && f.steps.is_empty() && f.step.is_none() {
                        errors.push(format!(
                            "step '{}': fan parallel/sequential needs 'over' or 'steps'/'step'",
                            step.name
                        ));
                    }
                }
                FanMode::Loop => {
                    if f.until.is_none() && f.max_iterations.is_none() {
                        errors.push(format!(
                            "step '{}': fan loop needs 'until' or 'max_iterations'",
                            step.name
                        ));
                    }
                }
            }
        }
    }

    // Validate on_fail references.
    if let Some(on_fail) = &step.on_fail {
        if let Some(target) = on_fail.strip_prefix("rework(").and_then(|s| s.strip_suffix(')')) {
            if !step_names.contains(target) {
                errors.push(format!(
                    "step '{}': on_fail rework target '{}' not found",
                    step.name, target
                ));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_and_validate(yaml: &str) -> Vec<String> {
        let pipeline = Pipeline::from_yaml(yaml).unwrap();
        validate_pipeline(&pipeline)
    }

    #[test]
    fn test_valid_pipeline() {
        let errors = parse_and_validate(
            r#"
name: factory
steps:
  - name: build
    type: agent
    prompt: prompts/build.md
  - name: inspect
    type: agent
    prompt: prompts/inspect.md
  - name: route
    type: branch
    input: steps.inspect.verdict
    approve: build
    rework: build
    max_iterations: 3
"#,
        );
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
    }

    #[test]
    fn test_empty_pipeline() {
        let errors = parse_and_validate(
            r#"
name: empty
steps: []
"#,
        );
        assert!(errors.iter().any(|e| e.contains("at least one step")));
    }

    #[test]
    fn test_duplicate_step_names() {
        let errors = parse_and_validate(
            r#"
name: test
steps:
  - name: build
    type: agent
    prompt: a.md
  - name: build
    type: agent
    prompt: b.md
"#,
        );
        assert!(errors.iter().any(|e| e.contains("duplicate step name")));
    }

    #[test]
    fn test_branch_invalid_target() {
        let errors = parse_and_validate(
            r#"
name: test
steps:
  - name: build
    type: agent
    prompt: a.md
  - name: route
    type: branch
    input: verdict
    approve: nonexistent
    rework: build
"#,
        );
        assert!(errors.iter().any(|e| e.contains("approve target")));
    }

    #[test]
    fn test_run_step_needs_command_or_check() {
        let errors = parse_and_validate(
            r#"
name: test
steps:
  - name: empty-run
    type: run
"#,
        );
        assert!(errors
            .iter()
            .any(|e| e.contains("command or check")));
    }

    #[test]
    fn test_agent_needs_prompt() {
        let errors = parse_and_validate(
            r#"
name: test
steps:
  - name: no-prompt
    type: agent
    prompt: ""
"#,
        );
        assert!(errors.iter().any(|e| e.contains("prompt is required")));
    }

    #[test]
    fn test_fan_loop_needs_termination() {
        let errors = parse_and_validate(
            r#"
name: test
steps:
  - name: loop
    type: fan
    mode: loop
"#,
        );
        assert!(errors
            .iter()
            .any(|e| e.contains("until") && e.contains("max_iterations")));
    }

    #[test]
    fn test_on_fail_rework_invalid_target() {
        let errors = parse_and_validate(
            r#"
name: test
steps:
  - name: gates
    type: run
    command: cargo check
    on_fail: rework(missing)
"#,
        );
        assert!(errors.iter().any(|e| e.contains("rework target")));
    }

    // -----------------------------------------------------------------------
    // Spec 061: Additional validation tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_empty_step_name() {
        let errors = parse_and_validate(
            r#"
name: test
steps:
  - name: ""
    type: run
    command: echo ok
"#,
        );
        assert!(errors.iter().any(|e| e.contains("step name is required")));
    }

    #[test]
    fn test_empty_pipeline_name() {
        let errors = parse_and_validate(
            r#"
name: ""
steps:
  - name: step1
    type: run
    command: echo ok
"#,
        );
        assert!(errors.iter().any(|e| e.contains("pipeline name is required")));
    }

    #[test]
    fn test_branch_rework_target_missing() {
        let errors = parse_and_validate(
            r#"
name: test
steps:
  - name: build
    type: agent
    prompt: build.md
  - name: route
    type: branch
    input: verdict
    approve: build
    rework: nonexistent
"#,
        );
        assert!(errors.iter().any(|e| e.contains("rework target 'nonexistent'")));
    }

    #[test]
    fn test_branch_zero_max_iterations() {
        let errors = parse_and_validate(
            r#"
name: test
steps:
  - name: build
    type: agent
    prompt: build.md
  - name: route
    type: branch
    input: verdict
    approve: build
    rework: build
    max_iterations: 0
"#,
        );
        assert!(errors.iter().any(|e| e.contains("max_iterations must be > 0")));
    }

    #[test]
    fn test_fan_parallel_needs_over_or_steps() {
        let errors = parse_and_validate(
            r#"
name: test
steps:
  - name: empty-fan
    type: fan
    mode: parallel
"#,
        );
        assert!(errors.iter().any(|e| e.contains("needs 'over' or 'steps'")));
    }

    #[test]
    fn test_on_fail_valid_rework_target() {
        let errors = parse_and_validate(
            r#"
name: test
steps:
  - name: build
    type: agent
    prompt: build.md
  - name: gates
    type: run
    command: cargo check
    on_fail: rework(build)
"#,
        );
        assert!(errors.is_empty(), "valid on_fail should produce no errors: {:?}", errors);
    }

    #[test]
    fn test_on_fail_escalate_no_error() {
        let errors = parse_and_validate(
            r#"
name: test
steps:
  - name: step1
    type: run
    command: cargo test
    on_fail: escalate
"#,
        );
        // escalate is a valid on_fail action, not a rework target
        assert!(
            !errors.iter().any(|e| e.contains("rework target")),
            "escalate should not trigger rework target validation"
        );
    }

    #[test]
    fn test_full_factory_pipeline_validates() {
        // Simulates a factory pipeline per spec 063
        let errors = parse_and_validate(
            r#"
name: factory
description: Linear BUILD → INSPECT → PR pipeline
config:
  max_rework: 3
steps:
  - name: build
    type: agent
    prompt: skills/factory/prompts/build.md
    tools: [Read, Edit, Write, Bash, Glob, Grep]
    max_turns: 50
    isolation: worktree
  - name: preflight
    type: run
    check: [preflight]
    match: ["*.rs", "*.ts"]
    retry: 2
    on_fail: rework(build)
  - name: inspect
    type: agent
    prompt: skills/factory/prompts/inspect.md
    tools: [Read, Glob, Grep]
    max_turns: 10
  - name: route
    type: branch
    input: steps.inspect.verdict
    approve: create-pr
    rework: build
    max_iterations: 3
    exhaust: escalate
  - name: create-pr
    type: run
    command: gh pr create --title "${spec.title}" --body "${spec.summary}"
"#,
        );
        assert!(errors.is_empty(), "factory pipeline should validate: {:?}", errors);
    }

    #[test]
    fn test_adversarial_pipeline_validates() {
        // Simulates an adversarial pipeline per spec 063
        let errors = parse_and_validate(
            r#"
name: adversarial
description: Generative-adversarial quality hardening
config:
  critic_modes: [syntax-and-types, edge-cases, concurrency-safety]
steps:
  - name: generate
    type: agent
    prompt: skills/adversarial/prompts/generate.md
    tools: [Read, Edit, Write, Bash]
    max_turns: 30
  - name: gates
    type: run
    check: [preflight]
    retry: 1
    on_fail: rework(generate)
  - name: adversarial-loop
    type: fan
    mode: loop
    max_iterations: 5
    termination:
      consecutive_clean: 2
      plateau_rounds: 3
  - name: create-pr
    type: run
    command: gh pr create
"#,
        );
        assert!(errors.is_empty(), "adversarial pipeline should validate: {:?}", errors);
    }

    #[test]
    fn test_multiple_errors_collected() {
        let errors = parse_and_validate(
            r#"
name: ""
steps:
  - name: ""
    type: agent
    prompt: ""
  - name: bad-branch
    type: branch
    input: ""
    approve: nowhere
    rework: nowhere
    max_iterations: 0
"#,
        );
        // Should collect multiple errors, not stop at first
        assert!(errors.len() >= 3, "should have multiple errors: {:?}", errors);
    }
}
