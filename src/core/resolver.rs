//! Dependency resolution
//!
//! Handles computing build order and detecting dependency conflicts.

use std::collections::{HashMap, HashSet};

use crate::error::ResolverError;
use semver::{Version, VersionReq};

/// A dependency with version constraint
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionedDependency {
    /// Package name
    pub name: String,
    /// Version constraint (semver syntax)
    pub constraint: VersionReq,
}

impl VersionedDependency {
    /// Create a new versioned dependency
    pub fn new(name: &str, constraint: &str) -> Result<Self, ResolverError> {
        let constraint = VersionReq::parse(constraint).map_err(|e| ResolverError::Conflict {
            message: format!("Invalid version constraint '{constraint}' for '{name}': {e}"),
        })?;
        Ok(Self {
            name: name.to_string(),
            constraint,
        })
    }

    /// Check if a version satisfies this constraint
    pub fn satisfies(&self, version: &Version) -> bool {
        self.constraint.matches(version)
    }
}

/// Parse a semver constraint string
pub fn parse_version_constraint(constraint: &str) -> Result<VersionReq, ResolverError> {
    VersionReq::parse(constraint).map_err(|e| ResolverError::Conflict {
        message: format!("Invalid version constraint '{constraint}': {e}"),
    })
}

/// Parse a version string
pub fn parse_version(version: &str) -> Result<Version, ResolverError> {
    Version::parse(version).map_err(|e| ResolverError::Conflict {
        message: format!("Invalid version '{version}': {e}"),
    })
}

/// Check if a version satisfies a constraint
pub fn version_satisfies(version: &str, constraint: &str) -> Result<bool, ResolverError> {
    let ver = parse_version(version)?;
    let req = parse_version_constraint(constraint)?;
    Ok(req.matches(&ver))
}

/// Find a compatible version from available versions that satisfies all constraints
pub fn find_compatible_version(
    available: &[String],
    constraints: &[String],
) -> Result<Option<String>, ResolverError> {
    // Parse all constraints
    let reqs: Vec<VersionReq> = constraints
        .iter()
        .map(|c| parse_version_constraint(c))
        .collect::<Result<Vec<_>, _>>()?;

    // Parse and filter available versions
    let mut versions: Vec<Version> = available
        .iter()
        .filter_map(|v| Version::parse(v).ok())
        .collect();

    // Sort descending to prefer newer versions
    versions.sort_by(|a, b| b.cmp(a));

    // Find first version that satisfies all constraints
    for ver in versions {
        if reqs.iter().all(|req| req.matches(&ver)) {
            return Ok(Some(ver.to_string()));
        }
    }

    Ok(None)
}

/// Detect version conflicts between multiple constraints for the same package
pub fn detect_version_conflict(
    package: &str,
    constraints: &[String],
    available_versions: &[String],
) -> Result<(), ResolverError> {
    if constraints.is_empty() {
        return Ok(());
    }

    // Try to find a compatible version
    if find_compatible_version(available_versions, constraints)?.is_some() {
        Ok(())
    } else {
        let constraints_str = constraints.join(", ");
        Err(ResolverError::Conflict {
            message: format!(
                "No compatible version found for package '{}'. \
                 Conflicting constraints: [{}]. \
                 Available versions: [{}]",
                package,
                constraints_str,
                available_versions.join(", ")
            ),
        })
    }
}

/// Dependency graph for packages
#[derive(Debug, Default)]
pub struct DependencyGraph {
    /// Adjacency list: package -> dependencies
    edges: HashMap<String, Vec<String>>,
    /// All known packages
    nodes: HashSet<String>,
}

impl DependencyGraph {
    /// Create a new empty dependency graph
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a package to the graph
    pub fn add_package(&mut self, name: &str, dependencies: Vec<String>) {
        self.nodes.insert(name.to_string());
        for dep in &dependencies {
            self.nodes.insert(dep.clone());
        }
        self.edges.insert(name.to_string(), dependencies);
    }

