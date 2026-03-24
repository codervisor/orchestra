use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

// ---------------------------------------------------------------------------
// Gate System — declarative preflight checks per spec 062
// ---------------------------------------------------------------------------

/// Gate definition file structure (gates.yml).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatesConfig {
    pub gates: GateGroups,
}

/// Named gate groups (e.g. "preflight").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateGroups {
    #[serde(default)]
    pub preflight: Vec<GateEntry>,
}

/// A single gate entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateEntry {
    pub name: String,
    /// Glob pattern for file-match filtering.
    #[serde(default, rename = "match")]
    pub match_pattern: Option<String>,
    /// Command to execute.
    pub command: String,
}

/// Result of running all gates in a group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateGroupResult {
    pub passed: bool,
    pub failures: Vec<GateFailure>,
    pub skipped: Vec<String>,
}

/// A single gate failure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateFailure {
    pub name: String,
    pub output: String,
}

/// Load gates.yml from the harness directory.
pub fn load_gates(harness_dir: &Path) -> Result<GatesConfig> {
    let path = harness_dir.join("gates.yml");
    if !path.exists() {
        // Return empty config if no gates.yml exists.
        return Ok(GatesConfig {
            gates: GateGroups {
                preflight: Vec::new(),
            },
        });
    }
    let content =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    serde_yaml::from_str(&content).with_context(|| format!("parsing {}", path.display()))
}

/// Get changed files via `git diff --name-only` against base ref.
pub fn get_changed_files(repo_root: &Path) -> Result<Vec<String>> {
    // Try HEAD first, fall back to listing all tracked files.
    let output = Command::new("git")
        .args(["diff", "--name-only", "HEAD"])
        .current_dir(repo_root)
        .output()
        .context("running git diff")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let files: Vec<String> = stdout
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| l.to_string())
        .collect();

    // If no changes against HEAD, also check staged.
    if files.is_empty() {
        let output = Command::new("git")
            .args(["diff", "--name-only", "--cached"])
            .current_dir(repo_root)
            .output()
            .context("running git diff --cached")?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Ok(stdout
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| l.to_string())
            .collect());
    }

    Ok(files)
}

/// Check if any changed files match a glob pattern.
pub fn files_match_pattern(files: &[String], pattern: &str) -> bool {
    let glob_pattern = glob::Pattern::new(pattern);
    match glob_pattern {
        Ok(p) => files.iter().any(|f| {
            // Match against the filename and the full path.
            let filename = Path::new(f)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            p.matches(&filename) || p.matches(f)
        }),
        Err(_) => false,
    }
}

