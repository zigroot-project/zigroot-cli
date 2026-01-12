//! Dependency tree visualization
//!
//! Provides functionality to display package dependencies as a tree
//! or export them in DOT graph format.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::core::manifest::Manifest;
use crate::error::ZigrootError;

/// Dependency type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyType {
    /// Build-time dependency (depends)
    Build,
    /// Runtime dependency (requires)
    Runtime,
}

impl std::fmt::Display for DependencyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Build => write!(f, "build"),
            Self::Runtime => write!(f, "runtime"),
        }
    }
}

/// A dependency edge in the tree
#[derive(Debug, Clone)]
pub struct DependencyEdge {
    /// Target package name
    pub target: String,
    /// Type of dependency
    pub dep_type: DependencyType,
}

/// Dependency tree structure
#[derive(Debug, Default)]
pub struct DependencyTree {
    /// Package dependencies: package -> list of dependencies
    dependencies: HashMap<String, Vec<DependencyEdge>>,
    /// All package names
    packages: HashSet<String>,
    /// Root packages (packages in manifest)
    roots: Vec<String>,
}

impl DependencyTree {
    /// Create a new empty dependency tree
    pub fn new() -> Self {
        Self::default()
    }

    /// Build dependency tree from a manifest
    pub fn from_manifest(manifest: &Manifest) -> Self {
        let mut tree = Self::new();

        // Add all packages from manifest as roots
        for pkg_name in manifest.packages.keys() {
            tree.packages.insert(pkg_name.clone());
            tree.roots.push(pkg_name.clone());
            // Initialize empty dependencies (actual deps would come from package definitions)
            tree.dependencies.insert(pkg_name.clone(), Vec::new());
        }

        // Sort roots for consistent output
        tree.roots.sort();

        tree
    }

    /// Add a dependency edge
    pub fn add_dependency(&mut self, from: &str, to: &str, dep_type: DependencyType) {
        self.packages.insert(from.to_string());
        self.packages.insert(to.to_string());

        let deps = self.dependencies.entry(from.to_string()).or_default();
        deps.push(DependencyEdge {
            target: to.to_string(),
            dep_type,
        });
    }

    /// Get all root packages
    pub fn roots(&self) -> &[String] {
        &self.roots
    }

    /// Get dependencies for a package
    pub fn dependencies(&self, package: &str) -> Option<&Vec<DependencyEdge>> {
        self.dependencies.get(package)
    }

    /// Check if tree is empty
    pub fn is_empty(&self) -> bool {
        self.packages.is_empty()
    }

    /// Get all packages
    pub fn packages(&self) -> &HashSet<String> {
        &self.packages
    }

    /// Detect circular dependencies
    pub fn detect_cycles(&self) -> Option<Vec<String>> {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        for pkg in &self.packages {
            if !visited.contains(pkg) {
                if let Some(cycle) = self.detect_cycle_dfs(pkg, &mut visited, &mut rec_stack, &mut path) {
                    return Some(cycle);
                }
            }
        }

        None
    }

    fn detect_cycle_dfs(
        &self,
        node: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());
        path.push(node.to_string());

        if let Some(deps) = self.dependencies.get(node) {
            for dep in deps {
                if !visited.contains(&dep.target) {
                    if let Some(cycle) = self.detect_cycle_dfs(&dep.target, visited, rec_stack, path) {
                        return Some(cycle);
                    }
                } else if rec_stack.contains(&dep.target) {
                    // Found a cycle
                    let mut cycle = path.clone();
                    cycle.push(dep.target.clone());
                    return Some(cycle);
                }
            }
        }

