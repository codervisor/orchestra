//! Prune gate — algorithmic redundancy detection.
//!
//! Algorithms used:
//! - Set subset analysis for output containment detection
//! - Greedy set cover for minimal covering set (NP-hard approximation)
//! - Identity detection for duplicate outputs

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use super::TreeNode;

/// Output of the prune gate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PruneOutput {
    /// Nodes recommended for pruning.
    pub prunable: Vec<String>,
    /// Reason each node is prunable.
    pub reasons: HashMap<String, String>,
    /// Nodes to keep.
    pub kept: Vec<String>,
    /// Which nodes touched each file.
    pub file_coverage: HashMap<String, Vec<String>>,
    /// Pairs of nodes with identical file sets.
    pub identical_pairs: Vec<(String, String)>,
    /// Minimal set of nodes that covers all files.
    pub minimal_covering_set: Vec<String>,
}

/// Find nodes whose output files are a strict subset of a sibling's.
fn detect_subset_outputs(nodes: &[&TreeNode]) -> HashMap<String, String> {
    let mut prunable = HashMap::new();

    for (i, node_a) in nodes.iter().enumerate() {
        let files_a: HashSet<&String> = node_a.files.iter().collect();

        if files_a.is_empty() {
            prunable.insert(
                node_a.slug.clone(),
                "produced no file changes".to_string(),
            );
            continue;
        }

        for (j, node_b) in nodes.iter().enumerate() {
            if i == j {
                continue;
            }
            let files_b: HashSet<&String> = node_b.files.iter().collect();
            // Strict subset: A ⊂ B (A is subset and A ≠ B)
            if files_a.is_subset(&files_b) && files_a != files_b {
                prunable.insert(
                    node_a.slug.clone(),
                    format!(
                        "output files are a strict subset of sibling '{}' ({} vs {} files)",
                        node_b.slug,
                        files_a.len(),
                        files_b.len(),
                    ),
                );
                break;
            }
        }
    }

    prunable
}

/// Find pairs of nodes with identical file change sets.
fn detect_identical_outputs(nodes: &[&TreeNode]) -> Vec<(String, String)> {
    let mut pairs = Vec::new();

    for i in 0..nodes.len() {
        for j in (i + 1)..nodes.len() {
            let files_a: HashSet<&String> = nodes[i].files.iter().collect();
            let files_b: HashSet<&String> = nodes[j].files.iter().collect();
            if !files_a.is_empty() && files_a == files_b {
                pairs.push((nodes[i].slug.clone(), nodes[j].slug.clone()));
            }
        }
    }

    pairs
}

/// Map each file to which nodes touched it.
fn compute_file_coverage(nodes: &[&TreeNode]) -> HashMap<String, Vec<String>> {
    let mut coverage: HashMap<String, Vec<String>> = HashMap::new();
    for node in nodes {
        for f in &node.files {
            coverage
                .entry(f.clone())
                .or_default()
                .push(node.slug.clone());
        }
    }
    coverage
}

/// Greedy set cover: find minimal set of nodes that covers all files.
///
/// Classic greedy approximation: repeatedly pick the node covering the most
/// uncovered files. Guarantees O(ln n) approximation ratio.
fn minimal_covering_set(nodes: &[&TreeNode]) -> Vec<String> {
    let mut all_files: HashSet<&String> = HashSet::new();
    let mut node_files: HashMap<&str, HashSet<&String>> = HashMap::new();

    for node in nodes {
        let files: HashSet<&String> = node.files.iter().collect();
        all_files.extend(files.iter());
        node_files.insert(&node.slug, files);
    }

    if all_files.is_empty() {
        return Vec::new();
    }

    let mut covered: HashSet<&String> = HashSet::new();
    let mut selected: Vec<String> = Vec::new();
    let mut remaining: HashMap<&str, HashSet<&String>> = node_files;

    while covered.len() < all_files.len() && !remaining.is_empty() {
        // Pick node covering most uncovered files
        let best = remaining
            .iter()
            .max_by_key(|(_, files)| files.difference(&covered).count())
            .map(|(&slug, _)| slug.to_string());

        let best = match best {
            Some(b) => b,
            None => break,
        };

        let new_coverage: HashSet<&String> = remaining
            .get(best.as_str())
            .map(|f| f.difference(&covered).cloned().collect())
            .unwrap_or_default();

        if new_coverage.is_empty() {
            break;
        }

        covered.extend(new_coverage);
        remaining.remove(best.as_str());
        selected.push(best);
    }

    selected
}

