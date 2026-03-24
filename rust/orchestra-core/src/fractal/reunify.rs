//! Reunify merge — algorithmic code reunification.
//!
//! Algorithms used:
//! - git merge-tree for 3-way merge (MergeSort merge step analog)
//! - Set intersection for boundary violation detection
//! - Set difference for interface gap detection

use std::collections::HashMap;
use std::process::Command;

use serde::{Deserialize, Serialize};

use super::extract_terms;

/// Input for reunification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReunifyInput {
    #[serde(default = "default_base")]
    pub base_ref: String,
    pub children: Vec<ReunifyChild>,
    #[serde(default)]
    pub dependency_order: Vec<Vec<String>>,
    #[serde(default)]
    pub node_slug: String,
}

fn default_base() -> String {
    "main".to_string()
}

/// A solved child with its branch and file info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReunifyChild {
    pub slug: String,
    #[serde(default)]
    pub branch: String,
    #[serde(default)]
    pub scope: String,
    #[serde(default)]
    pub boundaries: String,
    #[serde(default)]
    pub inputs: String,
    #[serde(default)]
    pub outputs: String,
    #[serde(default)]
    pub files: Vec<String>,
}

/// A detected conflict.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conflict {
    pub category: String,
    pub children: Vec<String>,
    pub description: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub files: Vec<String>,
}

/// An auto-resolved item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoResolved {
    pub description: String,
    pub child: String,
    pub resolution: String,
}

/// Output of reunification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReunifyOutput {
    pub status: String,
    pub auto_resolved: Vec<AutoResolved>,
    pub conflicts: Vec<Conflict>,
    pub merge_order: Vec<String>,
    pub needs_ai: bool,
}

// ---------------------------------------------------------------------------
// Structural Conflict Detection (pure set operations, no AI)
// ---------------------------------------------------------------------------

/// Detect boundary violations: child modified files outside its scope.
fn check_scope_violations(children: &[ReunifyChild]) -> Vec<Conflict> {
    let mut conflicts = Vec::new();

    for child in children {
        let child_files: std::collections::HashSet<&String> = child.files.iter().collect();

        for sibling in children {
            if sibling.slug == child.slug {
                continue;
            }
            let sibling_files: std::collections::HashSet<&String> =
                sibling.files.iter().collect();
            let overlap: Vec<String> = child_files
                .intersection(&sibling_files)
                .map(|f| f.to_string())
                .collect();

            if !overlap.is_empty() {
                // Only flag once per pair (alphabetical order)
                if child.slug < sibling.slug {
                    conflicts.push(Conflict {
                        category: "boundary".to_string(),
                        children: vec![child.slug.clone(), sibling.slug.clone()],
                        description: format!(
                            "Children '{}' and '{}' both modified: {}",
                            child.slug,
                            sibling.slug,
                            overlap.join(", "),
                        ),
                        files: overlap,
                    });
                }
            }
        }
    }
    conflicts
}

/// Detect redundancy: multiple children modified the same file.
fn check_redundant_files(children: &[ReunifyChild]) -> Vec<Conflict> {
    let mut file_owners: HashMap<&String, Vec<String>> = HashMap::new();

    for child in children {
        for f in &child.files {
            file_owners
                .entry(f)
                .or_default()
                .push(child.slug.clone());
        }
    }

    file_owners
        .into_iter()
        .filter(|(_, owners)| owners.len() > 1)
        .map(|(file, mut owners)| {
            owners.sort();
            Conflict {
                category: "redundancy".to_string(),
                description: format!(
                    "File '{}' was modified by multiple children: {}",
                    file,
                    owners.join(", "),
                ),
                children: owners,
                files: vec![file.clone()],
            }
        })
        .collect()
}

