//! Decompose gate — structural validation for fractal decomposition.
//!
//! Algorithms used:
//! - TF-IDF cosine similarity (via rust-tfidf) for orthogonality checking
//! - Kahn's topological sort for dependency cycle detection
//! - Weighted feature scoring for complexity estimation
//! - Proportional allocation for budget distribution

use std::collections::{HashMap, HashSet, VecDeque};

use tfidf::{TfIdf, TfIdfDefault};

use super::{
    extract_term_list, extract_terms, jaccard_similarity, Child, DecomposeInput, DecomposeOutput,
    Flag,
};

// ---------------------------------------------------------------------------
// TF-IDF Cosine Similarity
// ---------------------------------------------------------------------------

/// Build a TF-IDF vector for a text against a corpus of child scopes.
///
/// Uses rust-tfidf for proper IDF weighting: terms that appear in every
/// child scope get low weight, discriminative terms get high weight.
fn tfidf_vector(
    text: &str,
    corpus: &[Vec<(String, usize)>],
) -> HashMap<String, f64> {
    let terms = extract_term_list(text);
    let mut tf: HashMap<String, usize> = HashMap::new();
    for t in &terms {
        *tf.entry(t.clone()).or_default() += 1;
    }

    // Build processed document for this text: Vec<(&str, usize)>
    let doc: Vec<(&str, usize)> = tf.iter().map(|(k, &v)| (k.as_str(), v)).collect();

    // Build corpus refs for TfIdfDefault
    let corpus_refs: Vec<Vec<(&str, usize)>> = corpus
        .iter()
        .map(|d| d.iter().map(|(k, v)| (k.as_str(), *v)).collect())
        .collect();

    let mut vec = HashMap::new();
    for term in tf.keys() {
        let score = TfIdfDefault::tfidf(term.as_str(), &doc, corpus_refs.iter());
        if score > 0.0 {
            vec.insert(term.clone(), score);
        }
    }
    vec
}

/// Build a processed document (term frequency counts) from text.
fn processed_doc(text: &str) -> Vec<(String, usize)> {
    let terms = extract_term_list(text);
    let mut tf: HashMap<String, usize> = HashMap::new();
    for t in terms {
        *tf.entry(t).or_default() += 1;
    }
    tf.into_iter().collect()
}

/// Cosine similarity between two TF-IDF vectors.
fn cosine_similarity(a: &HashMap<String, f64>, b: &HashMap<String, f64>) -> f64 {
    let common: HashSet<&String> = a.keys().collect::<HashSet<_>>()
        .intersection(&b.keys().collect::<HashSet<_>>())
        .copied()
        .collect();

    if common.is_empty() {
        return 0.0;
    }

    let dot: f64 = common.iter().map(|t| a[*t] * b[*t]).sum();
    let mag_a: f64 = a.values().map(|v| v * v).sum::<f64>().sqrt();
    let mag_b: f64 = b.values().map(|v| v * v).sum::<f64>().sqrt();

    if mag_a == 0.0 || mag_b == 0.0 {
        0.0
    } else {
        dot / (mag_a * mag_b)
    }
}

