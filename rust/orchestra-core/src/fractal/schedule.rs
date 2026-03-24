//! Solve scheduler — DAG-based critical path scheduling.
//!
//! Algorithms used:
//! - BFS layer decomposition for parallel wave scheduling (MapReduce shuffle analog)
//! - Longest-path dynamic programming for critical path analysis

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use super::{extract_terms, Manifest, TreeNode};

/// Output of the solve scheduler.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleOutput {
    /// Groups of leaf slugs that can execute concurrently.
    pub waves: Vec<Vec<String>>,
    /// Longest dependency chain — determines minimum sequential waves.
    pub critical_path: Vec<String>,
    /// Number of waves (= critical path length).
    pub critical_path_length: usize,
    /// Peak concurrent leaves in any single wave.
    pub max_parallelism: usize,
    /// Total number of leaves scheduled.
    pub total_leaves: usize,
}

/// Collect all leaf nodes from a manifest tree.
fn collect_leaves(tree: &HashMap<String, TreeNode>) -> Vec<TreeNode> {
    tree.values()
        .filter(|node| {
            matches!(
                node.status.as_str(),
                "leaf" | "forced-leaf" | "pending"
            ) && node.children.is_empty()
        })
        .cloned()
        .collect()
}

/// Build dependency graph: slug → set of slugs it depends on.
fn build_dep_graph(leaves: &[TreeNode]) -> HashMap<String, HashSet<String>> {
    let slugs: HashSet<String> = leaves.iter().map(|l| l.slug.clone()).collect();

    // Map output terms to producing leaf
    let mut output_map: HashMap<String, String> = HashMap::new();
    for leaf in leaves {
        for term in extract_terms(&leaf.outputs) {
            output_map.insert(term, leaf.slug.clone());
        }
    }

    let mut deps: HashMap<String, HashSet<String>> = HashMap::new();
    for leaf in leaves {
        let entry = deps.entry(leaf.slug.clone()).or_default();
        for term in extract_terms(&leaf.inputs) {
            if let Some(producer) = output_map.get(&term) {
                if producer != &leaf.slug && slugs.contains(producer) {
                    entry.insert(producer.clone());
                }
            }
        }
    }

    deps
}

/// Topological sort into parallel execution waves.
fn compute_waves(deps: &HashMap<String, HashSet<String>>) -> Vec<Vec<String>> {
    let all_nodes: HashSet<String> = deps.keys().cloned().collect();
    let mut resolved: HashSet<String> = HashSet::new();
    let mut waves: Vec<Vec<String>> = Vec::new();

    while resolved.len() < all_nodes.len() {
        let mut wave: Vec<String> = all_nodes
            .iter()
            .filter(|n| {
                !resolved.contains(n.as_str())
                    && deps
                        .get(n.as_str())
                        .map_or(true, |d| d.iter().all(|dep| resolved.contains(dep)))
            })
            .cloned()
            .collect();

        if wave.is_empty() {
            let mut remaining: Vec<String> = all_nodes
                .iter()
                .filter(|n| !resolved.contains(n.as_str()))
                .cloned()
                .collect();
            remaining.sort();
            waves.push(remaining);
            break;
        }

        wave.sort();
        resolved.extend(wave.iter().cloned());
        waves.push(wave);
    }

    waves
}

/// Find the critical path (longest dependency chain) using DP.
fn compute_critical_path(
    deps: &HashMap<String, HashSet<String>>,
    waves: &[Vec<String>],
) -> Vec<String> {
    if waves.is_empty() {
        return Vec::new();
    }

    // longest[n] = length of longest path ending at n
    let mut longest: HashMap<&str, usize> = HashMap::new();
    let mut predecessor: HashMap<&str, Option<&str>> = HashMap::new();

    for slug in deps.keys() {
        longest.insert(slug.as_str(), 1);
        predecessor.insert(slug.as_str(), None);
    }

    // Process in wave order (topological order)
    for wave in waves {
        for node in wave {
            // Check all nodes that `node` depends on
            if let Some(node_deps) = deps.get(node) {
                for dep in node_deps {
                    let dep_len = longest.get(dep.as_str()).copied().unwrap_or(1);
                    let current = longest.get(node.as_str()).copied().unwrap_or(1);
                    if dep_len + 1 > current {
                        longest.insert(node.as_str(), dep_len + 1);
                        predecessor.insert(node.as_str(), Some(dep.as_str()));
                    }
                }
            }
        }
    }

    // Trace back from node with longest path
    if longest.is_empty() {
        return Vec::new();
    }

    let end_node = longest
        .iter()
        .max_by_key(|(_, &len)| len)
        .map(|(&node, _)| node)
        .unwrap();

    let mut path = Vec::new();
    let mut current: Option<&str> = Some(end_node);
    while let Some(node) = current {
        path.push(node.to_string());
        current = predecessor.get(node).copied().flatten();
    }
    path.reverse();
    path
}