/// Run the prune gate. Pure function over the tree.
pub fn run(tree: &HashMap<String, TreeNode>) -> PruneOutput {
    // Collect solved/reunified nodes
    let nodes: Vec<&TreeNode> = tree
        .values()
        .filter(|n| matches!(n.status.as_str(), "solved" | "reunified"))
        .collect();

    if nodes.is_empty() {
        return PruneOutput {
            prunable: Vec::new(),
            reasons: HashMap::new(),
            kept: Vec::new(),
            file_coverage: HashMap::new(),
            identical_pairs: Vec::new(),
            minimal_covering_set: Vec::new(),
        };
    }

    let subset_prunable = detect_subset_outputs(&nodes);
    let identical_pairs = detect_identical_outputs(&nodes);
    let file_coverage = compute_file_coverage(&nodes);
    let covering_set = minimal_covering_set(&nodes);

    // Nodes not in covering set are also prune candidates
    let all_slugs: HashSet<String> = nodes.iter().map(|n| n.slug.clone()).collect();
    let cover_set: HashSet<&String> = covering_set.iter().collect();

    let mut reasons: HashMap<String, String> = subset_prunable;

    for slug in &all_slugs {
        if !cover_set.contains(slug) && !reasons.contains_key(slug) {
            reasons.insert(
                slug.clone(),
                "not in minimal covering set (all files covered by other nodes)".to_string(),
            );
        }
    }

    // For identical pairs: keep the one in the covering set
    for (slug_a, slug_b) in &identical_pairs {
        if !reasons.contains_key(slug_a) && !reasons.contains_key(slug_b) {
            if cover_set.contains(slug_a) {
                reasons.insert(
                    slug_b.clone(),
                    format!("identical file set as '{}' (keeping '{}')", slug_a, slug_a),
                );
            } else {
                reasons.insert(
                    slug_a.clone(),
                    format!("identical file set as '{}' (keeping '{}')", slug_b, slug_b),
                );
            }
        }
    }

    let mut prunable: Vec<String> = reasons.keys().cloned().collect();
    prunable.sort();
    let mut kept: Vec<String> = all_slugs.difference(&prunable.iter().cloned().collect()).cloned().collect();
    kept.sort();

    PruneOutput {
        prunable,
        reasons,
        kept,
        file_coverage,
        identical_pairs,
        minimal_covering_set: covering_set,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_node(slug: &str, files: Vec<&str>) -> TreeNode {
        TreeNode {
            slug: slug.to_string(),
            depth: 1,
            status: "solved".to_string(),
            scope: String::new(),
            boundaries: String::new(),
            inputs: String::new(),
            outputs: String::new(),
            children: Vec::new(),
            files: files.into_iter().map(String::from).collect(),
            branch: String::new(),
        }
    }

    fn make_tree(nodes: Vec<TreeNode>) -> HashMap<String, TreeNode> {
        nodes.into_iter().map(|n| (n.slug.clone(), n)).collect()
    }

    #[test]
    fn test_empty_node_prunable() {
        let tree = make_tree(vec![
            make_node("auth", vec!["src/auth.rs"]),
            make_node("empty", vec![]),
        ]);
        let output = run(&tree);
        assert!(output.prunable.contains(&"empty".to_string()));
        assert!(output.kept.contains(&"auth".to_string()));
    }

    #[test]
    fn test_subset_prunable() {
        let tree = make_tree(vec![
            make_node("small", vec!["src/a.rs"]),
            make_node("big", vec!["src/a.rs", "src/b.rs", "src/c.rs"]),
        ]);
        let output = run(&tree);
        assert!(output.prunable.contains(&"small".to_string()));
        assert!(output.kept.contains(&"big".to_string()));
    }

    #[test]
    fn test_identical_pair() {
        let tree = make_tree(vec![
            make_node("alpha", vec!["src/a.rs", "src/b.rs"]),
            make_node("beta", vec!["src/a.rs", "src/b.rs"]),
        ]);
        let output = run(&tree);
        assert_eq!(output.identical_pairs.len(), 1);
        // One should be prunable
        assert_eq!(output.prunable.len(), 1);
    }

    #[test]
    fn test_no_redundancy() {
        let tree = make_tree(vec![
            make_node("auth", vec!["src/auth.rs"]),
            make_node("data", vec!["src/data.rs"]),
            make_node("api", vec!["src/api.rs"]),
        ]);
        let output = run(&tree);
        assert!(output.prunable.is_empty(), "no redundancy to prune");
        assert_eq!(output.kept.len(), 3);
    }

    #[test]
    fn test_covering_set() {
        let tree = make_tree(vec![
            make_node("big", vec!["src/a.rs", "src/b.rs", "src/c.rs"]),
            make_node("small-a", vec!["src/a.rs"]),
            make_node("small-b", vec!["src/b.rs"]),
        ]);
        let output = run(&tree);
        // "big" alone covers everything
        assert_eq!(output.minimal_covering_set.len(), 1);
        assert!(output.minimal_covering_set.contains(&"big".to_string()));
    }

    // -----------------------------------------------------------------------
    // Spec 064: Additional prune tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_empty_tree_no_pruning() {
        let tree = make_tree(vec![]);
        let output = run(&tree);
        assert!(output.prunable.is_empty());
        assert!(output.kept.is_empty());
    }

    #[test]
    fn test_all_empty_nodes_prunable() {
        let tree = make_tree(vec![
            make_node("a", vec![]),
            make_node("b", vec![]),
        ]);
        let output = run(&tree);
        // Both have no files → both prunable
        assert_eq!(output.prunable.len(), 2);
    }

    #[test]
    fn test_mixed_redundancy_unique() {
        let tree = make_tree(vec![
            make_node("unique-a", vec!["src/a.rs"]),
            make_node("unique-b", vec!["src/b.rs"]),
            make_node("duplicate-of-a", vec!["src/a.rs"]),
            make_node("subset-of-b", vec![]), // empty is subset of everything
        ]);
        let output = run(&tree);
        // subset-of-b (empty) and duplicate-of-a should be prunable
        assert!(output.prunable.contains(&"subset-of-b".to_string()));
        // duplicate-of-a has identical file set as unique-a
        assert_eq!(output.identical_pairs.len(), 1);
    }

    #[test]
    fn test_file_coverage_map() {
        let tree = make_tree(vec![
            make_node("auth", vec!["src/auth.rs", "src/shared.rs"]),
            make_node("api", vec!["src/api.rs", "src/shared.rs"]),
        ]);
        let output = run(&tree);
        let shared_owners = &output.file_coverage["src/shared.rs"];
        assert_eq!(shared_owners.len(), 2);
    }

    #[test]
    fn test_non_solved_nodes_excluded() {
        let mut tree = HashMap::new();
        tree.insert(
            "pending".to_string(),
            TreeNode {
                slug: "pending".to_string(),
                depth: 1,
                status: "pending".to_string(), // not solved
                scope: String::new(),
                boundaries: String::new(),
                inputs: String::new(),
                outputs: String::new(),
                children: Vec::new(),
                files: vec!["src/a.rs".to_string()],
                branch: String::new(),
            },
        );
        tree.insert(
            "solved".to_string(),
            TreeNode {
                slug: "solved".to_string(),
                depth: 1,
                status: "solved".to_string(),
                scope: String::new(),
                boundaries: String::new(),
                inputs: String::new(),
                outputs: String::new(),
                children: Vec::new(),
                files: vec!["src/b.rs".to_string()],
                branch: String::new(),
            },
        );
        let output = run(&tree);
        // Only the solved node should be considered
        assert!(output.kept.contains(&"solved".to_string()));
        assert!(!output.kept.contains(&"pending".to_string()));
        assert!(!output.prunable.contains(&"pending".to_string()));
    }
}