/// Check orthogonality using combined Jaccard + TF-IDF cosine similarity.
///
/// Two-tier detection:
/// - Jaccard > 0.30 catches brute term overlap (many shared words)
/// - TF-IDF cosine > 0.30 catches nuanced overlap (important shared words)
/// Either trigger flags the pair.
fn check_orthogonality(children: &[Child]) -> Vec<Flag> {
    let mut flags = Vec::new();
    if children.len() < 2 {
        return flags;
    }

    // Build corpus of processed documents for IDF computation
    let corpus: Vec<Vec<(String, usize)>> = children
        .iter()
        .map(|c| processed_doc(&c.scope))
        .collect();

    // Pre-compute term sets for Jaccard
    let term_sets: Vec<HashSet<String>> = children.iter().map(|c| extract_terms(&c.scope)).collect();

    for i in 0..children.len() {
        for j in (i + 1)..children.len() {
            let jaccard = jaccard_similarity(&term_sets[i], &term_sets[j]);

            // Skip pairs with near-zero overlap
            if jaccard < 0.10 {
                continue;
            }

            // Compute TF-IDF cosine for more precise measurement
            let vec_a = tfidf_vector(&children[i].scope, &corpus);
            let vec_b = tfidf_vector(&children[j].scope, &corpus);
            let cosine = cosine_similarity(&vec_a, &vec_b);

            // Flag if EITHER metric exceeds threshold
            if jaccard > 0.30 || cosine > 0.30 {
                let shared: Vec<&String> = term_sets[i]
                    .intersection(&term_sets[j])
                    .take(5)
                    .collect();
                let shared_str: Vec<&str> = shared.iter().map(|s| s.as_str()).collect();

                flags.push(Flag {
                    category: "orthogonality".to_string(),
                    description: format!(
                        "Children '{}' and '{}' have {:.0}% TF-IDF cosine similarity \
                         (Jaccard: {:.0}%). Shared terms: {}",
                        children[i].slug,
                        children[j].slug,
                        cosine * 100.0,
                        jaccard * 100.0,
                        shared_str.join(", "),
                    ),
                });
            }
        }
    }
    flags
}

// ---------------------------------------------------------------------------
// Coverage Check
// ---------------------------------------------------------------------------

fn check_coverage(parent_spec: &str, children: &[Child]) -> Vec<Flag> {
    let parent_terms = extract_terms(parent_spec);
    if parent_terms.is_empty() {
        return Vec::new();
    }

    let mut child_terms = HashSet::new();
    for c in children {
        child_terms.extend(extract_terms(&c.scope));
        child_terms.extend(extract_terms(&c.slug));
    }

    let uncovered: HashSet<&String> = parent_terms.difference(&child_terms).collect();
    let coverage_ratio = 1.0 - (uncovered.len() as f64 / parent_terms.len() as f64);

    if coverage_ratio < 0.80 {
        let mut sample: Vec<&&String> = uncovered.iter().take(5).collect();
        sample.sort();
        let sample_str: Vec<&str> = sample.iter().map(|s| s.as_str()).collect();

        vec![Flag {
            category: "coverage".to_string(),
            description: format!(
                "Only {:.0}% of parent spec terms are covered by children. \
                 Uncovered terms (sample): {}",
                coverage_ratio * 100.0,
                sample_str.join(", "),
            ),
        }]
    } else {
        Vec::new()
    }
}

// ---------------------------------------------------------------------------
// Dependency Cycle Detection (Kahn's Algorithm)
// ---------------------------------------------------------------------------

/// Build dependency graph: slug → set of slugs it depends on.
fn build_dep_graph(children: &[Child]) -> HashMap<String, HashSet<String>> {
    let slugs: HashSet<String> = children.iter().map(|c| c.slug.clone()).collect();

    // Map output terms to producing child
    let mut output_map: HashMap<String, String> = HashMap::new();
    for c in children {
        for term in extract_terms(&c.outputs) {
            output_map.insert(term, c.slug.clone());
        }
    }

    // Build dependencies
    let mut deps: HashMap<String, HashSet<String>> = HashMap::new();
    for c in children {
        let entry = deps.entry(c.slug.clone()).or_default();
        for term in extract_terms(&c.inputs) {
            if let Some(producer) = output_map.get(&term) {
                if producer != &c.slug && slugs.contains(producer) {
                    entry.insert(producer.clone());
                }
            }
        }
    }

    deps
}