        path.pop();
        rec_stack.remove(node);
        None
    }

    /// Format as tree string
    pub fn format_tree(&self) -> String {
        if self.is_empty() {
            return "No packages in project".to_string();
        }

        let mut output = String::new();
        output.push_str("Dependency Tree:\n");

        // Check for cycles first
        if let Some(cycle) = self.detect_cycles() {
            output.push_str(&format!("\n⚠ Circular dependency detected: {}\n", cycle.join(" -> ")));
        }

        for (i, root) in self.roots.iter().enumerate() {
            let is_last = i == self.roots.len() - 1;
            self.format_node(&mut output, root, "", is_last, &mut HashSet::new());
        }

        output
    }

    fn format_node(
        &self,
        output: &mut String,
        node: &str,
        prefix: &str,
        is_last: bool,
        visited: &mut HashSet<String>,
    ) {
        let connector = if is_last { "└── " } else { "├── " };
        output.push_str(&format!("{prefix}{connector}{node}\n"));

        if visited.contains(node) {
            // Already visited, don't recurse (prevents infinite loops)
            return;
        }
        visited.insert(node.to_string());

        if let Some(deps) = self.dependencies.get(node) {
            let child_prefix = if is_last {
                format!("{prefix}    ")
            } else {
                format!("{prefix}│   ")
            };

            for (i, dep) in deps.iter().enumerate() {
                let is_last_dep = i == deps.len() - 1;
                let dep_marker = match dep.dep_type {
                    DependencyType::Build => "[build]",
                    DependencyType::Runtime => "[runtime]",
                };
                
                let dep_node = format!("{} {}", dep.target, dep_marker);
                self.format_node(output, &dep_node, &child_prefix, is_last_dep, visited);
            }
        }

        visited.remove(node);
    }

    /// Format as DOT graph
    pub fn format_dot(&self) -> String {
        let mut output = String::new();
        output.push_str("digraph dependencies {\n");
        output.push_str("    rankdir=TB;\n");
        output.push_str("    node [shape=box];\n");
        output.push('\n');

        // Add nodes
        for pkg in &self.packages {
            output.push_str(&format!("    \"{}\";\n", pkg));
        }
        output.push('\n');

        // Add edges
        for (from, deps) in &self.dependencies {
            for dep in deps {
                let style = match dep.dep_type {
                    DependencyType::Build => "solid",
                    DependencyType::Runtime => "dashed",
                };
                output.push_str(&format!(
                    "    \"{}\" -> \"{}\" [style={}, label=\"{}\"];\n",
                    from, dep.target, style, dep.dep_type
                ));
            }
        }

        output.push_str("}\n");
        output
    }

    /// Format tree for a specific package
    pub fn format_tree_for_package(&self, package: &str) -> String {
        if !self.packages.contains(package) {
            return format!("Package '{}' not found in project", package);
        }

        let mut output = String::new();
        output.push_str(&format!("Dependencies for '{}':\n", package));

        // Check for cycles involving this package
        if let Some(cycle) = self.detect_cycles() {
            if cycle.contains(&package.to_string()) {
                output.push_str(&format!("\n⚠ Circular dependency detected: {}\n", cycle.join(" -> ")));
            }
        }

        // Format just this package as root
        self.format_node(&mut output, package, "", true, &mut HashSet::new());

        output
    }

    /// Format DOT graph for a specific package
    pub fn format_dot_for_package(&self, package: &str) -> String {
        if !self.packages.contains(package) {
            return format!("// Package '{}' not found in project", package);
        }

        let mut output = String::new();
        output.push_str(&format!("digraph \"{}\" {{\n", package));
        output.push_str("    rankdir=TB;\n");
        output.push_str("    node [shape=box];\n");
        output.push('\n');

        // Collect all packages reachable from this package
        let mut reachable = HashSet::new();
        self.collect_reachable(package, &mut reachable);

        // Add nodes
        for pkg in &reachable {
            output.push_str(&format!("    \"{}\";\n", pkg));
        }
        output.push('\n');

        // Add edges only for reachable packages
        for from in &reachable {
            if let Some(deps) = self.dependencies.get(from) {
                for dep in deps {
                    let style = match dep.dep_type {
                        DependencyType::Build => "solid",
                        DependencyType::Runtime => "dashed",
                    };
                    output.push_str(&format!(
                        "    \"{}\" -> \"{}\" [style={}, label=\"{}\"];\n",
                        from, dep.target, style, dep.dep_type
                    ));
                }
            }
        }

        output.push_str("}\n");
        output
    }

    /// Collect all packages reachable from a given package
    fn collect_reachable(&self, package: &str, reachable: &mut HashSet<String>) {
        if reachable.contains(package) {
            return;
        }
        reachable.insert(package.to_string());

        if let Some(deps) = self.dependencies.get(package) {
            for dep in deps {
                self.collect_reachable(&dep.target, reachable);
            }
        }
    }
}

