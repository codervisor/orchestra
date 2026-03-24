pub mod decompose;
pub mod prune;
pub mod reunify;
pub mod schedule;

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

// ---------------------------------------------------------------------------
// Shared types for the fractal algorithmic spine
// ---------------------------------------------------------------------------

/// A child node produced by decomposition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Child {
    pub slug: String,
    #[serde(default)]
    pub scope: String,
    #[serde(default)]
    pub boundaries: String,
    #[serde(default)]
    pub inputs: String,
    #[serde(default)]
    pub outputs: String,
}

/// Input to the decompose gate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecomposeInput {
    pub parent_spec: String,
    pub children: Vec<Child>,
    #[serde(default = "default_depth")]
    pub current_depth: usize,
    #[serde(default = "default_max_depth")]
    pub max_depth: usize,
    #[serde(default)]
    pub total_nodes: usize,
    #[serde(default = "default_max_nodes")]
    pub max_total_nodes: usize,
}

fn default_depth() -> usize {
    0
}
fn default_max_depth() -> usize {
    3
}
fn default_max_nodes() -> usize {
    20
}

/// An advisory flag from structural validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Flag {
    pub category: String,
    pub description: String,
}

/// Full output of the decompose gate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecomposeOutput {
    pub flags: Vec<Flag>,
    pub complexity_score: f64,
    pub budget_allocation: HashMap<String, usize>,
    pub dependency_order: Vec<Vec<String>>,
}

/// A node in the fractal manifest tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeNode {
    pub slug: String,
    #[serde(default)]
    pub depth: usize,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub scope: String,
    #[serde(default)]
    pub boundaries: String,
    #[serde(default)]
    pub inputs: String,
    #[serde(default)]
    pub outputs: String,
    #[serde(default)]
    pub children: Vec<String>,
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default)]
    pub branch: String,
}

/// Fractal manifest (subset needed by algorithmic spine).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub tree: HashMap<String, TreeNode>,
}

// ---------------------------------------------------------------------------
// Shared NLP utilities
// ---------------------------------------------------------------------------

/// Stop words filtered during term extraction.
const STOP_WORDS: &[&str] = &[
    "the", "and", "for", "that", "this", "with", "from", "are", "was", "were",
    "been", "have", "has", "had", "not", "but", "its", "can", "will", "should",
    "must", "may", "each", "all", "any", "into", "when", "how", "what", "which",
    "their", "them", "they", "you", "your", "about", "also", "does", "using",
    "used", "use", "none",
];

/// Extract lowercase alphanumeric terms (>=3 chars), filtering stop words.
pub fn extract_terms(text: &str) -> HashSet<String> {
    let stop: HashSet<&str> = STOP_WORDS.iter().copied().collect();
    let re = regex::Regex::new(r"[a-zA-Z][a-zA-Z0-9_-]{2,}").unwrap();
    re.find_iter(text)
        .map(|m| m.as_str().to_lowercase())
        .filter(|w| !stop.contains(w.as_str()))
        .collect()
}

/// Extract terms preserving duplicates (for TF calculation).
pub fn extract_term_list(text: &str) -> Vec<String> {
    let stop: HashSet<&str> = STOP_WORDS.iter().copied().collect();
    let re = regex::Regex::new(r"[a-zA-Z][a-zA-Z0-9_-]{2,}").unwrap();
    re.find_iter(text)
        .map(|m| m.as_str().to_lowercase())
        .filter(|w| !stop.contains(w.as_str()))
        .collect()
}

/// Jaccard similarity between two term sets.
pub fn jaccard_similarity(a: &HashSet<String>, b: &HashSet<String>) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 0.0;
    }
    let intersection = a.intersection(b).count();
    let union = a.union(b).count();
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
    fn test_extract_terms() {
        let terms = extract_terms("Build an authentication system with OAuth2");
        assert!(terms.contains("build"));
        assert!(terms.contains("authentication"));
        assert!(terms.contains("system"));
        assert!(terms.contains("oauth2"));
        // Stop words filtered
        assert!(!terms.contains("with"));
        // Short words filtered
        assert!(!terms.contains("an"));
    }

    #[test]
    fn test_jaccard_similarity() {
        let a: HashSet<String> = ["auth", "system", "oauth"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let b: HashSet<String> = ["auth", "system", "data"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let sim = jaccard_similarity(&a, &b);
        // intersection=2, union=4 → 0.5
        assert!((sim - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_jaccard_empty() {
        let a: HashSet<String> = HashSet::new();
        let b: HashSet<String> = HashSet::new();
        assert_eq!(jaccard_similarity(&a, &b), 0.0);
    }

    #[test]
    fn test_jaccard_identical() {
        let a: HashSet<String> = ["auth", "system"].iter().map(|s| s.to_string()).collect();
        let b = a.clone();
        assert!((jaccard_similarity(&a, &b) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_extract_terms_stop_words_filtered() {
        let terms = extract_terms("the system should have this feature");
        assert!(!terms.contains("the"));
        assert!(!terms.contains("should"));
        assert!(!terms.contains("have"));
        assert!(terms.contains("system"));
        assert!(terms.contains("feature"));
    }

    #[test]
    fn test_extract_terms_short_words_filtered() {
        let terms = extract_terms("go do it on my way");
        // All words are < 3 chars or stop words
        assert!(terms.is_empty() || terms.iter().all(|t| t.len() >= 3));
    }

    #[test]
    fn test_extract_term_list_preserves_duplicates() {
        let terms = extract_term_list("auth auth auth system system");
        let auth_count = terms.iter().filter(|t| *t == "auth").count();
        assert_eq!(auth_count, 3, "term list should preserve duplicate counts");
    }

    #[test]
    fn test_extract_terms_hyphenated() {
        let terms = extract_terms("error-handling and cross-cutting concerns");
        assert!(terms.contains("error-handling"));
        assert!(terms.contains("cross-cutting"));
        assert!(terms.contains("concerns"));
    }

    #[test]
    fn test_child_serialization_roundtrip() {
        let child = Child {
            slug: "auth".to_string(),
            scope: "authentication system".to_string(),
            boundaries: "no external calls".to_string(),
            inputs: "user credentials".to_string(),
            outputs: "auth tokens".to_string(),
        };
        let json = serde_json::to_string(&child).unwrap();
        let back: Child = serde_json::from_str(&json).unwrap();
        assert_eq!(back.slug, "auth");
        assert_eq!(back.scope, "authentication system");
    }

    #[test]
    fn test_manifest_serialization_roundtrip() {
        let mut tree = HashMap::new();
        tree.insert(
            "root".to_string(),
            TreeNode {
                slug: "root".to_string(),
                depth: 0,
                status: "decomposed".to_string(),
                scope: "full system".to_string(),
                boundaries: String::new(),
                inputs: String::new(),
                outputs: String::new(),
                children: vec!["child-a".to_string()],
                files: Vec::new(),
                branch: String::new(),
            },
        );
        let manifest = Manifest {
            id: "test-manifest".to_string(),
            status: "solving".to_string(),
            tree,
        };
        let json = serde_json::to_string(&manifest).unwrap();
        let back: Manifest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "test-manifest");
        assert_eq!(back.tree.len(), 1);
    }
}
