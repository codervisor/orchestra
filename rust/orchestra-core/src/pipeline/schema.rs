use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::validate;

// ---------------------------------------------------------------------------
// Pipeline YAML schema — 4-type step system per spec 061
// ---------------------------------------------------------------------------

/// Top-level pipeline definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub config: HashMap<String, serde_yaml::Value>,
    pub steps: Vec<Step>,
}

/// A single step in the pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    pub name: String,
    #[serde(flatten)]
    pub kind: StepKind,
    // Middleware
    #[serde(default)]
    pub retry: Option<u32>,
    #[serde(default)]
    pub timeout: Option<u64>,
    #[serde(default)]
    pub log: Option<String>,
    #[serde(default)]
    pub on_fail: Option<String>,
    #[serde(default)]
    pub condition: Option<String>,
}

/// The 4 step types (agent, run, branch, fan).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum StepKind {
    Agent(AgentStep),
    Run(RunStep),
    Branch(BranchStep),
    Fan(FanStep),
}

/// Agent step: invoke `claude -p` with constrained tools and structured output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStep {
    /// Path to prompt template (Markdown).
    pub prompt: String,
    /// Allowed tools list.
    #[serde(default)]
    pub tools: Vec<String>,
    /// Maximum agent turns.
    #[serde(default)]
    pub max_turns: Option<u32>,
    /// Whether to run in an isolated worktree.
    #[serde(default)]
    pub isolation: Option<String>,
    /// JSON Schema for structured output.
    #[serde(default)]
    pub output_schema: Option<String>,
    /// Context variables injected into the prompt.
    #[serde(default)]
    pub context: HashMap<String, String>,
}

/// Run step: execute commands. Flags enable gate/watch behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunStep {
    /// Command to execute.
    #[serde(default)]
    pub command: Option<String>,
    /// Gate group names to check (e.g. ["preflight"]).
    #[serde(default)]
    pub check: Vec<String>,
    /// File-match glob patterns (only run when changed files match).
    #[serde(default, rename = "match")]
    pub match_patterns: Vec<String>,
    /// Polling configuration for watch behavior.
    #[serde(default)]
    pub poll: Option<PollConfig>,
}

/// Polling configuration for run steps with watch behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollConfig {
    /// Polling interval in milliseconds.
    pub interval: u64,
    /// Total timeout in milliseconds.
    pub timeout: u64,
}

/// Branch step: route based on verdict (approve/rework/exhaust).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchStep {
    /// Variable reference for the input value (e.g. "steps.inspect.verdict").
    pub input: String,
    /// Step name to jump to on approval.
    pub approve: String,
    /// Step name to jump to on rework.
    pub rework: String,
    /// Max iterations before exhaust.
    #[serde(default = "default_max_iterations")]
    pub max_iterations: u32,
    /// Step name or action on exhaust (e.g. "escalate").
    #[serde(default)]
    pub exhaust: Option<String>,
}

fn default_max_iterations() -> u32 {
    3
}

/// Fan step: collection processing (parallel/sequential/loop).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FanStep {
    /// Processing mode.
    pub mode: FanMode,
    /// Collection to iterate over (variable reference).
    #[serde(default)]
    pub over: Option<String>,
    /// Termination condition for loop mode.
    #[serde(default)]
    pub until: Option<String>,
    /// Max iterations for loop mode.
    #[serde(default)]
    pub max_iterations: Option<u32>,
    /// Termination config for loop mode.
    #[serde(default)]
    pub termination: Option<TerminationConfig>,
    /// Inline step template for fan-out.
    #[serde(default)]
    pub step: Option<Box<Step>>,
    /// Inline steps for loop mode.
    #[serde(default)]
    pub steps: Vec<Step>,
}

/// Fan processing modes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FanMode {
    Parallel,
    Sequential,
    Loop,
}