    /// Compute topological sort (build order)
    ///
    /// Returns packages in order such that dependencies come before dependents.
    pub fn topological_sort(&self) -> Result<Vec<String>, ResolverError> {
        let mut visited = HashSet::new();
        let mut temp_visited = HashSet::new();
        let mut result = Vec::new();
        let mut cycle_path = Vec::new();

        for node in &self.nodes {
            if !visited.contains(node) {
                self.visit(
                    node,
                    &mut visited,
                    &mut temp_visited,
                    &mut result,
                    &mut cycle_path,
                )?;
            }
        }

        Ok(result)
    }

    fn visit(
        &self,
        node: &str,
        visited: &mut HashSet<String>,
        temp_visited: &mut HashSet<String>,
        result: &mut Vec<String>,
        cycle_path: &mut Vec<String>,
    ) -> Result<(), ResolverError> {
        if temp_visited.contains(node) {
            // Found a cycle
            cycle_path.push(node.to_string());
            return Err(ResolverError::CircularDependency {
                cycle: cycle_path.clone(),
            });
        }

        if visited.contains(node) {
            return Ok(());
        }

        temp_visited.insert(node.to_string());
        cycle_path.push(node.to_string());

        if let Some(deps) = self.edges.get(node) {
            for dep in deps {
                self.visit(dep, visited, temp_visited, result, cycle_path)?;
            }
        }

        cycle_path.pop();
        temp_visited.remove(node);
        visited.insert(node.to_string());
        result.push(node.to_string());

        Ok(())
    }

