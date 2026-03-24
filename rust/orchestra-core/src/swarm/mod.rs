pub mod checkpoint;
pub mod prune;

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

// ---------------------------------------------------------------------------
// Shared types for the swarm algorithmic spine
// ---------------------------------------------------------------------------

/// A swarm branch in the manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmBranch {
    pub id: String,
    #[serde(default)]
    pub strategy: String,
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default)]
    pub status: String,
}

/// Swarm manifest (subset needed by algorithmic spine).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmManifest {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub branches: Vec<SwarmBranch>,
}

/// Jaccard similarity between two file sets.
pub fn jaccard_similarity(a: &[String], b: &[String]) -> f64 {
    let set_a: HashSet<&str> = a.iter().map(|s| s.as_str()).collect();
    let set_b: HashSet<&str> = b.iter().map(|s| s.as_str()).collect();
    if set_a.is_empty() && set_b.is_empty() {
        return 0.0;
    }
    let intersection = set_a.intersection(&set_b).count();
    let union = set_a.union(&set_b).count();
    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jaccard_identical() {
        let a = vec!["src/main.rs".to_string(), "src/lib.rs".to_string()];
        let b = vec!["src/main.rs".to_string(), "src/lib.rs".to_string()];
        assert!((jaccard_similarity(&a, &b) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_jaccard_disjoint() {
        let a = vec!["src/a.rs".to_string()];
        let b = vec!["src/b.rs".to_string()];
        assert!((jaccard_similarity(&a, &b)).abs() < 1e-10);
    }

    #[test]
    fn test_jaccard_partial() {
        let a = vec!["a.rs".to_string(), "b.rs".to_string()];
        let b = vec!["b.rs".to_string(), "c.rs".to_string()];
        // intersection=1, union=3 → 0.333...
        assert!((jaccard_similarity(&a, &b) - 1.0 / 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_jaccard_empty() {
        let a: Vec<String> = vec![];
        let b: Vec<String> = vec![];
        assert_eq!(jaccard_similarity(&a, &b), 0.0);
    }
}