/// Termination configuration for fan loop mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminationConfig {
    /// Number of consecutive clean iterations to terminate.
    #[serde(default)]
    pub consecutive_clean: Option<u32>,
    /// Number of plateau rounds before termination.
    #[serde(default)]
    pub plateau_rounds: Option<u32>,
}

// ---------------------------------------------------------------------------
// Pipeline parsing
// ---------------------------------------------------------------------------

impl Pipeline {
    /// Parse a pipeline from a YAML string.
    pub fn from_yaml(yaml: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(yaml)
    }

    /// Parse a pipeline from a YAML file.
    pub fn from_file(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("reading {}: {}", path.display(), e))?;
        Self::from_yaml(&content)
            .map_err(|e| anyhow::anyhow!("parsing {}: {}", path.display(), e))
    }

    /// Find a step by name.
    pub fn find_step(&self, name: &str) -> Option<&Step> {
        self.steps.iter().find(|s| s.name == name)
    }

    /// Validate structural correctness (see validate module for details).
    pub fn validate(&self) -> Vec<String> {
        validate::validate_pipeline(self)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_agent_step() {
        let yaml = r#"
name: test-pipeline
description: A test pipeline
steps:
  - name: build
    type: agent
    prompt: prompts/build.md
    tools: [Read, Edit, Write, Bash]
    max_turns: 50
    isolation: worktree
    output_schema: schemas/build-report.json
"#;
        let pipeline = Pipeline::from_yaml(yaml).unwrap();
        assert_eq!(pipeline.name, "test-pipeline");
        assert_eq!(pipeline.steps.len(), 1);
        match &pipeline.steps[0].kind {
            StepKind::Agent(a) => {
                assert_eq!(a.prompt, "prompts/build.md");
                assert_eq!(a.tools.len(), 4);
                assert_eq!(a.max_turns, Some(50));
                assert_eq!(a.isolation.as_deref(), Some("worktree"));
            }
            _ => panic!("expected agent step"),
        }
    }

    #[test]
    fn test_parse_run_step() {
        let yaml = r#"
name: test
steps:
  - name: gates
    type: run
    check: [preflight]
    match: ["*.rs", "*.ts"]
    retry: 2
    on_fail: rework(build)
"#;
        let pipeline = Pipeline::from_yaml(yaml).unwrap();
        let step = &pipeline.steps[0];
        assert_eq!(step.retry, Some(2));
        assert_eq!(step.on_fail.as_deref(), Some("rework(build)"));
        match &step.kind {
            StepKind::Run(r) => {
                assert_eq!(r.check, vec!["preflight"]);
                assert_eq!(r.match_patterns, vec!["*.rs", "*.ts"]);
            }
            _ => panic!("expected run step"),
        }
    }

    #[test]
    fn test_parse_branch_step() {
        let yaml = r#"
name: test
steps:
  - name: route
    type: branch
    input: steps.inspect.verdict
    approve: create-pr
    rework: build
    max_iterations: 3
    exhaust: escalate
"#;
        let pipeline = Pipeline::from_yaml(yaml).unwrap();
        match &pipeline.steps[0].kind {
            StepKind::Branch(b) => {
                assert_eq!(b.input, "steps.inspect.verdict");
                assert_eq!(b.approve, "create-pr");
                assert_eq!(b.rework, "build");
                assert_eq!(b.max_iterations, 3);
                assert_eq!(b.exhaust.as_deref(), Some("escalate"));
            }
            _ => panic!("expected branch step"),
        }
    }

    #[test]
    fn test_parse_fan_step_parallel() {
        let yaml = r#"
name: test
steps:
  - name: solve-leaves
    type: fan
    mode: parallel
    over: pending_nodes
"#;
        let pipeline = Pipeline::from_yaml(yaml).unwrap();
        match &pipeline.steps[0].kind {
            StepKind::Fan(f) => {
                assert!(matches!(f.mode, FanMode::Parallel));
                assert_eq!(f.over.as_deref(), Some("pending_nodes"));
            }
            _ => panic!("expected fan step"),
        }
    }

    #[test]
    fn test_parse_fan_step_loop() {
        let yaml = r#"
name: test
steps:
  - name: adversarial-loop
    type: fan
    mode: loop
    until: terminated
    max_iterations: 5
    termination:
      consecutive_clean: 2
      plateau_rounds: 3
"#;
        let pipeline = Pipeline::from_yaml(yaml).unwrap();
        match &pipeline.steps[0].kind {
            StepKind::Fan(f) => {
                assert!(matches!(f.mode, FanMode::Loop));
                assert_eq!(f.until.as_deref(), Some("terminated"));
                assert_eq!(f.max_iterations, Some(5));
                let t = f.termination.as_ref().unwrap();
                assert_eq!(t.consecutive_clean, Some(2));
                assert_eq!(t.plateau_rounds, Some(3));
            }
            _ => panic!("expected fan step"),
        }
    }

    #[test]
    fn test_parse_middleware() {
        let yaml = r#"
name: test
steps:
  - name: gated
    type: run
    command: cargo check
    retry: 3
    timeout: 60000
    log: logs/gate.log
    on_fail: escalate
    condition: config.run_gates
"#;
        let pipeline = Pipeline::from_yaml(yaml).unwrap();
        let step = &pipeline.steps[0];
        assert_eq!(step.retry, Some(3));
        assert_eq!(step.timeout, Some(60000));
        assert_eq!(step.log.as_deref(), Some("logs/gate.log"));
        assert_eq!(step.on_fail.as_deref(), Some("escalate"));
        assert_eq!(step.condition.as_deref(), Some("config.run_gates"));
    }

    #[test]
    fn test_parse_config() {
        let yaml = r#"
name: test
config:
  critic_modes: [syntax, edge-cases, concurrency]
  max_rework: 3
steps:
  - name: step1
    type: run
    command: echo hello
"#;
        let pipeline = Pipeline::from_yaml(yaml).unwrap();
        assert_eq!(pipeline.config.len(), 2);
    }

    #[test]
    fn test_find_step() {
        let yaml = r#"
name: test
steps:
  - name: build
    type: agent
    prompt: build.md
  - name: inspect
    type: agent
    prompt: inspect.md
"#;
        let pipeline = Pipeline::from_yaml(yaml).unwrap();
        assert!(pipeline.find_step("build").is_some());
        assert!(pipeline.find_step("inspect").is_some());
        assert!(pipeline.find_step("nonexistent").is_none());
    }

    // -----------------------------------------------------------------------
    // Spec 058 / 061: Additional schema parsing tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_all_four_step_types_in_one_pipeline() {
        // Spec 058 defines exactly 4 step types. Verify a pipeline with all 4 parses.
        let yaml = r#"
name: full-pipeline
description: Pipeline exercising all 4 step types
config:
  max_rework: 3
steps:
  - name: build
    type: agent
    prompt: prompts/build.md
    tools: [Read, Edit, Write, Bash]
    max_turns: 50
  - name: gates
    type: run
    check: [preflight]
    match: ["*.rs"]
    retry: 2
    on_fail: rework(build)
  - name: inspect
    type: agent
    prompt: prompts/inspect.md
  - name: route
    type: branch
    input: steps.inspect.verdict
    approve: create-pr
    rework: build
    max_iterations: 3
    exhaust: escalate
  - name: create-pr
    type: run
    command: gh pr create
  - name: solve-leaves
    type: fan
    mode: parallel
    over: pending_nodes
  - name: adversarial-loop
    type: fan
    mode: loop
    until: terminated
    max_iterations: 5
    termination:
      consecutive_clean: 2
      plateau_rounds: 3
"#;
        let pipeline = Pipeline::from_yaml(yaml).unwrap();
        assert_eq!(pipeline.name, "full-pipeline");
        assert_eq!(pipeline.steps.len(), 7);

        // Verify each step type is present
        let types: Vec<&str> = pipeline
            .steps
            .iter()
            .map(|s| match &s.kind {
                StepKind::Agent(_) => "agent",
                StepKind::Run(_) => "run",
                StepKind::Branch(_) => "branch",
                StepKind::Fan(_) => "fan",
            })
            .collect();
        assert!(types.contains(&"agent"));
        assert!(types.contains(&"run"));
        assert!(types.contains(&"branch"));
        assert!(types.contains(&"fan"));
    }

    #[test]
    fn test_malformed_yaml_returns_error() {
        let bad_yaml = "this is not: [valid: yaml: {{{";
        assert!(Pipeline::from_yaml(bad_yaml).is_err());
    }

    #[test]
    fn test_missing_required_fields_error() {
        // Missing name field
        let yaml = r#"
steps:
  - name: build
    type: agent
    prompt: build.md
"#;
        assert!(Pipeline::from_yaml(yaml).is_err());
    }

    #[test]
    fn test_agent_step_context_map() {
        let yaml = r#"
name: test
steps:
  - name: ci-fix
    type: agent
    prompt: prompts/ci-fix.md
    context:
      build_diff: "${steps.build.diff}"
      error_output: "${steps.gates.failures}"
"#;
        let pipeline = Pipeline::from_yaml(yaml).unwrap();
        match &pipeline.steps[0].kind {
            StepKind::Agent(a) => {
                assert_eq!(a.context.len(), 2);
                assert_eq!(a.context["build_diff"], "${steps.build.diff}");
                assert_eq!(a.context["error_output"], "${steps.gates.failures}");
            }
            _ => panic!("expected agent step"),
        }
    }

    #[test]
    fn test_run_step_with_poll_config() {
        let yaml = r#"
name: test
steps:
  - name: wait-ci
    type: run
    command: gh pr checks --watch
    poll:
      interval: 30000
      timeout: 600000
"#;
        let pipeline = Pipeline::from_yaml(yaml).unwrap();
        match &pipeline.steps[0].kind {
            StepKind::Run(r) => {
                let poll = r.poll.as_ref().unwrap();
                assert_eq!(poll.interval, 30000);
                assert_eq!(poll.timeout, 600000);
            }
            _ => panic!("expected run step"),
        }
    }

    #[test]
    fn test_branch_default_max_iterations() {
        let yaml = r#"
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
"#;
        let pipeline = Pipeline::from_yaml(yaml).unwrap();
        match &pipeline.steps[1].kind {
            StepKind::Branch(b) => {
                assert_eq!(b.max_iterations, 3, "default max_iterations should be 3");
            }
            _ => panic!("expected branch step"),
        }
    }

    #[test]
    fn test_fan_sequential_mode() {
        let yaml = r#"
name: test
steps:
  - name: process-nodes
    type: fan
    mode: sequential
    over: leaf_nodes
"#;
        let pipeline = Pipeline::from_yaml(yaml).unwrap();
        match &pipeline.steps[0].kind {
            StepKind::Fan(f) => {
                assert!(matches!(f.mode, FanMode::Sequential));
                assert_eq!(f.over.as_deref(), Some("leaf_nodes"));
            }
            _ => panic!("expected fan step"),
        }
    }

    #[test]
    fn test_pipeline_validate_returns_errors() {
        let yaml = r#"
name: test
steps:
  - name: bad-run
    type: run
"#;
        let pipeline = Pipeline::from_yaml(yaml).unwrap();
        let errors = pipeline.validate();
        assert!(!errors.is_empty(), "validation should catch missing command/check");
    }

    #[test]
    fn test_pipeline_description_optional() {
        let yaml = r#"
name: minimal
steps:
  - name: step1
    type: run
    command: echo ok
"#;
        let pipeline = Pipeline::from_yaml(yaml).unwrap();
        assert_eq!(pipeline.description, "");
    }
}
