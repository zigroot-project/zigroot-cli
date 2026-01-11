//! Dependency resolution
//!
//! Handles computing build order and detecting dependency conflicts.

use std::collections::{HashMap, HashSet};

use crate::error::ResolverError;

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
}
