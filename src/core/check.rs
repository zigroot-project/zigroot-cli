//! Check command logic
//!
//! Validates configuration, checks dependencies, verifies toolchains,
//! and reports what would be built without actually building.
//!
//! **Validates: Requirements 4.13**

use std::collections::HashSet;
use std::path::Path;

use crate::core::manifest::Manifest;
use crate::core::package::PackageDefinition;
use crate::core::resolver::DependencyGraph;
use crate::error::ZigrootError;

/// Result of the check operation
#[derive(Debug)]
pub struct CheckResult {
    /// Whether the configuration is valid
    pub config_valid: bool,
    /// Whether all dependencies are resolvable
    pub dependencies_valid: bool,
    /// Whether toolchains are available
    pub toolchains_available: bool,
    /// Packages that would be built
    pub packages_to_build: Vec<String>,
    /// Build order (topologically sorted)
    pub build_order: Vec<String>,
    /// Warnings encountered during check
    pub warnings: Vec<String>,
    /// Missing dependencies (if any)
    pub missing_dependencies: Vec<String>,
}

impl CheckResult {
    /// Create a new check result
    pub fn new() -> Self {
        Self {
            config_valid: true,
            dependencies_valid: true,
            toolchains_available: true,
            packages_to_build: Vec::new(),
            build_order: Vec::new(),
            warnings: Vec::new(),
            missing_dependencies: Vec::new(),
        }
    }

    /// Check if all validations passed
    pub fn is_valid(&self) -> bool {
        self.config_valid && self.dependencies_valid
    }
}

impl Default for CheckResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Perform check operation on a project
///
/// This validates the configuration, checks dependencies, verifies toolchains,
/// and reports what would be built without actually building.
pub fn check(project_dir: &Path, manifest: &Manifest) -> Result<CheckResult, ZigrootError> {
    let mut result = CheckResult::new();

    // Collect packages to build
    result.packages_to_build = manifest.packages.keys().cloned().collect();

    // Check for local packages and load their definitions
    let packages_dir = project_dir.join("packages");
    let mut dependency_graph = DependencyGraph::new();
    let mut all_dependencies: HashSet<String> = HashSet::new();

    for pkg_name in &result.packages_to_build {
        let local_pkg_path = packages_dir.join(pkg_name).join("package.toml");

        if local_pkg_path.exists() {
            // Load local package definition to check dependencies
            match std::fs::read_to_string(&local_pkg_path) {
                Ok(content) => {
                    match PackageDefinition::from_toml(&content) {
                        Ok(pkg_def) => {
                            // Add package to dependency graph
                            let deps: Vec<String> = pkg_def.package.depends.clone();

                            for dep in &deps {
                                all_dependencies.insert(dep.clone());
                            }

                            dependency_graph.add_package(pkg_name, deps);
                        }
                        Err(e) => {
                            result.warnings.push(format!(
                                "Failed to parse package definition for '{pkg_name}': {e}"
                            ));
                        }
                    }
                }
                Err(e) => {
                    result.warnings.push(format!(
                        "Failed to read package definition for '{pkg_name}': {e}"
                    ));
                }
            }
        } else {
            // Registry package - add with no dependencies for now
            dependency_graph.add_package(pkg_name, vec![]);
        }
    }

    // Check for missing dependencies
    let known_packages: HashSet<String> = result.packages_to_build.iter().cloned().collect();
    for dep in &all_dependencies {
        if !known_packages.contains(dep) {
            // Check if it's a local package
            let local_dep_path = packages_dir.join(dep).join("package.toml");
            if !local_dep_path.exists() {
                result.missing_dependencies.push(dep.clone());
            }
        }
    }

    if !result.missing_dependencies.is_empty() {
        result.dependencies_valid = false;
        result.warnings.push(format!(
            "Missing dependencies: {}",
            result.missing_dependencies.join(", ")
        ));
    }

    // Compute build order
    match dependency_graph.topological_sort() {
        Ok(order) => {
            result.build_order = order;
        }
        Err(e) => {
            result.dependencies_valid = false;
            result.warnings.push(format!("Dependency resolution failed: {e}"));
        }
    }

    // Check toolchain availability
    result.toolchains_available = check_toolchain_availability();
    if !result.toolchains_available {
        result.warnings.push("Zig toolchain not found in PATH".to_string());
    }

    // Validate external artifacts
    for (name, artifact) in &manifest.external {
        if artifact.url.is_some() && artifact.sha256.is_none() {
            result.warnings.push(format!(
                "External artifact '{name}' has URL but no sha256 checksum"
            ));
        }
    }

    Ok(result)
}

/// Check if the Zig toolchain is available
fn check_toolchain_availability() -> bool {
    which::which("zig").is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::manifest::{BoardConfig, BuildConfig, ProjectConfig};
    use std::collections::HashMap;
    use tempfile::TempDir;

    fn create_test_manifest() -> Manifest {
        Manifest {
            project: ProjectConfig {
                name: "test-project".to_string(),
                version: "1.0.0".to_string(),
                description: None,
            },
            board: BoardConfig::default(),
            build: BuildConfig::default(),
            packages: HashMap::new(),
            external: HashMap::new(),
        }
    }

    #[test]
    fn test_check_empty_project() {
        let temp_dir = TempDir::new().unwrap();
        let manifest = create_test_manifest();

        let result = check(temp_dir.path(), &manifest).unwrap();

        assert!(result.config_valid);
        assert!(result.dependencies_valid);
        assert!(result.packages_to_build.is_empty());
    }

    #[test]
    fn test_check_result_is_valid() {
        let result = CheckResult::new();
        assert!(result.is_valid());
    }

    #[test]
    fn test_check_result_invalid_when_deps_fail() {
        let mut result = CheckResult::new();
        result.dependencies_valid = false;
        assert!(!result.is_valid());
    }
}
