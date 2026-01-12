//! Package addition logic
//!
//! This module contains the business logic for adding packages to a project.
//! It handles fetching from registry, git sources, and custom registries,
//! as well as resolving transitive dependencies and updating the lock file.

use std::collections::HashMap;
use std::path::Path;

use crate::core::lock::{LockFile, LockedPackage, LockedPackageBuilder};
use crate::core::manifest::{Manifest, PackageRef};
use crate::core::resolver::{detect_version_conflict, DependencyGraph};
use crate::registry::client::{PackageIndexEntry, RegistryClient};
use thiserror::Error;

/// Errors that can occur during package addition
#[derive(Error, Debug)]
pub enum AddError {
    /// Package not found in registry
    #[error("Package '{name}' not found in registry")]
    PackageNotFound { name: String },

    /// Version not found
    #[error("Version '{version}' not found for package '{package}'")]
    VersionNotFound { package: String, version: String },

    /// Manifest error
    #[error("Failed to read/write manifest: {0}")]
    ManifestError(String),

    /// Lock file error
    #[error("Failed to read/write lock file: {0}")]
    LockError(String),

    /// Registry error
    #[error("Registry error: {0}")]
    RegistryError(String),

    /// Dependency conflict
    #[error("Dependency conflict: {0}")]
    DependencyConflict(String),

    /// IO error
    #[error("IO error: {0}")]
    IoError(String),

    /// Invalid package specification
    #[error("Invalid package specification: {0}")]
    InvalidSpec(String),
}

/// Options for adding a package
#[derive(Debug, Clone, Default)]
pub struct AddOptions {
    /// Git repository URL (for --git flag)
    pub git: Option<String>,
    /// Custom registry URL (for --registry flag)
    pub registry: Option<String>,
}

/// Result of adding a package
#[derive(Debug)]
pub struct AddResult {
    /// Name of the added package
    pub package_name: String,
    /// Version that was added
    pub version: String,
    /// Transitive dependencies that were added
    pub dependencies: Vec<String>,
    /// Whether the lock file was updated
    pub lock_updated: bool,
}

/// Parse a package specification (name or name@version)
pub fn parse_package_spec(spec: &str) -> (String, Option<String>) {
    if let Some(at_pos) = spec.rfind('@') {
        let name = spec[..at_pos].to_string();
        let version = spec[at_pos + 1..].to_string();
        if !version.is_empty() {
            return (name, Some(version));
        }
    }
    (spec.to_string(), None)
}

/// Parse git URL with optional ref (url#ref)
pub fn parse_git_url(url: &str) -> (String, Option<String>) {
    if let Some(hash_pos) = url.rfind('#') {
        let base_url = url[..hash_pos].to_string();
        let git_ref = url[hash_pos + 1..].to_string();
        if !git_ref.is_empty() {
            return (base_url, Some(git_ref));
        }
    }
    (url.to_string(), None)
}

/// Add a package to the project
pub async fn add_package(
    project_path: &Path,
    package_spec: &str,
    options: &AddOptions,
) -> Result<AddResult, AddError> {
    let manifest_path = project_path.join("zigroot.toml");
    let lock_path = project_path.join("zigroot.lock");

    // Load existing manifest
    let manifest_content =
        std::fs::read_to_string(&manifest_path).map_err(|e| AddError::ManifestError(e.to_string()))?;
    let mut manifest =
        Manifest::from_toml(&manifest_content).map_err(|e| AddError::ManifestError(e.to_string()))?;

    // Parse package specification
    let (package_name, requested_version) = parse_package_spec(package_spec);

    // Determine source and create package reference
    let (package_ref, version, dependencies) = if let Some(git_url) = &options.git {
        // Git source
        let (url, git_ref) = parse_git_url(git_url);
        let pkg_ref = PackageRef {
            version: None,
            git: Some(url),
            ref_: git_ref.clone(),
            registry: None,
            options: HashMap::new(),
        };
        let ver = git_ref.unwrap_or_else(|| "HEAD".to_string());
        (pkg_ref, ver, vec![])
    } else if let Some(registry_url) = &options.registry {
        // Custom registry source
        let pkg_ref = PackageRef {
            version: requested_version.clone(),
            git: None,
            ref_: None,
            registry: Some(registry_url.clone()),
            options: HashMap::new(),
        };
        let ver = requested_version.unwrap_or_else(|| "latest".to_string());
        (pkg_ref, ver, vec![])
    } else {
        // Default registry source - try to fetch, but fall back to offline mode
        let client = RegistryClient::new();
        if let Ok((version, deps)) =
            resolve_from_registry(&client, &package_name, requested_version.as_deref(), &manifest)
                .await
        {
            let pkg_ref = PackageRef {
                version: Some(version.clone()),
                git: None,
                ref_: None,
                registry: None,
                options: HashMap::new(),
            };
            (pkg_ref, version, deps)
        } else {
            // Offline mode: add package with requested version or "latest"
            let version = requested_version.unwrap_or_else(|| "latest".to_string());
            let pkg_ref = PackageRef {
                version: Some(version.clone()),
                git: None,
                ref_: None,
                registry: None,
                options: HashMap::new(),
            };
            (pkg_ref, version, vec![])
        }
    };

    // Add package to manifest
    manifest.packages.insert(package_name.clone(), package_ref);

    // Save manifest
    let new_manifest_content =
        manifest.to_toml().map_err(|e| AddError::ManifestError(e.to_string()))?;
    std::fs::write(&manifest_path, new_manifest_content)
        .map_err(|e| AddError::IoError(e.to_string()))?;

    // Update lock file
    let mut lock_file = if lock_path.exists() {
        LockFile::load(&lock_path).map_err(|e| AddError::LockError(e.to_string()))?
    } else {
        LockFile::new(env!("CARGO_PKG_VERSION"), "unknown")
    };

    // Add the main package to lock file
    let locked_pkg = create_locked_package(&package_name, &version, options);
    lock_file.add_package(locked_pkg);

    // Add dependencies to lock file
    for dep in &dependencies {
        let (dep_name, dep_version) = parse_package_spec(dep);
        let dep_locked = LockedPackageBuilder::new(
            &dep_name,
            &dep_version.unwrap_or_else(|| "latest".to_string()),
            "pending", // Checksum will be filled during fetch
        )
        .build();
        lock_file.add_package(dep_locked);
    }

    // Save lock file
    lock_file
        .save(&lock_path)
        .map_err(|e| AddError::LockError(e.to_string()))?;

    Ok(AddResult {
        package_name,
        version,
        dependencies,
        lock_updated: true,
    })
}


