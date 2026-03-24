use regex::Regex;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Variable interpolation — ${scope.field} substitution per spec 061
// ---------------------------------------------------------------------------

/// Variable context holding all available scopes for interpolation.
#[derive(Debug, Default)]
pub struct VarContext {
    /// Flat key-value store: "scope.field" -> value.
    values: HashMap<String, String>,
}

impl VarContext {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a scoped variable (e.g. "config.max_rework" -> "3").
    pub fn set(&mut self, key: &str, value: &str) {
        self.values.insert(key.to_string(), value.to_string());
    }

    /// Set all fields from a scope prefix + map.
    pub fn set_scope(&mut self, scope: &str, map: &HashMap<String, String>) {
        for (k, v) in map {
            self.values.insert(format!("{}.{}", scope, k), v.clone());
        }
    }

    /// Get a variable by its full scoped key.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(|s| s.as_str())
    }

    /// Interpolate all `${scope.field}` references in a string.
    /// Returns an error for unset variables (fail-fast per spec).
    pub fn interpolate(&self, input: &str) -> Result<String, VarError> {
        let re = Regex::new(r"\$\{([a-zA-Z_][a-zA-Z0-9_.]*)\}").unwrap();
        let mut result = input.to_string();
        let mut errors = Vec::new();

        // Collect all matches first to avoid mutable borrow issues.
        let matches: Vec<(String, String)> = re
            .captures_iter(input)
            .map(|cap| {
                let full = cap[0].to_string();
                let key = cap[1].to_string();
                (full, key)
            })
            .collect();

        for (full, key) in matches {
            match self.values.get(&key) {
                Some(val) => {
                    result = result.replacen(&full, val, 1);
                }
                None => {
                    errors.push(key);
                }
            }
        }

        if errors.is_empty() {
            Ok(result)
        } else {
            Err(VarError::UnsetVariables(errors))
        }
    }
}

/// Variable interpolation error.
#[derive(Debug)]
pub enum VarError {
    UnsetVariables(Vec<String>),
}

impl std::fmt::Display for VarError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VarError::UnsetVariables(vars) => {
                write!(f, "unset variables: {}", vars.join(", "))
            }
        }
    }
}

impl std::error::Error for VarError {}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_interpolation() {
        let mut ctx = VarContext::new();
        ctx.set("config.name", "factory");
        ctx.set("spec.path", "specs/001/README.md");

        let result = ctx.interpolate("Pipeline: ${config.name}, Spec: ${spec.path}");
        assert_eq!(
            result.unwrap(),
            "Pipeline: factory, Spec: specs/001/README.md"
        );
    }

    #[test]
    fn test_unset_variable_errors() {
        let ctx = VarContext::new();
        let result = ctx.interpolate("Value: ${config.missing}");
        assert!(result.is_err());
        match result.unwrap_err() {
            VarError::UnsetVariables(vars) => {
                assert_eq!(vars, vec!["config.missing"]);
            }
        }
    }

    #[test]
    fn test_no_variables() {
        let ctx = VarContext::new();
        let result = ctx.interpolate("No variables here");
        assert_eq!(result.unwrap(), "No variables here");
    }

    #[test]
    fn test_multiple_same_scope() {
        let mut ctx = VarContext::new();
        ctx.set("steps.build.diff", "+added line");
        ctx.set("steps.build.status", "passed");

        let result = ctx.interpolate("Diff: ${steps.build.diff}, Status: ${steps.build.status}");
        assert_eq!(result.unwrap(), "Diff: +added line, Status: passed");
    }

    #[test]
    fn test_set_scope() {
        let mut ctx = VarContext::new();
        let mut map = HashMap::new();
        map.insert("name".to_string(), "factory".to_string());
        map.insert("version".to_string(), "1.0".to_string());
        ctx.set_scope("config", &map);

        assert_eq!(ctx.get("config.name"), Some("factory"));
        assert_eq!(ctx.get("config.version"), Some("1.0"));
    }

    #[test]
    fn test_loop_scope() {
        let mut ctx = VarContext::new();
        ctx.set("loop.iteration", "2");
        ctx.set("loop.item", "node-auth");

        let result = ctx.interpolate("Iteration ${loop.iteration}: ${loop.item}");
        assert_eq!(result.unwrap(), "Iteration 2: node-auth");
    }

    // -----------------------------------------------------------------------
    // Spec 061: Additional variable interpolation tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_multiple_unset_variables_all_reported() {
        let ctx = VarContext::new();
        let result = ctx.interpolate("${a.b} and ${c.d} and ${e.f}");
        match result.unwrap_err() {
            VarError::UnsetVariables(vars) => {
                assert_eq!(vars.len(), 3, "all 3 unset variables should be reported");
            }
        }
    }

    #[test]
    fn test_same_variable_twice_in_string() {
        let mut ctx = VarContext::new();
        ctx.set("config.name", "factory");
        let result = ctx
            .interpolate("${config.name} is called ${config.name}")
            .unwrap();
        assert_eq!(result, "factory is called factory");
    }

    #[test]
    fn test_literal_dollar_brace_not_a_variable() {
        let ctx = VarContext::new();
        // Text without proper variable syntax should pass through
        let result = ctx.interpolate("price is $100 and {name}");
        assert_eq!(result.unwrap(), "price is $100 and {name}");
    }

    #[test]
    fn test_steps_scope_deep_nesting() {
        let mut ctx = VarContext::new();
        ctx.set("steps.build.output", "success");
        ctx.set("steps.inspect.verdict", "approved");
        ctx.set("steps.gates.failures", "none");

        let result = ctx
            .interpolate(
                "Build: ${steps.build.output}, Inspect: ${steps.inspect.verdict}, Gates: ${steps.gates.failures}",
            )
            .unwrap();
        assert_eq!(
            result,
            "Build: success, Inspect: approved, Gates: none"
        );
    }

    #[test]
    fn test_manifest_scope() {
        let mut ctx = VarContext::new();
        ctx.set("manifest.summary", "3 files changed");
        ctx.set("manifest.id", "run-001");

        let result = ctx
            .interpolate("Manifest ${manifest.id}: ${manifest.summary}")
            .unwrap();
        assert_eq!(result, "Manifest run-001: 3 files changed");
    }

    #[test]
    fn test_mixed_set_and_unset_variables() {
        let mut ctx = VarContext::new();
        ctx.set("config.name", "factory");
        // spec.path is not set
        let result = ctx.interpolate("${config.name} at ${spec.path}");
        assert!(result.is_err());
    }

    #[test]
    fn test_var_error_display() {
        let err = VarError::UnsetVariables(vec!["config.x".to_string(), "spec.y".to_string()]);
        let msg = format!("{}", err);
        assert!(msg.contains("config.x"));
        assert!(msg.contains("spec.y"));
    }

    #[test]
    fn test_empty_string_interpolation() {
        let ctx = VarContext::new();
        let result = ctx.interpolate("").unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_underscored_variable_names() {
        let mut ctx = VarContext::new();
        ctx.set("config.max_rework", "3");
        ctx.set("config.output_format", "json");
        let result = ctx
            .interpolate("${config.max_rework} ${config.output_format}")
            .unwrap();
        assert_eq!(result, "3 json");
    }
}