/// Detect cycles using Kahn's topological sort.
/// Returns flags if a cycle is found, plus the dependency graph.
fn detect_cycles(children: &[Child]) -> (Vec<Flag>, HashMap<String, HashSet<String>>) {
    let deps = build_dep_graph(children);
    let slugs: HashSet<&String> = deps.keys().collect();

    // in_degree[x] = how many unresolved dependencies x has
    let mut in_degree: HashMap<&String, usize> = HashMap::new();
    for slug in &slugs {
        in_degree.insert(slug, deps.get(*slug).map_or(0, |d| d.len()));
    }

    let mut queue: VecDeque<&String> = in_degree
        .iter()
        .filter(|(_, &d)| d == 0)
        .map(|(&s, _)| s)
        .collect();

    let mut visited = 0usize;
    while let Some(node) = queue.pop_front() {
        visited += 1;
        // Find nodes that depend on `node` and decrement their in-degree
        for (dependent, dep_set) in &deps {
            if dep_set.contains(node) {
                if let Some(deg) = in_degree.get_mut(dependent) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(dependent);
                    }
                }
            }
        }
    }

    let flags = if visited < slugs.len() {
        let cycle_nodes: Vec<String> = in_degree
            .iter()
            .filter(|(_, &d)| d > 0)
            .map(|(&s, _)| s.clone())
            .collect();
        let mut sorted = cycle_nodes;
        sorted.sort();
        vec![Flag {
            category: "cycle".to_string(),
            description: format!(
                "Circular dependency detected among children: {}. \
                 These children have mutual input/output dependencies \
                 that cannot be resolved by sequential execution.",
                sorted.join(", "),
            ),
        }]
    } else {
        Vec::new()
    };

    (flags, deps)
}

// ---------------------------------------------------------------------------
// Budget Check
// ---------------------------------------------------------------------------

fn check_budget(input: &DecomposeInput) -> Vec<Flag> {
    let projected = input.total_nodes + input.children.len();
    let remaining_depth = input.max_depth.saturating_sub(input.current_depth + 1);

    if input.max_total_nodes > 0
        && projected as f64 / input.max_total_nodes as f64 > 0.80
        && remaining_depth > 0
    {
        vec![Flag {
            category: "budget".to_string(),
            description: format!(
                "After this split, {}/{} nodes used ({:.0}%) with {} \
                 depth levels remaining. Budget is tight.",
                projected,
                input.max_total_nodes,
                projected as f64 / input.max_total_nodes as f64 * 100.0,
                remaining_depth,
            ),
        }]
    } else {
        Vec::new()
    }
}

// ---------------------------------------------------------------------------
// Complexity Scoring
// ---------------------------------------------------------------------------

/// Cross-cutting terms that predict architectural complexity.
const CROSS_CUTTING: &[&str] = &[
    "authentication",
    "authorization",
    "logging",
    "monitoring",
    "caching",
    "error-handling",
    "validation",
    "security",
    "testing",
    "deployment",
    "configuration",
    "middleware",
    "database",
    "migration",
    "api",
];

/// Compute complexity score from 0.0 (trivial) to 1.0 (very complex).
///
/// Uses weighted feature scoring — same principle as decision tree
/// split criteria. Each signal contributes proportionally.
pub fn complexity_score(spec_text: &str) -> f64 {
    let terms = extract_terms(spec_text);
    let lines: Vec<&str> = spec_text.lines().collect();

    let cross_cutting_set: HashSet<&str> = CROSS_CUTTING.iter().copied().collect();
    let cross_cutting_count = terms.iter().filter(|t| cross_cutting_set.contains(t.as_str())).count();

    let enumeration_count = lines
        .iter()
        .filter(|l| {
            let trimmed = l.trim();
            trimmed.starts_with("- ")
                || trimmed.starts_with("* ")
                || regex::Regex::new(r"^\d+\.")
                    .unwrap()
                    .is_match(trimmed)
        })
        .count();

    let signals = [
        ("line_count", (lines.len() as f64 / 200.0).min(1.0), 0.15),
        ("term_diversity", (terms.len() as f64 / 80.0).min(1.0), 0.25),
        ("cross_cutting", (cross_cutting_count as f64 / 5.0).min(1.0), 0.35),
        ("enumeration", (enumeration_count as f64 / 10.0).min(1.0), 0.25),
    ];

    let score: f64 = signals.iter().map(|(_, val, weight)| val * weight).sum();
    (score.min(1.0) * 1000.0).round() / 1000.0
}

// ---------------------------------------------------------------------------
// Budget Allocation
// ---------------------------------------------------------------------------