/// Detect gaps: declared inputs not satisfied by any sibling's outputs.
fn check_interface_gaps(children: &[ReunifyChild]) -> Vec<Conflict> {
    let mut all_outputs = std::collections::HashSet::new();
    for c in children {
        all_outputs.extend(extract_terms(&c.outputs));
    }

    let mut gaps = Vec::new();
    for c in children {
        let inputs = extract_terms(&c.inputs);
        if inputs.is_empty() {
            continue;
        }
        let missing: Vec<String> = inputs
            .difference(&all_outputs)
            .take(5)
            .cloned()
            .collect();
        if !missing.is_empty() {
            gaps.push(Conflict {
                category: "gap".to_string(),
                children: vec![c.slug.clone()],
                description: format!(
                    "Child '{}' declares inputs not produced by any sibling: {}",
                    c.slug,
                    missing.join(", "),
                ),
                files: Vec::new(),
            });
        }
    }
    gaps
}

// ---------------------------------------------------------------------------
// Git Merge-Tree (3-way merge algorithm)
// ---------------------------------------------------------------------------

/// Result of a git merge-tree check.
struct MergeTreeResult {
    clean: bool,
    conflicts: Vec<Conflict>,
}

/// Run git merge-tree to test a 3-way merge without touching the working tree.
fn try_merge_tree(base_ref: &str, child: &ReunifyChild) -> Option<MergeTreeResult> {
    if child.branch.is_empty() {
        return None;
    }

    // Find merge base
    let merge_base = Command::new("git")
        .args(["merge-base", base_ref, &child.branch])
        .output()
        .ok()?;

    if !merge_base.status.success() {
        return None;
    }

    let base = String::from_utf8_lossy(&merge_base.stdout)
        .trim()
        .to_string();

    // Try merge-tree
    let result = Command::new("git")
        .args(["merge-tree", "--write-tree", &base, base_ref, &child.branch])
        .output()
        .ok()?;

    if result.status.success() {
        return Some(MergeTreeResult {
            clean: true,
            conflicts: Vec::new(),
        });
    }

    // Parse conflicts from output
    let output = String::from_utf8_lossy(&result.stdout);
    let stderr = String::from_utf8_lossy(&result.stderr);
    let all_output = format!("{}\n{}", output, stderr);

    let conflicts: Vec<Conflict> = all_output
        .lines()
        .filter(|l| l.contains("CONFLICT"))
        .map(|line| {
            let conflict_type = if line.contains("rename/rename") {
                "rename"
            } else if line.contains("modify/delete") {
                "modify_delete"
            } else if line.contains("add/add") {
                "add_add"
            } else {
                "content"
            };
            Conflict {
                category: format!("merge_{}", conflict_type),
                children: vec![child.slug.clone()],
                description: line.trim().to_string(),
                files: Vec::new(),
            }
        })
        .collect();

    Some(MergeTreeResult {
        clean: false,
        conflicts,
    })
}

/// Classify whether a merge conflict needs AI to resolve.
fn needs_ai_resolution(conflict: &Conflict) -> bool {
    // Rename and modify/delete can often be resolved mechanically
    matches!(
        conflict.category.as_str(),
        "merge_content" | "merge_add_add" | "boundary" | "gap"
    )
}

// ---------------------------------------------------------------------------
// Flatten dependency order into merge sequence
// ---------------------------------------------------------------------------