/// Display dependency tree for a project
pub fn display_tree(project_dir: &Path, package: Option<&str>, graph_format: bool) -> Result<String, ZigrootError> {
    let manifest_path = project_dir.join("zigroot.toml");

    if !manifest_path.exists() {
        return Err(ZigrootError::ManifestNotFound {
            path: manifest_path.display().to_string(),
        });
    }

    let manifest_content = std::fs::read_to_string(&manifest_path)
        .map_err(|e| ZigrootError::Io { source: e })?;

    let manifest = Manifest::from_toml(&manifest_content)
        .map_err(|e| ZigrootError::ManifestParse { source: e })?;

    let tree = DependencyTree::from_manifest(&manifest);

    // If a specific package is requested, filter the tree
    if let Some(pkg_name) = package {
        if !tree.packages().contains(pkg_name) {
            return Err(ZigrootError::Package(crate::error::PackageError::NotFound {
                name: pkg_name.to_string(),
            }));
        }
        
        if graph_format {
            Ok(tree.format_dot_for_package(pkg_name))
        } else {
            Ok(tree.format_tree_for_package(pkg_name))
        }
    } else if graph_format {
        Ok(tree.format_dot())
    } else {
        Ok(tree.format_tree())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_tree() {
        let tree = DependencyTree::new();
        assert!(tree.is_empty());
        assert_eq!(tree.format_tree(), "No packages in project");
    }

    #[test]
    fn test_tree_with_packages() {
        let mut tree = DependencyTree::new();
        tree.packages.insert("app".to_string());
        tree.packages.insert("lib".to_string());
        tree.roots.push("app".to_string());
        tree.dependencies.insert("app".to_string(), vec![
            DependencyEdge {
                target: "lib".to_string(),
                dep_type: DependencyType::Build,
            }
        ]);
        tree.dependencies.insert("lib".to_string(), Vec::new());

        let output = tree.format_tree();
        assert!(output.contains("app"));
        assert!(output.contains("lib"));
        assert!(output.contains("[build]"));
    }

    #[test]
    fn test_dot_format() {
        let mut tree = DependencyTree::new();
        tree.packages.insert("app".to_string());
        tree.packages.insert("lib".to_string());
        tree.roots.push("app".to_string());
        tree.dependencies.insert("app".to_string(), vec![
            DependencyEdge {
                target: "lib".to_string(),
                dep_type: DependencyType::Build,
            }
        ]);

        let output = tree.format_dot();
        assert!(output.contains("digraph"));
        assert!(output.contains("app"));
        assert!(output.contains("lib"));
        assert!(output.contains("->"));
    }

    #[test]
    fn test_cycle_detection() {
        let mut tree = DependencyTree::new();
        tree.packages.insert("a".to_string());
        tree.packages.insert("b".to_string());
        tree.packages.insert("c".to_string());
        tree.dependencies.insert("a".to_string(), vec![
            DependencyEdge { target: "b".to_string(), dep_type: DependencyType::Build }
        ]);
        tree.dependencies.insert("b".to_string(), vec![
            DependencyEdge { target: "c".to_string(), dep_type: DependencyType::Build }
        ]);
        tree.dependencies.insert("c".to_string(), vec![
            DependencyEdge { target: "a".to_string(), dep_type: DependencyType::Build }
        ]);

        let cycle = tree.detect_cycles();
        assert!(cycle.is_some());
    }

    #[test]
    fn test_no_cycle() {
        let mut tree = DependencyTree::new();
        tree.packages.insert("a".to_string());
        tree.packages.insert("b".to_string());
        tree.packages.insert("c".to_string());
        tree.dependencies.insert("a".to_string(), vec![
            DependencyEdge { target: "b".to_string(), dep_type: DependencyType::Build }
        ]);
        tree.dependencies.insert("b".to_string(), vec![
            DependencyEdge { target: "c".to_string(), dep_type: DependencyType::Build }
        ]);
        tree.dependencies.insert("c".to_string(), Vec::new());

        let cycle = tree.detect_cycles();
        assert!(cycle.is_none());
    }

    #[test]
    fn test_distinguishes_dependency_types() {
        let mut tree = DependencyTree::new();
        tree.packages.insert("app".to_string());
        tree.packages.insert("build-lib".to_string());
        tree.packages.insert("runtime-lib".to_string());
        tree.roots.push("app".to_string());
        tree.dependencies.insert("app".to_string(), vec![
            DependencyEdge {
                target: "build-lib".to_string(),
                dep_type: DependencyType::Build,
            },
            DependencyEdge {
                target: "runtime-lib".to_string(),
                dep_type: DependencyType::Runtime,
            },
        ]);

        let output = tree.format_tree();
        assert!(output.contains("[build]"));
        assert!(output.contains("[runtime]"));

        let dot_output = tree.format_dot();
        assert!(output.contains("build"));
        assert!(dot_output.contains("solid"));
        assert!(dot_output.contains("dashed"));
    }

    #[test]
    fn test_format_tree_for_package() {
        let mut tree = DependencyTree::new();
        tree.packages.insert("app".to_string());
        tree.packages.insert("lib".to_string());
        tree.packages.insert("other".to_string());
        tree.roots.push("app".to_string());
        tree.roots.push("other".to_string());
        tree.dependencies.insert("app".to_string(), vec![
            DependencyEdge {
                target: "lib".to_string(),
                dep_type: DependencyType::Build,
            }
        ]);
        tree.dependencies.insert("lib".to_string(), Vec::new());
        tree.dependencies.insert("other".to_string(), Vec::new());

        let output = tree.format_tree_for_package("app");
        assert!(output.contains("app"));
        assert!(output.contains("lib"));
        assert!(output.contains("Dependencies for 'app'"));
    }

    #[test]
    fn test_format_tree_for_nonexistent_package() {
        let tree = DependencyTree::new();
        let output = tree.format_tree_for_package("nonexistent");
        assert!(output.contains("not found"));
    }

    #[test]
    fn test_format_dot_for_package() {
        let mut tree = DependencyTree::new();
        tree.packages.insert("app".to_string());
        tree.packages.insert("lib".to_string());
        tree.packages.insert("other".to_string());
        tree.roots.push("app".to_string());
        tree.roots.push("other".to_string());
        tree.dependencies.insert("app".to_string(), vec![
            DependencyEdge {
                target: "lib".to_string(),
                dep_type: DependencyType::Build,
            }
        ]);
        tree.dependencies.insert("lib".to_string(), Vec::new());
        tree.dependencies.insert("other".to_string(), Vec::new());

        let output = tree.format_dot_for_package("app");
        assert!(output.contains("digraph"));
        assert!(output.contains("app"));
        assert!(output.contains("lib"));
        // Should not contain "other" since it's not reachable from "app"
        assert!(!output.contains("other"));
    }

    #[test]
    fn test_format_dot_for_nonexistent_package() {
        let tree = DependencyTree::new();
        let output = tree.format_dot_for_package("nonexistent");
        assert!(output.contains("not found"));
    }

    #[test]
    fn test_collect_reachable() {
        let mut tree = DependencyTree::new();
        tree.packages.insert("a".to_string());
        tree.packages.insert("b".to_string());
        tree.packages.insert("c".to_string());
        tree.packages.insert("d".to_string());
        tree.dependencies.insert("a".to_string(), vec![
            DependencyEdge { target: "b".to_string(), dep_type: DependencyType::Build }
        ]);
        tree.dependencies.insert("b".to_string(), vec![
            DependencyEdge { target: "c".to_string(), dep_type: DependencyType::Build }
        ]);
        tree.dependencies.insert("c".to_string(), Vec::new());
        tree.dependencies.insert("d".to_string(), Vec::new());

        let mut reachable = HashSet::new();
        tree.collect_reachable("a", &mut reachable);

        assert!(reachable.contains("a"));
        assert!(reachable.contains("b"));
        assert!(reachable.contains("c"));
        assert!(!reachable.contains("d")); // d is not reachable from a
    }
}