    /// Check if the graph has any cycles
    pub fn has_cycle(&self) -> bool {
        self.topological_sort().is_err()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // ============================================
    // Unit Tests - Basic dependency graph operations
    // ============================================

    #[test]
    fn test_simple_dependency_order() {
        let mut graph = DependencyGraph::new();
        graph.add_package("app", vec!["lib".to_string()]);
        graph.add_package("lib", vec![]);

        let order = graph.topological_sort().unwrap();
        let lib_pos = order.iter().position(|x| x == "lib").unwrap();
        let app_pos = order.iter().position(|x| x == "app").unwrap();

        assert!(lib_pos < app_pos, "lib should be built before app");
    }

    #[test]
    fn test_circular_dependency_detection() {
        let mut graph = DependencyGraph::new();
        graph.add_package("a", vec!["b".to_string()]);
        graph.add_package("b", vec!["c".to_string()]);
        graph.add_package("c", vec!["a".to_string()]);

        assert!(graph.has_cycle());
        assert!(graph.topological_sort().is_err());
    }

    #[test]
    fn test_dependency_graph_builds_from_package_definitions() {
        let mut graph = DependencyGraph::new();

        // Simulate building from package definitions
        graph.add_package("busybox", vec![]);
        graph.add_package("dropbear", vec!["zlib".to_string()]);
        graph.add_package("zlib", vec![]);
        graph.add_package("nginx", vec!["zlib".to_string(), "openssl".to_string()]);
        graph.add_package("openssl", vec![]);

        let order = graph.topological_sort().unwrap();

        // Verify all packages are in the order
        assert_eq!(order.len(), 5);
        assert!(order.contains(&"busybox".to_string()));
        assert!(order.contains(&"dropbear".to_string()));
        assert!(order.contains(&"zlib".to_string()));
        assert!(order.contains(&"nginx".to_string()));
        assert!(order.contains(&"openssl".to_string()));
    }

    #[test]
    fn test_topological_sort_produces_valid_build_order() {
        let mut graph = DependencyGraph::new();
        graph.add_package("app", vec!["lib1".to_string(), "lib2".to_string()]);
        graph.add_package("lib1", vec!["base".to_string()]);
        graph.add_package("lib2", vec!["base".to_string()]);
        graph.add_package("base", vec![]);

        let order = graph.topological_sort().unwrap();

        // Verify base comes before lib1 and lib2
        let base_pos = order.iter().position(|x| x == "base").unwrap();
        let lib1_pos = order.iter().position(|x| x == "lib1").unwrap();
        let lib2_pos = order.iter().position(|x| x == "lib2").unwrap();
        let app_pos = order.iter().position(|x| x == "app").unwrap();

        assert!(base_pos < lib1_pos, "base should come before lib1");
        assert!(base_pos < lib2_pos, "base should come before lib2");
        assert!(lib1_pos < app_pos, "lib1 should come before app");
        assert!(lib2_pos < app_pos, "lib2 should come before app");
    }

    #[test]
    fn test_every_package_built_after_dependencies() {
        let mut graph = DependencyGraph::new();
        graph.add_package("a", vec!["b".to_string(), "c".to_string()]);
        graph.add_package("b", vec!["d".to_string()]);
        graph.add_package("c", vec!["d".to_string()]);
        graph.add_package("d", vec![]);

        let order = graph.topological_sort().unwrap();

        // For each package, verify all its dependencies come before it
        let deps_map: HashMap<&str, Vec<&str>> = [
            ("a", vec!["b", "c"]),
            ("b", vec!["d"]),
            ("c", vec!["d"]),
            ("d", vec![]),
        ]
        .into_iter()
        .collect();

        for (pkg, deps) in deps_map {
            let pkg_pos = order.iter().position(|x| x == pkg).unwrap();
            for dep in deps {
                let dep_pos = order.iter().position(|x| x == dep).unwrap();
                assert!(
                    dep_pos < pkg_pos,
                    "Dependency {dep} should come before {pkg}"
                );
            }
        }
    }

    #[test]
    fn test_circular_dependency_reported_with_cycle() {
        let mut graph = DependencyGraph::new();
        graph.add_package("a", vec!["b".to_string()]);
        graph.add_package("b", vec!["c".to_string()]);
        graph.add_package("c", vec!["a".to_string()]);

        let result = graph.topological_sort();
        assert!(result.is_err());

        match result.unwrap_err() {
            ResolverError::CircularDependency { cycle } => {
                // The cycle should contain the packages involved
                assert!(!cycle.is_empty(), "Cycle should not be empty");
                // At least one of the cycle packages should be in the path
                let has_cycle_pkg = cycle.iter().any(|p| p == "a" || p == "b" || p == "c");
                assert!(has_cycle_pkg, "Cycle should contain cycle packages");
            }
            _ => panic!("Expected CircularDependency error"),
        }
    }

    #[test]
    fn test_self_referential_dependency() {
        let mut graph = DependencyGraph::new();
        graph.add_package("a", vec!["a".to_string()]);

        assert!(graph.has_cycle());
        let result = graph.topological_sort();
        assert!(result.is_err());
    }

    #[test]
    fn test_two_node_cycle() {
        let mut graph = DependencyGraph::new();
        graph.add_package("a", vec!["b".to_string()]);
        graph.add_package("b", vec!["a".to_string()]);

        assert!(graph.has_cycle());
    }

    #[test]
    fn test_empty_graph() {
        let graph = DependencyGraph::new();
        let order = graph.topological_sort().unwrap();
        assert!(order.is_empty());
    }

    #[test]
    fn test_single_package_no_deps() {
        let mut graph = DependencyGraph::new();
        graph.add_package("solo", vec![]);

        let order = graph.topological_sort().unwrap();
        assert_eq!(order, vec!["solo"]);
    }

    #[test]
    fn test_diamond_dependency() {
        // Diamond pattern: A depends on B and C, both B and C depend on D
        let mut graph = DependencyGraph::new();
        graph.add_package("a", vec!["b".to_string(), "c".to_string()]);
        graph.add_package("b", vec!["d".to_string()]);
        graph.add_package("c", vec!["d".to_string()]);
        graph.add_package("d", vec![]);

        let order = graph.topological_sort().unwrap();

        let d_pos = order.iter().position(|x| x == "d").unwrap();
        let b_pos = order.iter().position(|x| x == "b").unwrap();
        let c_pos = order.iter().position(|x| x == "c").unwrap();
        let a_pos = order.iter().position(|x| x == "a").unwrap();

        assert!(d_pos < b_pos);
        assert!(d_pos < c_pos);
        assert!(b_pos < a_pos);
        assert!(c_pos < a_pos);
    }

    #[test]
    fn test_multiple_independent_chains() {
        let mut graph = DependencyGraph::new();
        // Chain 1: a -> b -> c
        graph.add_package("a", vec!["b".to_string()]);
        graph.add_package("b", vec!["c".to_string()]);
        graph.add_package("c", vec![]);
        // Chain 2: x -> y -> z
        graph.add_package("x", vec!["y".to_string()]);
        graph.add_package("y", vec!["z".to_string()]);
        graph.add_package("z", vec![]);

        let order = graph.topological_sort().unwrap();
        assert_eq!(order.len(), 6);

        // Verify chain 1 order
        let c_pos = order.iter().position(|x| x == "c").unwrap();
        let b_pos = order.iter().position(|x| x == "b").unwrap();
        let a_pos = order.iter().position(|x| x == "a").unwrap();
        assert!(c_pos < b_pos);
        assert!(b_pos < a_pos);

        // Verify chain 2 order
        let z_pos = order.iter().position(|x| x == "z").unwrap();
        let y_pos = order.iter().position(|x| x == "y").unwrap();
        let x_pos = order.iter().position(|x| x == "x").unwrap();
        assert!(z_pos < y_pos);
        assert!(y_pos < x_pos);
    }

    // ============================================
    // Property-Based Tests
    // ============================================

    /// Strategy for generating a DAG (directed acyclic graph)
    /// We generate packages in layers where each layer can only depend on previous layers
    fn dag_strategy() -> impl Strategy<Value = Vec<(String, Vec<String>)>> {
        // Generate 2-5 layers with 1-3 packages each
        (2usize..=5, 1usize..=3).prop_flat_map(|(num_layers, pkgs_per_layer)| {
            let mut strategies: Vec<BoxedStrategy<(String, Vec<String>)>> = Vec::new();

            for layer in 0..num_layers {
                for pkg_idx in 0..pkgs_per_layer {
                    let pkg_name = format!("pkg_l{layer}_p{pkg_idx}");

                    if layer == 0 {
                        // First layer has no dependencies
                        strategies.push(Just((pkg_name, vec![])).boxed());
                    } else {
                        // Later layers can depend on any package from previous layers
                        let prev_pkgs: Vec<String> = (0..layer)
                            .flat_map(|l| {
                                (0..pkgs_per_layer).map(move |p| format!("pkg_l{l}_p{p}"))
                            })
                            .collect();

                        let pkg_name_clone = pkg_name.clone();
                        strategies.push(
                            proptest::collection::vec(proptest::sample::select(prev_pkgs), 0..=2)
                                .prop_map(move |deps| (pkg_name_clone.clone(), deps))
                                .boxed(),
                        );
                    }
                }
            }

            strategies.into_iter().collect::<Vec<_>>()
        })
    }

    /// Strategy for generating a graph with a cycle
    fn cyclic_graph_strategy() -> impl Strategy<Value = Vec<(String, Vec<String>)>> {
        // Generate a simple cycle: a -> b -> c -> a
        Just(vec![
            ("a".to_string(), vec!["b".to_string()]),
            ("b".to_string(), vec!["c".to_string()]),
            ("c".to_string(), vec!["a".to_string()]),
        ])
    }

    // ============================================
    // Unit Tests - Version Constraints
    // ============================================

    #[test]
    fn test_parse_exact_version_constraint() {
        let req = parse_version_constraint("=1.0.0").unwrap();
        assert!(req.matches(&Version::parse("1.0.0").unwrap()));
        assert!(!req.matches(&Version::parse("1.0.1").unwrap()));
    }

    #[test]
    fn test_parse_caret_version_constraint() {
        let req = parse_version_constraint("^1.2.3").unwrap();
        assert!(req.matches(&Version::parse("1.2.3").unwrap()));
        assert!(req.matches(&Version::parse("1.2.4").unwrap()));
        assert!(req.matches(&Version::parse("1.9.0").unwrap()));
        assert!(!req.matches(&Version::parse("2.0.0").unwrap()));
        assert!(!req.matches(&Version::parse("1.2.2").unwrap()));
    }

    #[test]
    fn test_parse_tilde_version_constraint() {
        let req = parse_version_constraint("~1.2.3").unwrap();
        assert!(req.matches(&Version::parse("1.2.3").unwrap()));
        assert!(req.matches(&Version::parse("1.2.9").unwrap()));
        assert!(!req.matches(&Version::parse("1.3.0").unwrap()));
    }

    #[test]
    fn test_parse_greater_than_constraint() {
        let req = parse_version_constraint(">=1.0.0").unwrap();
        assert!(req.matches(&Version::parse("1.0.0").unwrap()));
        assert!(req.matches(&Version::parse("2.0.0").unwrap()));
        assert!(!req.matches(&Version::parse("0.9.9").unwrap()));
    }

    #[test]
    fn test_parse_less_than_constraint() {
        let req = parse_version_constraint("<2.0.0").unwrap();
        assert!(req.matches(&Version::parse("1.9.9").unwrap()));
        assert!(!req.matches(&Version::parse("2.0.0").unwrap()));
    }

    #[test]
    fn test_parse_range_constraint() {
        let req = parse_version_constraint(">=1.0.0, <2.0.0").unwrap();
        assert!(req.matches(&Version::parse("1.0.0").unwrap()));
        assert!(req.matches(&Version::parse("1.5.0").unwrap()));
        assert!(!req.matches(&Version::parse("2.0.0").unwrap()));
        assert!(!req.matches(&Version::parse("0.9.0").unwrap()));
    }

    #[test]
    fn test_parse_wildcard_constraint() {
        let req = parse_version_constraint("1.*").unwrap();
        assert!(req.matches(&Version::parse("1.0.0").unwrap()));
        assert!(req.matches(&Version::parse("1.9.9").unwrap()));
        assert!(!req.matches(&Version::parse("2.0.0").unwrap()));
    }

    #[test]
    fn test_version_satisfies_valid() {
        assert!(version_satisfies("1.2.3", "^1.0.0").unwrap());
        assert!(version_satisfies("1.2.3", ">=1.0.0").unwrap());
        assert!(version_satisfies("1.2.3", "~1.2.0").unwrap());
    }

    #[test]
    fn test_version_satisfies_invalid() {
        assert!(!version_satisfies("2.0.0", "^1.0.0").unwrap());
        assert!(!version_satisfies("0.9.0", ">=1.0.0").unwrap());
    }

    #[test]
    fn test_find_compatible_version_single_constraint() {
        let available = vec![
            "1.0.0".to_string(),
            "1.1.0".to_string(),
            "2.0.0".to_string(),
        ];
        let constraints = vec!["^1.0.0".to_string()];

        let result = find_compatible_version(&available, &constraints).unwrap();
        assert_eq!(result, Some("1.1.0".to_string())); // Prefers newer
    }

    #[test]
    fn test_find_compatible_version_multiple_constraints() {
        let available = vec![
            "1.0.0".to_string(),
            "1.1.0".to_string(),
            "1.2.0".to_string(),
            "2.0.0".to_string(),
        ];
        let constraints = vec![">=1.0.0".to_string(), "<1.2.0".to_string()];

        let result = find_compatible_version(&available, &constraints).unwrap();
        assert_eq!(result, Some("1.1.0".to_string()));
    }

    #[test]
    fn test_find_compatible_version_no_match() {
        let available = vec!["1.0.0".to_string(), "1.1.0".to_string()];
        let constraints = vec![">=2.0.0".to_string()];

        let result = find_compatible_version(&available, &constraints).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_detect_version_conflict_no_conflict() {
        let constraints = vec![">=1.0.0".to_string(), "<2.0.0".to_string()];
        let available = vec![
            "1.0.0".to_string(),
            "1.5.0".to_string(),
            "2.0.0".to_string(),
        ];

        let result = detect_version_conflict("test-pkg", &constraints, &available);
        assert!(result.is_ok());
    }

    #[test]
    fn test_detect_version_conflict_with_conflict() {
        let constraints = vec![">=2.0.0".to_string(), "<1.5.0".to_string()];
        let available = vec![
            "1.0.0".to_string(),
            "1.5.0".to_string(),
            "2.0.0".to_string(),
        ];

        let result = detect_version_conflict("test-pkg", &constraints, &available);
        assert!(result.is_err());

        match result.unwrap_err() {
            ResolverError::Conflict { message } => {
                assert!(
                    message.contains("test-pkg"),
                    "Error should mention package name"
                );
                assert!(
                    message.contains(">=2.0.0"),
                    "Error should mention constraint"
                );
                assert!(
                    message.contains("<1.5.0"),
                    "Error should mention constraint"
                );
            }
            _ => panic!("Expected Conflict error"),
        }
    }

    #[test]
    fn test_versioned_dependency_creation() {
        let dep = VersionedDependency::new("zlib", ">=1.2.0").unwrap();
        assert_eq!(dep.name, "zlib");
        assert!(dep.satisfies(&Version::parse("1.2.0").unwrap()));
        assert!(dep.satisfies(&Version::parse("1.3.0").unwrap()));
        assert!(!dep.satisfies(&Version::parse("1.1.0").unwrap()));
    }

    #[test]
    fn test_versioned_dependency_invalid_constraint() {
        let result = VersionedDependency::new("zlib", "invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_conflict_error_message_is_clear() {
        let constraints = vec!["^1.0.0".to_string(), "^2.0.0".to_string()];
        let available = vec!["1.0.0".to_string(), "2.0.0".to_string()];

        let result = detect_version_conflict("conflicting-pkg", &constraints, &available);
        assert!(result.is_err());

        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("conflicting-pkg"),
            "Should mention package: {err_msg}"
        );
        assert!(
            err_msg.contains("^1.0.0"),
            "Should mention first constraint: {err_msg}"
        );
        assert!(
            err_msg.contains("^2.0.0"),
            "Should mention second constraint: {err_msg}"
        );
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: zigroot-cli, Property 2: Dependency Build Order
        /// For any valid dependency graph without cycles, the computed build order
        /// SHALL ensure that every package is built after all of its dependencies.
        /// **Validates: Requirements 20.1, 20.2**
        #[test]
        fn prop_build_order_respects_dependencies(packages in dag_strategy()) {
            let mut graph = DependencyGraph::new();
            let mut deps_map: HashMap<String, Vec<String>> = HashMap::new();

            for (name, deps) in &packages {
                graph.add_package(name, deps.clone());
                deps_map.insert(name.clone(), deps.clone());
            }

            let order = graph.topological_sort()
                .expect("DAG should produce valid topological sort");

            // Verify every package comes after its dependencies
            for (pkg, deps) in &deps_map {
                if let Some(pkg_pos) = order.iter().position(|x| x == pkg) {
                    for dep in deps {
                        if let Some(dep_pos) = order.iter().position(|x| x == dep) {
                            prop_assert!(
                                dep_pos < pkg_pos,
                                "Dependency {} (pos {}) should come before {} (pos {})",
                                dep, dep_pos, pkg, pkg_pos
                            );
                        }
                    }
                }
            }
        }

        /// Feature: zigroot-cli, Property 3: Circular Dependency Detection
        /// For any dependency graph containing a cycle, the dependency resolver
        /// SHALL detect and report the cycle.
        /// **Validates: Requirements 20.3**
        #[test]
        fn prop_detects_circular_dependencies(packages in cyclic_graph_strategy()) {
            let mut graph = DependencyGraph::new();

            for (name, deps) in &packages {
                graph.add_package(name, deps.clone());
            }

            prop_assert!(graph.has_cycle(), "Should detect cycle in cyclic graph");

            let result = graph.topological_sort();
            prop_assert!(result.is_err(), "Topological sort should fail for cyclic graph");

            match result.unwrap_err() {
                ResolverError::CircularDependency { cycle } => {
                    prop_assert!(!cycle.is_empty(), "Cycle path should not be empty");
                }
                _ => prop_assert!(false, "Expected CircularDependency error"),
            }
        }

        /// Property: All packages in graph appear in build order
        #[test]
        fn prop_all_packages_in_build_order(packages in dag_strategy()) {
            let mut graph = DependencyGraph::new();
            let mut all_names: HashSet<String> = HashSet::new();

            for (name, deps) in &packages {
                graph.add_package(name, deps.clone());
                all_names.insert(name.clone());
                for dep in deps {
                    all_names.insert(dep.clone());
                }
            }

            let order = graph.topological_sort()
                .expect("DAG should produce valid topological sort");

            let order_set: HashSet<String> = order.into_iter().collect();

            for name in &all_names {
                prop_assert!(
                    order_set.contains(name),
                    "Package {} should be in build order",
                    name
                );
            }
        }

        /// Property: Build order has no duplicates
        #[test]
        fn prop_build_order_no_duplicates(packages in dag_strategy()) {
            let mut graph = DependencyGraph::new();

            for (name, deps) in &packages {
                graph.add_package(name, deps.clone());
            }

            let order = graph.topological_sort()
                .expect("DAG should produce valid topological sort");

            let unique: HashSet<&String> = order.iter().collect();
            prop_assert_eq!(
                unique.len(),
                order.len(),
                "Build order should have no duplicates"
            );
        }

        /// Feature: zigroot-cli, Property 21: Dependency Conflict Detection
        /// For any set of packages with incompatible version constraints,
        /// the resolver SHALL detect and report the conflict with a clear error message.
        /// **Validates: Requirements 2.9**
        #[test]
        fn prop_conflict_detection_reports_clearly(
            major1 in 1u32..5,
            major2 in 1u32..5,
        ) {
            // Create constraints that may or may not conflict
            let constraint1 = format!("^{major1}.0.0");
            let constraint2 = format!("^{major2}.0.0");
            let constraints = vec![constraint1.clone(), constraint2.clone()];

            // Available versions span multiple majors
            let available: Vec<String> = (1..=5)
                .map(|m| format!("{m}.0.0"))
                .collect();

            let result = detect_version_conflict("test-pkg", &constraints, &available);

            if major1 != major2 {
                // Different major versions with caret constraints should conflict
                prop_assert!(result.is_err(), "Different major versions should conflict");

                let err_msg = result.unwrap_err().to_string();
                prop_assert!(
                    err_msg.contains("test-pkg"),
                    "Error should mention package name"
                );
            } else {
                // Same major version should not conflict
                prop_assert!(result.is_ok(), "Same major version should not conflict");
            }
        }

        /// Property: Compatible constraints always find a version
        #[test]
        fn prop_compatible_constraints_find_version(
            major in 1u32..10,
            minor in 0u32..10,
        ) {
            let version = format!("{major}.{minor}.0");
            let constraint = format!(">={major}.0.0, <{}.0.0", major + 1);
            let available = vec![version.clone()];
            let constraints = vec![constraint];

            let result = find_compatible_version(&available, &constraints);
            prop_assert!(result.is_ok());
            prop_assert_eq!(result.unwrap(), Some(version));
        }
    }
}