/// Resolve package from registry, including transitive dependencies
async fn resolve_from_registry(
    client: &RegistryClient,
    package_name: &str,
    requested_version: Option<&str>,
    manifest: &Manifest,
) -> Result<(String, Vec<String>), AddError> {
    // Fetch package index
    let index = client
        .fetch_package_index()
        .await
        .map_err(|e| AddError::RegistryError(e.to_string()))?;

    // Find package in index
    let package_entry = index
        .packages
        .iter()
        .find(|p| p.name == package_name)
        .ok_or_else(|| AddError::PackageNotFound {
            name: package_name.to_string(),
        })?;

    // Determine version to use
    let version = if let Some(req_ver) = requested_version {
        // Check if requested version exists
        if !package_entry.versions.iter().any(|v| v.version == req_ver) {
            return Err(AddError::VersionNotFound {
                package: package_name.to_string(),
                version: req_ver.to_string(),
            });
        }
        req_ver.to_string()
    } else {
        // Use latest version
        package_entry.latest.clone()
    };

    // Resolve transitive dependencies
    let dependencies = resolve_dependencies(client, package_entry, &version, manifest).await?;

    Ok((version, dependencies))
}

/// Resolve transitive dependencies for a package
async fn resolve_dependencies(
    client: &RegistryClient,
    package_entry: &PackageIndexEntry,
    _version: &str,
    manifest: &Manifest,
) -> Result<Vec<String>, AddError> {
    let mut dependencies = Vec::new();
    let mut graph = DependencyGraph::new();

    // Fetch package metadata to get dependencies
    let metadata = client
        .fetch_package_metadata(&package_entry.name)
        .await
        .map_err(|e| AddError::RegistryError(e.to_string()))?;

    // Extract dependencies from metadata
    let deps = extract_dependencies(&metadata);

    // Add to graph
    graph.add_package(&package_entry.name, deps.clone());

    // Check for conflicts with existing packages
    for dep in &deps {
        let (dep_name, dep_constraint) = parse_dependency_constraint(dep);

        // Check if this dependency conflicts with existing packages
        if let Some(existing) = manifest.packages.get(&dep_name) {
            if let Some(existing_version) = &existing.version {
                // Check version compatibility
                let constraints = vec![
                    dep_constraint.unwrap_or_else(|| "*".to_string()),
                    format!("={existing_version}"),
                ];
                let available = vec![existing_version.clone()];

                if detect_version_conflict(&dep_name, &constraints, &available).is_err() {
                    return Err(AddError::DependencyConflict(format!(
                        "Package '{}' requires '{}' but version '{}' is already installed",
                        package_entry.name, dep, existing_version
                    )));
                }
            }
        }

        dependencies.push(dep.clone());
    }

    // Recursively resolve transitive dependencies
    for dep in deps {
        let (dep_name, _) = parse_dependency_constraint(&dep);

        // Skip if already in manifest
        if manifest.packages.contains_key(&dep_name) {
            continue;
        }

        // Fetch dependency's dependencies
        if let Ok(dep_metadata) = client.fetch_package_metadata(&dep_name).await {
            let transitive_deps = extract_dependencies(&dep_metadata);
            for trans_dep in transitive_deps {
                if !dependencies.contains(&trans_dep) {
                    dependencies.push(trans_dep);
                }
            }
        }
    }

    Ok(dependencies)
}