/// Allocate remaining budget proportional to child complexity.
fn allocate_budget(children: &[Child], remaining_budget: usize) -> HashMap<String, usize> {
    let n = children.len();
    if n == 0 {
        return HashMap::new();
    }

    if remaining_budget <= n {
        return children
            .iter()
            .map(|c| (c.slug.clone(), remaining_budget.min(1) / n.max(1)))
            .collect();
    }

    let scores: Vec<f64> = children.iter().map(|c| complexity_score(&c.scope)).collect();
    let total_score: f64 = scores.iter().sum();
    let leftover = remaining_budget - n;

    children
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let extra = if total_score > 0.0 {
                (leftover as f64 * (scores[i] / total_score)) as usize
            } else {
                leftover / n
            };
            (c.slug.clone(), 1 + extra)
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Solve Wave Scheduling (topological sort into parallel waves)
// ---------------------------------------------------------------------------

/// Schedule children into parallel execution waves.
fn compute_solve_waves(deps: &HashMap<String, HashSet<String>>) -> Vec<Vec<String>> {
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

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Run the full decompose gate. Pure function — no I/O.
pub fn run(input: &DecomposeInput) -> DecomposeOutput {
    let remaining_budget = input.max_total_nodes.saturating_sub(input.total_nodes);

    // Collect flags
    let mut flags = Vec::new();
    flags.extend(check_orthogonality(&input.children));
    flags.extend(check_coverage(&input.parent_spec, &input.children));
    flags.extend(check_budget(input));

    let (cycle_flags, deps) = detect_cycles(&input.children);
    flags.extend(cycle_flags);

    // Algorithmic outputs
    let score = complexity_score(&input.parent_spec);
    let budget = allocate_budget(&input.children, remaining_budget);
    let waves = compute_solve_waves(&deps);

    DecomposeOutput {
        flags,
        complexity_score: score,
        budget_allocation: budget,
        dependency_order: waves,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_input(children: Vec<Child>) -> DecomposeInput {
        DecomposeInput {
            parent_spec: "Build a microservices platform with authentication, \
                          data storage, API gateway, and monitoring"
                .to_string(),
            children,
            current_depth: 1,
            max_depth: 3,
            total_nodes: 5,
            max_total_nodes: 20,
        }
    }

    #[test]
    fn test_tfidf_orthogonality_no_overlap() {
        let children = vec![
            Child {
                slug: "auth".into(),
                scope: "OAuth2 authentication and session management".into(),
                boundaries: String::new(),
                inputs: "none".into(),
                outputs: "auth tokens".into(),
            },
            Child {
                slug: "data".into(),
                scope: "PostgreSQL database schema and migrations".into(),
                boundaries: String::new(),
                inputs: "none".into(),
                outputs: "database models".into(),
            },
        ];
        let input = make_input(children);
        let output = run(&input);
        let ortho_flags: Vec<_> = output
            .flags
            .iter()
            .filter(|f| f.category == "orthogonality")
            .collect();
        assert!(ortho_flags.is_empty(), "distinct scopes should not overlap");
    }

    #[test]
    fn test_tfidf_orthogonality_overlap() {
        // With 3+ children, TF-IDF can properly distinguish shared vs unique terms.
        // Terms appearing in overlapping children but not in others get high IDF weight.
        let children = vec![
            Child {
                slug: "auth-login".into(),
                scope: "User authentication login flow with OAuth2 tokens and password validation".into(),
                boundaries: String::new(),
                inputs: "none".into(),
                outputs: "tokens".into(),
            },
            Child {
                slug: "auth-session".into(),
                scope: "User authentication session management with OAuth2 tokens and password reset".into(),
                boundaries: String::new(),
                inputs: "none".into(),
                outputs: "sessions".into(),
            },
            Child {
                slug: "data".into(),
                scope: "PostgreSQL database schema design with migrations and indexing".into(),
                boundaries: String::new(),
                inputs: "none".into(),
                outputs: "models".into(),
            },
        ];
        let input = make_input(children);
        let output = run(&input);
        let ortho_flags: Vec<_> = output
            .flags
            .iter()
            .filter(|f| f.category == "orthogonality")
            .collect();
        assert!(
            !ortho_flags.is_empty(),
            "overlapping auth scopes should be flagged"
        );
        // The flag should mention the auth children, not the data child
        assert!(
            ortho_flags[0].description.contains("auth-login")
                && ortho_flags[0].description.contains("auth-session"),
            "flag should identify the overlapping pair"
        );
    }

    #[test]
    fn test_cycle_detection() {
        let children = vec![
            Child {
                slug: "a".into(),
                scope: "module a".into(),
                boundaries: String::new(),
                inputs: "result-b".into(),
                outputs: "result-a".into(),
            },
            Child {
                slug: "b".into(),
                scope: "module b".into(),
                boundaries: String::new(),
                inputs: "result-a".into(),
                outputs: "result-b".into(),
            },
        ];
        let input = make_input(children);
        let output = run(&input);
        let cycle_flags: Vec<_> = output
            .flags
            .iter()
            .filter(|f| f.category == "cycle")
            .collect();
        assert!(!cycle_flags.is_empty(), "circular deps should be detected");
    }

    #[test]
    fn test_no_cycle() {
        let children = vec![
            Child {
                slug: "auth".into(),
                scope: "authentication".into(),
                boundaries: String::new(),
                inputs: "none".into(),
                outputs: "tokens".into(),
            },
            Child {
                slug: "api".into(),
                scope: "api gateway".into(),
                boundaries: String::new(),
                inputs: "tokens".into(),
                outputs: "endpoints".into(),
            },
        ];
        let input = make_input(children);
        let output = run(&input);
        let cycle_flags: Vec<_> = output
            .flags
            .iter()
            .filter(|f| f.category == "cycle")
            .collect();
        assert!(cycle_flags.is_empty(), "linear deps should have no cycle");
    }

    #[test]
    fn test_solve_waves_linear() {
        let children = vec![
            Child {
                slug: "auth".into(),
                scope: "auth".into(),
                boundaries: String::new(),
                inputs: "none".into(),
                outputs: "tokens sessions".into(),
            },
            Child {
                slug: "api".into(),
                scope: "api".into(),
                boundaries: String::new(),
                inputs: "tokens".into(),
                outputs: "endpoints".into(),
            },
            Child {
                slug: "monitoring".into(),
                scope: "monitoring".into(),
                boundaries: String::new(),
                inputs: "endpoints".into(),
                outputs: "dashboards".into(),
            },
        ];
        let input = make_input(children);
        let output = run(&input);
        // Should be 3 waves: [auth] → [api] → [monitoring]
        assert_eq!(output.dependency_order.len(), 3);
        assert!(output.dependency_order[0].contains(&"auth".to_string()));
        assert!(output.dependency_order[1].contains(&"api".to_string()));
        assert!(output.dependency_order[2].contains(&"monitoring".to_string()));
    }

    #[test]
    fn test_solve_waves_parallel() {
        let children = vec![
            Child {
                slug: "auth".into(),
                scope: "auth".into(),
                boundaries: String::new(),
                inputs: "none".into(),
                outputs: "tokens".into(),
            },
            Child {
                slug: "data".into(),
                scope: "data".into(),
                boundaries: String::new(),
                inputs: "none".into(),
                outputs: "models".into(),
            },
            Child {
                slug: "cache".into(),
                scope: "cache".into(),
                boundaries: String::new(),
                inputs: "none".into(),
                outputs: "cache-layer".into(),
            },
        ];
        let input = make_input(children);
        let output = run(&input);
        // All independent → single wave
        assert_eq!(output.dependency_order.len(), 1);
        assert_eq!(output.dependency_order[0].len(), 3);
    }

    #[test]
    fn test_complexity_score() {
        let simple = complexity_score("Add a button");
        let complex = complexity_score(
            "Build a microservices platform with:\n\
             - Authentication and authorization via OAuth2\n\
             - PostgreSQL database with migrations\n\
             - REST API gateway with middleware\n\
             - Monitoring and logging infrastructure\n\
             - Caching layer with Redis\n\
             - Security hardening\n\
             - Deployment pipeline with CI/CD\n\
             - Configuration management\n\
             - Error handling and validation",
        );
        assert!(
            complex > simple,
            "complex spec ({complex}) should score higher than simple ({simple})"
        );
        assert!(simple < 0.3, "simple spec should score low: {simple}");
        assert!(complex > 0.3, "complex spec should score high: {complex}");
    }

    #[test]
    fn test_budget_allocation() {
        let children = vec![
            Child {
                slug: "simple".into(),
                scope: "add button".into(),
                boundaries: String::new(),
                inputs: String::new(),
                outputs: String::new(),
            },
            Child {
                slug: "complex".into(),
                scope: "Build authentication system with OAuth2, session management, \
                        authorization middleware, and security validation"
                    .into(),
                boundaries: String::new(),
                inputs: String::new(),
                outputs: String::new(),
            },
        ];
        let budget = allocate_budget(&children, 10);
        assert!(
            budget["complex"] > budget["simple"],
            "complex child should get more budget"
        );
        assert!(budget["simple"] >= 1, "every child gets at least 1");
    }

    #[test]
    fn test_coverage_gap() {
        let input = DecomposeInput {
            parent_spec: "Build authentication, database, API, monitoring, and caching".into(),
            children: vec![Child {
                slug: "auth".into(),
                scope: "authentication only".into(),
                boundaries: String::new(),
                inputs: String::new(),
                outputs: String::new(),
            }],
            current_depth: 0,
            max_depth: 3,
            total_nodes: 0,
            max_total_nodes: 20,
        };
        let output = run(&input);
        let coverage_flags: Vec<_> = output
            .flags
            .iter()
            .filter(|f| f.category == "coverage")
            .collect();
        assert!(!coverage_flags.is_empty(), "missing coverage should be flagged");
    }

    // -----------------------------------------------------------------------
    // Spec 064: Additional algorithmic tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_empty_children_no_flags() {
        let input = DecomposeInput {
            parent_spec: "Simple task".into(),
            children: vec![],
            current_depth: 0,
            max_depth: 3,
            total_nodes: 0,
            max_total_nodes: 20,
        };
        let output = run(&input);
        // No children → no orthogonality/cycle flags
        let ortho: Vec<_> = output.flags.iter().filter(|f| f.category == "orthogonality").collect();
        let cycle: Vec<_> = output.flags.iter().filter(|f| f.category == "cycle").collect();
        assert!(ortho.is_empty());
        assert!(cycle.is_empty());
    }

    #[test]
    fn test_budget_tight_flagged() {
        let input = DecomposeInput {
            parent_spec: "Build a system".into(),
            children: vec![
                Child { slug: "a".into(), scope: "module a".into(), boundaries: String::new(), inputs: String::new(), outputs: String::new() },
                Child { slug: "b".into(), scope: "module b".into(), boundaries: String::new(), inputs: String::new(), outputs: String::new() },
            ],
            current_depth: 0,
            max_depth: 3,
            total_nodes: 16, // 16 + 2 = 18/20 = 90% > 80%
            max_total_nodes: 20,
        };
        let output = run(&input);
        let budget_flags: Vec<_> = output.flags.iter().filter(|f| f.category == "budget").collect();
        assert!(!budget_flags.is_empty(), "budget should be flagged when > 80% used");
    }

    #[test]
    fn test_budget_ok_not_flagged() {
        let input = DecomposeInput {
            parent_spec: "Build a system".into(),
            children: vec![
                Child { slug: "a".into(), scope: "module a".into(), boundaries: String::new(), inputs: String::new(), outputs: String::new() },
            ],
            current_depth: 0,
            max_depth: 3,
            total_nodes: 2, // 2 + 1 = 3/20 = 15%
            max_total_nodes: 20,
        };
        let output = run(&input);
        let budget_flags: Vec<_> = output.flags.iter().filter(|f| f.category == "budget").collect();
        assert!(budget_flags.is_empty(), "low budget usage should not be flagged");
    }

    #[test]
    fn test_complexity_score_range() {
        // Trivial
        let trivial = complexity_score("");
        assert!(trivial >= 0.0 && trivial <= 1.0, "score must be in [0,1]: {}", trivial);

        // Very complex
        let complex = complexity_score(&"authentication authorization logging monitoring caching error-handling validation security testing deployment configuration middleware database migration api\n".repeat(20));
        assert!(complex >= 0.0 && complex <= 1.0, "score must be in [0,1]: {}", complex);
        assert!(complex > 0.3, "heavily cross-cutting spec should score high: {}", complex);
    }

    #[test]
    fn test_allocate_budget_empty_children() {
        let budget = allocate_budget(&[], 10);
        assert!(budget.is_empty());
    }

    #[test]
    fn test_allocate_budget_all_get_at_least_one() {
        let children = vec![
            Child { slug: "a".into(), scope: "tiny".into(), boundaries: String::new(), inputs: String::new(), outputs: String::new() },
            Child { slug: "b".into(), scope: "tiny".into(), boundaries: String::new(), inputs: String::new(), outputs: String::new() },
            Child { slug: "c".into(), scope: "tiny".into(), boundaries: String::new(), inputs: String::new(), outputs: String::new() },
        ];
        let budget = allocate_budget(&children, 10);
        for child in &children {
            assert!(*budget.get(&child.slug).unwrap_or(&0) >= 1, "every child should get at least 1");
        }
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let mut a = HashMap::new();
        a.insert("auth".to_string(), 0.5);
        a.insert("system".to_string(), 0.3);
        let sim = cosine_similarity(&a, &a);
        assert!((sim - 1.0).abs() < 1e-6, "identical vectors should have cosine 1.0");
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let mut a = HashMap::new();
        a.insert("auth".to_string(), 1.0);
        let mut b = HashMap::new();
        b.insert("database".to_string(), 1.0);
        let sim = cosine_similarity(&a, &b);
        assert!((sim).abs() < 1e-6, "orthogonal vectors should have cosine 0.0");
    }

    #[test]
    fn test_three_child_dependency_chain() {
        // A → B → C chain
        let children = vec![
            Child { slug: "a".into(), scope: "foundation".into(), boundaries: String::new(), inputs: "none".into(), outputs: "core-modules".into() },
            Child { slug: "b".into(), scope: "middleware".into(), boundaries: String::new(), inputs: "core-modules".into(), outputs: "api-layer".into() },
            Child { slug: "c".into(), scope: "frontend".into(), boundaries: String::new(), inputs: "api-layer".into(), outputs: "ui-components".into() },
        ];
        let input = make_input(children);
        let output = run(&input);
        // Should produce 3 sequential waves
        assert_eq!(output.dependency_order.len(), 3);
    }

    #[test]
    fn test_diamond_dependency_pattern() {
        // a → b, a → c, b+c → d
        let children = vec![
            Child { slug: "a".into(), scope: "base".into(), boundaries: String::new(), inputs: "none".into(), outputs: "foundation".into() },
            Child { slug: "b".into(), scope: "left-branch".into(), boundaries: String::new(), inputs: "foundation".into(), outputs: "left-output".into() },
            Child { slug: "c".into(), scope: "right-branch".into(), boundaries: String::new(), inputs: "foundation".into(), outputs: "right-output".into() },
            Child { slug: "d".into(), scope: "merge-point".into(), boundaries: String::new(), inputs: "left-output right-output".into(), outputs: "final".into() },
        ];
        let input = make_input(children);
        let output = run(&input);
        // Wave 1: [a], Wave 2: [b,c], Wave 3: [d]
        assert_eq!(output.dependency_order.len(), 3);
        assert!(output.dependency_order[0].contains(&"a".to_string()));
        assert_eq!(output.dependency_order[1].len(), 2); // b and c parallel
        assert!(output.dependency_order[2].contains(&"d".to_string()));
    }
}