/// Run the solve scheduler on a manifest. Pure function — no I/O.
pub fn run(manifest: &Manifest) -> ScheduleOutput {
    let leaves = collect_leaves(&manifest.tree);

    if leaves.is_empty() {
        return ScheduleOutput {
            waves: Vec::new(),
            critical_path: Vec::new(),
            critical_path_length: 0,
            max_parallelism: 0,
            total_leaves: 0,
        };
    }

    let deps = build_dep_graph(&leaves);
    let waves = compute_waves(&deps);
    let critical_path = compute_critical_path(&deps, &waves);

    ScheduleOutput {
        critical_path_length: waves.len(),
        max_parallelism: waves.iter().map(|w| w.len()).max().unwrap_or(0),
        total_leaves: leaves.len(),
        critical_path,
        waves,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_leaf(slug: &str, inputs: &str, outputs: &str) -> TreeNode {
        TreeNode {
            slug: slug.to_string(),
            depth: 1,
            status: "leaf".to_string(),
            scope: String::new(),
            boundaries: String::new(),
            inputs: inputs.to_string(),
            outputs: outputs.to_string(),
            children: Vec::new(),
            files: Vec::new(),
            branch: String::new(),
        }
    }

    fn make_manifest(nodes: Vec<TreeNode>) -> Manifest {
        let tree: HashMap<String, TreeNode> = nodes
            .into_iter()
            .map(|n| (n.slug.clone(), n))
            .collect();
        Manifest {
            id: "test".to_string(),
            status: "solving".to_string(),
            tree,
        }
    }

    #[test]
    fn test_all_independent() {
        let manifest = make_manifest(vec![
            make_leaf("auth", "none", "tokens"),
            make_leaf("data", "none", "models"),
            make_leaf("cache", "none", "cache-layer"),
        ]);
        let output = run(&manifest);
        assert_eq!(output.waves.len(), 1, "all independent → single wave");
        assert_eq!(output.max_parallelism, 3);
        assert_eq!(output.total_leaves, 3);
    }

    #[test]
    fn test_linear_chain() {
        let manifest = make_manifest(vec![
            make_leaf("auth", "none", "tokens sessions"),
            make_leaf("api", "tokens", "endpoints"),
            make_leaf("monitoring", "endpoints", "dashboards"),
        ]);
        let output = run(&manifest);
        assert_eq!(output.waves.len(), 3, "linear chain → 3 waves");
        assert_eq!(output.max_parallelism, 1);
        assert_eq!(output.critical_path.len(), 3);
    }

    #[test]
    fn test_diamond_dependency() {
        // auth → api, auth → cache, api+cache → gateway
        let manifest = make_manifest(vec![
            make_leaf("auth", "none", "tokens"),
            make_leaf("api", "tokens", "endpoints"),
            make_leaf("cache", "tokens", "cached-data"),
            make_leaf("gateway", "endpoints cached-data", "gateway-routes"),
        ]);
        let output = run(&manifest);
        // Wave 1: [auth], Wave 2: [api, cache], Wave 3: [gateway]
        assert_eq!(output.waves.len(), 3);
        assert_eq!(output.waves[0], vec!["auth"]);
        assert!(output.waves[1].contains(&"api".to_string()));
        assert!(output.waves[1].contains(&"cache".to_string()));
        assert_eq!(output.waves[2], vec!["gateway"]);
        assert_eq!(output.max_parallelism, 2);
    }

    #[test]
    fn test_empty_manifest() {
        let manifest = make_manifest(Vec::new());
        let output = run(&manifest);
        assert!(output.waves.is_empty());
        assert_eq!(output.total_leaves, 0);
    }

    // -----------------------------------------------------------------------
    // Spec 064: Additional scheduling tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_critical_path_length_equals_waves() {
        let manifest = make_manifest(vec![
            make_leaf("auth", "none", "tokens sessions"),
            make_leaf("api", "tokens", "endpoints"),
            make_leaf("monitoring", "endpoints", "dashboards"),
        ]);
        let output = run(&manifest);
        assert_eq!(output.critical_path_length, output.waves.len());
    }

    #[test]
    fn test_max_parallelism_diamond() {
        // a → (b, c) → d
        let manifest = make_manifest(vec![
            make_leaf("a", "none", "tokens"),
            make_leaf("b", "tokens", "endpoints"),
            make_leaf("c", "tokens", "cached-data"),
            make_leaf("d", "endpoints cached-data", "routes"),
        ]);
        let output = run(&manifest);
        assert_eq!(output.max_parallelism, 2, "b and c are parallel");
        assert_eq!(output.total_leaves, 4);
    }

    #[test]
    fn test_single_node_schedule() {
        let manifest = make_manifest(vec![make_leaf("only", "none", "result")]);
        let output = run(&manifest);
        assert_eq!(output.waves.len(), 1);
        assert_eq!(output.waves[0], vec!["only"]);
        assert_eq!(output.total_leaves, 1);
        assert_eq!(output.max_parallelism, 1);
    }

    #[test]
    fn test_non_leaf_nodes_excluded() {
        let mut manifest = make_manifest(vec![
            make_leaf("leaf1", "none", "data"),
            make_leaf("leaf2", "none", "more-data"),
        ]);
        // Add a non-leaf node (has children)
        manifest.tree.insert(
            "parent".to_string(),
            TreeNode {
                slug: "parent".to_string(),
                depth: 0,
                status: "decomposed".to_string(),
                scope: String::new(),
                boundaries: String::new(),
                inputs: String::new(),
                outputs: String::new(),
                children: vec!["leaf1".to_string(), "leaf2".to_string()],
                files: Vec::new(),
                branch: String::new(),
            },
        );
        let output = run(&manifest);
        // Only leaf nodes should be scheduled
        assert_eq!(output.total_leaves, 2);
    }

    #[test]
    fn test_wide_parallel_tree() {
        let manifest = make_manifest(vec![
            make_leaf("a", "none", "out-a"),
            make_leaf("b", "none", "out-b"),
            make_leaf("c", "none", "out-c"),
            make_leaf("d", "none", "out-d"),
            make_leaf("e", "none", "out-e"),
        ]);
        let output = run(&manifest);
        assert_eq!(output.waves.len(), 1, "all independent → 1 wave");
        assert_eq!(output.max_parallelism, 5);
    }
}