/// Run specified gate groups and return aggregated results.
pub fn run_gate_groups(
    groups: &[String],
    match_patterns: &[String],
    harness_dir: &Path,
    repo_root: &Path,
) -> Result<GateGroupResult> {
    let config = load_gates(harness_dir)?;
    let changed_files = get_changed_files(repo_root).unwrap_or_default();
    let mut failures = Vec::new();
    let mut skipped = Vec::new();

    for group_name in groups {
        let gates = match group_name.as_str() {
            "preflight" => &config.gates.preflight,
            _ => {
                eprintln!("warning: unknown gate group '{}'", group_name);
                continue;
            }
        };

        for gate in gates {
            // File-match filtering: skip if no changed files match.
            if let Some(pattern) = &gate.match_pattern {
                if !changed_files.is_empty() && !files_match_pattern(&changed_files, pattern) {
                    skipped.push(gate.name.clone());
                    continue;
                }
            }

            // Additional match patterns from the step.
            if !match_patterns.is_empty()
                && !match_patterns
                    .iter()
                    .any(|p| files_match_pattern(&changed_files, p))
            {
                skipped.push(gate.name.clone());
                continue;
            }

            // Execute the gate command.
            let output = Command::new("sh")
                .arg("-c")
                .arg(&gate.command)
                .current_dir(repo_root)
                .output()
                .with_context(|| format!("executing gate '{}'", gate.name))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let combined = if stderr.is_empty() {
                    stdout
                } else {
                    format!("{}\n{}", stdout, stderr)
                };
                failures.push(GateFailure {
                    name: gate.name.clone(),
                    output: combined,
                });
            }
        }
    }

    Ok(GateGroupResult {
        passed: failures.is_empty(),
        failures,
        skipped,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_gates_yaml() {
        let yaml = r#"
gates:
  preflight:
    - name: rust-check
      match: "*.rs"
      command: cd cli && cargo check
    - name: rust-lint
      match: "*.rs"
      command: cd cli && cargo clippy -- -D warnings
    - name: ts-typecheck
      match: "*.ts,*.tsx"
      command: npx tsc --noEmit
"#;
        let config: GatesConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.gates.preflight.len(), 3);
        assert_eq!(config.gates.preflight[0].name, "rust-check");
        assert_eq!(
            config.gates.preflight[0].match_pattern.as_deref(),
            Some("*.rs")
        );
    }

    #[test]
    fn test_files_match_pattern_rs() {
        let files = vec![
            "src/main.rs".to_string(),
            "src/lib.rs".to_string(),
            "README.md".to_string(),
        ];
        assert!(files_match_pattern(&files, "*.rs"));
        assert!(files_match_pattern(&files, "*.md"));
        assert!(!files_match_pattern(&files, "*.py"));
    }

    #[test]
    fn test_files_match_pattern_empty() {
        let files: Vec<String> = vec![];
        assert!(!files_match_pattern(&files, "*.rs"));
    }

    #[test]
    fn test_gate_group_result_serialization() {
        let result = GateGroupResult {
            passed: false,
            failures: vec![GateFailure {
                name: "rust-check".to_string(),
                output: "error[E0308]: mismatched types".to_string(),
            }],
            skipped: vec!["ts-typecheck".to_string()],
        };
        let json = serde_json::to_string_pretty(&result).unwrap();
        assert!(json.contains("rust-check"));
        assert!(json.contains("ts-typecheck"));
    }

    #[test]
    fn test_empty_gates_config() {
        let config = GatesConfig {
            gates: GateGroups {
                preflight: Vec::new(),
            },
        };
        assert!(config.gates.preflight.is_empty());
    }

    // -----------------------------------------------------------------------
    // Spec 062: Additional gate system tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_files_match_pattern_multiple_extensions() {
        let files = vec![
            "src/main.rs".to_string(),
            "src/app.ts".to_string(),
            "src/style.css".to_string(),
            "README.md".to_string(),
        ];
        assert!(files_match_pattern(&files, "*.ts"));
        assert!(files_match_pattern(&files, "*.css"));
        assert!(!files_match_pattern(&files, "*.py"));
        assert!(!files_match_pattern(&files, "*.go"));
    }

    #[test]
    fn test_files_match_pattern_full_path() {
        let files = vec!["cli/synodic/src/main.rs".to_string()];
        // Should match by filename
        assert!(files_match_pattern(&files, "*.rs"));
    }

    #[test]
    fn test_gate_group_result_all_passed() {
        let result = GateGroupResult {
            passed: true,
            failures: vec![],
            skipped: vec![],
        };
        assert!(result.passed);
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"passed\":true"));
    }

    #[test]
    fn test_gate_group_result_with_skipped() {
        let result = GateGroupResult {
            passed: true,
            failures: vec![],
            skipped: vec!["ts-typecheck".to_string(), "python-lint".to_string()],
        };
        assert!(result.passed);
        assert_eq!(result.skipped.len(), 2);
    }

    #[test]
    fn test_gate_group_result_multiple_failures() {
        let result = GateGroupResult {
            passed: false,
            failures: vec![
                GateFailure {
                    name: "rust-check".to_string(),
                    output: "error[E0308]".to_string(),
                },
                GateFailure {
                    name: "rust-lint".to_string(),
                    output: "warning: unused variable".to_string(),
                },
            ],
            skipped: vec![],
        };
        assert!(!result.passed);
        assert_eq!(result.failures.len(), 2);
    }

    #[test]
    fn test_load_gates_missing_dir() {
        let result = load_gates(std::path::Path::new("/nonexistent/dir"));
        // Should return empty config when gates.yml doesn't exist
        let config = result.unwrap();
        assert!(config.gates.preflight.is_empty());
    }

    #[test]
    fn test_parse_gates_yaml_no_match_pattern() {
        let yaml = r#"
gates:
  preflight:
    - name: custom-rules
      command: .harness/scripts/run-rules.sh
"#;
        let config: GatesConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.gates.preflight.len(), 1);
        assert!(config.gates.preflight[0].match_pattern.is_none());
    }

    #[test]
    fn test_parse_gates_yaml_per_spec_062() {
        // Full gates.yml from spec 062
        let yaml = r#"
gates:
  preflight:
    - name: rust-check
      match: "*.rs"
      command: cd cli && cargo check
    - name: rust-lint
      match: "*.rs"
      command: cd cli && cargo clippy -- -D warnings
    - name: ts-typecheck
      match: "*.ts,*.tsx"
      command: npx tsc --noEmit
    - name: custom-rules
      match: "*"
      command: .harness/scripts/run-rules.sh
"#;
        let config: GatesConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.gates.preflight.len(), 4);
        // Verify all gates have correct structure
        for gate in &config.gates.preflight {
            assert!(!gate.name.is_empty());
            assert!(!gate.command.is_empty());
        }
    }
}