/// Extract dependencies from package metadata
fn extract_dependencies(metadata: &toml::Value) -> Vec<String> {
    let mut deps = Vec::new();

    // Check for depends array in package section
    if let Some(package) = metadata.get("package") {
        if let Some(depends) = package.get("depends") {
            if let Some(arr) = depends.as_array() {
                for dep in arr {
                    if let Some(s) = dep.as_str() {
                        deps.push(s.to_string());
                    }
                }
            }
        }
    }

    // Also check for dependencies in build section
    if let Some(build) = metadata.get("build") {
        if let Some(depends) = build.get("depends") {
            if let Some(arr) = depends.as_array() {
                for dep in arr {
                    if let Some(s) = dep.as_str() {
                        if !deps.contains(&s.to_string()) {
                            deps.push(s.to_string());
                        }
                    }
                }
            }
        }
    }

    deps
}

/// Parse a dependency constraint (e.g., "zlib>=1.2.0" -> ("zlib", Some(">=1.2.0")))
fn parse_dependency_constraint(dep: &str) -> (String, Option<String>) {
    // Find first version constraint character
    let constraint_chars = ['>', '<', '=', '^', '~'];
    if let Some(pos) = dep.find(|c| constraint_chars.contains(&c)) {
        let name = dep[..pos].to_string();
        let constraint = dep[pos..].to_string();
        (name, Some(constraint))
    } else {
        (dep.to_string(), None)
    }
}

/// Create a locked package entry
fn create_locked_package(name: &str, version: &str, options: &AddOptions) -> LockedPackage {
    let mut builder = LockedPackageBuilder::new(name, version, "pending");

    if let Some(git_url) = &options.git {
        let (url, git_ref) = parse_git_url(git_url);
        let source = format!("git:{}#{}", url, git_ref.unwrap_or_else(|| "HEAD".to_string()));
        builder = builder.source(&source);
    } else if let Some(registry_url) = &options.registry {
        builder = builder.source(&format!("registry:{registry_url}"));
    }

    builder.build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_package_spec_name_only() {
        let (name, version) = parse_package_spec("busybox");
        assert_eq!(name, "busybox");
        assert_eq!(version, None);
    }

    #[test]
    fn test_parse_package_spec_with_version() {
        let (name, version) = parse_package_spec("busybox@1.36.1");
        assert_eq!(name, "busybox");
        assert_eq!(version, Some("1.36.1".to_string()));
    }

    #[test]
    fn test_parse_package_spec_empty_version() {
        let (name, version) = parse_package_spec("busybox@");
        assert_eq!(name, "busybox@");
        assert_eq!(version, None);
    }

    #[test]
    fn test_parse_git_url_with_ref() {
        let (url, git_ref) = parse_git_url("https://github.com/example/repo#v1.0.0");
        assert_eq!(url, "https://github.com/example/repo");
        assert_eq!(git_ref, Some("v1.0.0".to_string()));
    }

    #[test]
    fn test_parse_git_url_without_ref() {
        let (url, git_ref) = parse_git_url("https://github.com/example/repo");
        assert_eq!(url, "https://github.com/example/repo");
        assert_eq!(git_ref, None);
    }

    #[test]
    fn test_parse_dependency_constraint_with_version() {
        let (name, constraint) = parse_dependency_constraint("zlib>=1.2.0");
        assert_eq!(name, "zlib");
        assert_eq!(constraint, Some(">=1.2.0".to_string()));
    }

    #[test]
    fn test_parse_dependency_constraint_without_version() {
        let (name, constraint) = parse_dependency_constraint("zlib");
        assert_eq!(name, "zlib");
        assert_eq!(constraint, None);
    }

    #[test]
    fn test_extract_dependencies_from_metadata() {
        let metadata: toml::Value = toml::from_str(
            r#"
            [package]
            name = "nginx"
            depends = ["zlib", "openssl>=1.1.0"]
            "#,
        )
        .unwrap();

        let deps = extract_dependencies(&metadata);
        assert_eq!(deps.len(), 2);
        assert!(deps.contains(&"zlib".to_string()));
        assert!(deps.contains(&"openssl>=1.1.0".to_string()));
    }

    #[test]
    fn test_extract_dependencies_empty() {
        let metadata: toml::Value = toml::from_str(
            r#"
            [package]
            name = "standalone"
            "#,
        )
        .unwrap();

        let deps = extract_dependencies(&metadata);
        assert!(deps.is_empty());
    }
}