fn flatten_waves(waves: &[Vec<String>], children: &[ReunifyChild]) -> Vec<String> {
    if waves.is_empty() {
        let mut order: Vec<String> = children.iter().map(|c| c.slug.clone()).collect();
        order.sort();
        return order;
    }
    waves.iter().flat_map(|w| w.iter().cloned()).collect()
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Run reunification analysis. Structural checks are pure; git checks need
/// a git repo (will be skipped if branches don't exist).
pub fn run(input: &ReunifyInput) -> ReunifyOutput {
    let merge_order = flatten_waves(&input.dependency_order, &input.children);

    // Structural checks (pure, no git needed)
    let mut all_conflicts = Vec::new();
    all_conflicts.extend(check_scope_violations(&input.children));
    all_conflicts.extend(check_redundant_files(&input.children));
    all_conflicts.extend(check_interface_gaps(&input.children));

    // Git merge-tree checks
    let mut auto_resolved = Vec::new();
    let children_by_slug: HashMap<&str, &ReunifyChild> = input
        .children
        .iter()
        .map(|c| (c.slug.as_str(), c))
        .collect();

    for slug in &merge_order {
        if let Some(child) = children_by_slug.get(slug.as_str()) {
            if let Some(merge_result) = try_merge_tree(&input.base_ref, child) {
                if !merge_result.clean {
                    for conflict in merge_result.conflicts {
                        if needs_ai_resolution(&conflict) {
                            all_conflicts.push(conflict);
                        } else {
                            auto_resolved.push(AutoResolved {
                                description: conflict.description,
                                child: slug.clone(),
                                resolution: "auto".to_string(),
                            });
                        }
                    }
                }
            }
        }
    }

    let needs_ai = all_conflicts.iter().any(|c| needs_ai_resolution(c));
    let status = if all_conflicts.is_empty() {
        "MERGED"
    } else if needs_ai {
        "CONFLICT"
    } else {
        "PARTIAL"
    };

    ReunifyOutput {
        status: status.to_string(),
        auto_resolved,
        conflicts: all_conflicts,
        merge_order,
        needs_ai,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_child(slug: &str, files: Vec<&str>) -> ReunifyChild {
        ReunifyChild {
            slug: slug.to_string(),
            branch: String::new(),
            scope: String::new(),
            boundaries: String::new(),
            inputs: String::new(),
            outputs: String::new(),
            files: files.into_iter().map(String::from).collect(),
        }
    }

    #[test]
    fn test_no_conflicts() {
        let input = ReunifyInput {
            base_ref: "main".to_string(),
            children: vec![
                make_child("auth", vec!["src/auth.rs"]),
                make_child("data", vec!["src/data.rs"]),
            ],
            dependency_order: Vec::new(),
            node_slug: "root".to_string(),
        };
        let output = run(&input);
        assert_eq!(output.status, "MERGED");
        assert!(output.conflicts.is_empty());
    }

    #[test]
    fn test_file_overlap_detected() {
        let input = ReunifyInput {
            base_ref: "main".to_string(),
            children: vec![
                make_child("auth", vec!["src/auth.rs", "src/shared.rs"]),
                make_child("data", vec!["src/data.rs", "src/shared.rs"]),
            ],
            dependency_order: Vec::new(),
            node_slug: "root".to_string(),
        };
        let output = run(&input);
        assert!(!output.conflicts.is_empty());

        let boundary = output
            .conflicts
            .iter()
            .find(|c| c.category == "boundary");
        assert!(boundary.is_some(), "should detect boundary violation");
    }

    #[test]
    fn test_interface_gap_detected() {
        let input = ReunifyInput {
            base_ref: "main".to_string(),
            children: vec![
                ReunifyChild {
                    slug: "api".to_string(),
                    branch: String::new(),
                    scope: String::new(),
                    boundaries: String::new(),
                    inputs: "auth-tokens database-models".to_string(),
                    outputs: "endpoints".to_string(),
                    files: vec!["src/api.rs".to_string()],
                },
                ReunifyChild {
                    slug: "auth".to_string(),
                    branch: String::new(),
                    scope: String::new(),
                    boundaries: String::new(),
                    inputs: String::new(),
                    outputs: "auth-tokens".to_string(),
                    files: vec!["src/auth.rs".to_string()],
                },
            ],
            dependency_order: Vec::new(),
            node_slug: "root".to_string(),
        };
        let output = run(&input);
        let gap = output.conflicts.iter().find(|c| c.category == "gap");
        assert!(gap.is_some(), "should detect missing database-models input");
    }

    #[test]
    fn test_merge_order_follows_waves() {
        let input = ReunifyInput {
            base_ref: "main".to_string(),
            children: vec![
                make_child("z-last", vec!["src/z.rs"]),
                make_child("a-first", vec!["src/a.rs"]),
            ],
            dependency_order: vec![
                vec!["a-first".to_string()],
                vec!["z-last".to_string()],
            ],
            node_slug: "root".to_string(),
        };
        let output = run(&input);
        assert_eq!(output.merge_order, vec!["a-first", "z-last"]);
    }

    // -----------------------------------------------------------------------
    // Spec 064: Additional reunification tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_redundancy_conflict_detected() {
        let input = ReunifyInput {
            base_ref: "main".to_string(),
            children: vec![
                make_child("auth", vec!["src/shared.rs", "src/auth.rs"]),
                make_child("api", vec!["src/shared.rs", "src/api.rs"]),
            ],
            dependency_order: Vec::new(),
            node_slug: "root".to_string(),
        };
        let output = run(&input);
        let redundancy = output.conflicts.iter().find(|c| c.category == "redundancy");
        assert!(redundancy.is_some(), "should detect redundant file modification");
        assert!(redundancy.unwrap().description.contains("shared.rs"));
    }

    #[test]
    fn test_clean_merge_status() {
        let input = ReunifyInput {
            base_ref: "main".to_string(),
            children: vec![
                make_child("auth", vec!["src/auth.rs"]),
                make_child("data", vec!["src/data.rs"]),
                make_child("api", vec!["src/api.rs"]),
            ],
            dependency_order: Vec::new(),
            node_slug: "root".to_string(),
        };
        let output = run(&input);
        assert_eq!(output.status, "MERGED");
        assert!(!output.needs_ai);
    }

    #[test]
    fn test_interface_gap_needs_ai() {
        let input = ReunifyInput {
            base_ref: "main".to_string(),
            children: vec![
                ReunifyChild {
                    slug: "consumer".to_string(),
                    branch: String::new(),
                    scope: String::new(),
                    boundaries: String::new(),
                    inputs: "missing-dependency".to_string(),
                    outputs: "result".to_string(),
                    files: vec!["src/consumer.rs".to_string()],
                },
            ],
            dependency_order: Vec::new(),
            node_slug: "root".to_string(),
        };
        let output = run(&input);
        assert!(output.needs_ai, "gap conflicts need AI resolution");
    }

    #[test]
    fn test_empty_children_clean_merge() {
        let input = ReunifyInput {
            base_ref: "main".to_string(),
            children: vec![],
            dependency_order: Vec::new(),
            node_slug: "root".to_string(),
        };
        let output = run(&input);
        assert_eq!(output.status, "MERGED");
        assert!(output.conflicts.is_empty());
    }

    #[test]
    fn test_merge_order_without_waves_alphabetical() {
        let input = ReunifyInput {
            base_ref: "main".to_string(),
            children: vec![
                make_child("zebra", vec!["z.rs"]),
                make_child("alpha", vec!["a.rs"]),
                make_child("middle", vec!["m.rs"]),
            ],
            dependency_order: vec![], // empty → alphabetical
            node_slug: "root".to_string(),
        };
        let output = run(&input);
        assert_eq!(output.merge_order, vec!["alpha", "middle", "zebra"]);
    }

    #[test]
    fn test_multiple_conflict_types_aggregated() {
        let input = ReunifyInput {
            base_ref: "main".to_string(),
            children: vec![
                ReunifyChild {
                    slug: "auth".to_string(),
                    branch: String::new(),
                    scope: String::new(),
                    boundaries: String::new(),
                    inputs: "database-models".to_string(), // gap: not produced
                    outputs: "tokens".to_string(),
                    files: vec!["src/shared.rs".to_string(), "src/auth.rs".to_string()],
                },
                ReunifyChild {
                    slug: "api".to_string(),
                    branch: String::new(),
                    scope: String::new(),
                    boundaries: String::new(),
                    inputs: String::new(),
                    outputs: "endpoints".to_string(),
                    files: vec!["src/shared.rs".to_string(), "src/api.rs".to_string()],
                },
            ],
            dependency_order: Vec::new(),
            node_slug: "root".to_string(),
        };
        let output = run(&input);
        // Should have both boundary and gap conflicts
        let categories: Vec<&str> = output.conflicts.iter().map(|c| c.category.as_str()).collect();
        assert!(categories.contains(&"boundary"), "should detect boundary violation");
        assert!(categories.contains(&"gap"), "should detect interface gap");
    }
}
