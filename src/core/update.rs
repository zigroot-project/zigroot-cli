//! Package update logic
//!
//! This module contains the business logic for updating packages in a project.
//! It handles checking for newer versions and updating the lock file.

use std::path::Path;

use crate::core::lock::{LockFile, LockedPackageBuilder};
use crate::core::manifest::Manifest;
use crate::registry::client::RegistryClient;
use thiserror::Error;

/// Errors that can occur during package update
#[derive(Error, Debug)]
pub enum UpdateError {
    /// Package not found in manifest
    #[error("Package '{name}' is not installed")]
    PackageNotFound { name: String },

    /// Manifest error
    #[error("Failed to read/write manifest: {0}")]
    ManifestError(String),

    /// Lock file error
    #[error("Failed to read/write lock file: {0}")]
    LockError(String),

    /// Registry error
    #[error("Registry error: {0}")]
    RegistryError(String),

    /// IO error
    #[error("IO error: {0}")]
    IoError(String),

    /// No packages to update
    #[error("No packages to update")]
    NoPackages,
}

/// Result of updating packages
#[derive(Debug)]
pub struct UpdateResult {
    /// Packages that were checked
    pub checked: Vec<String>,
    /// Packages that were updated (name, `old_version`, `new_version`)
    pub updated: Vec<(String, String, String)>,
    /// Packages that are already up to date
    pub up_to_date: Vec<String>,
    /// Whether the lock file was updated
    pub lock_updated: bool,
}

/// Update packages in the project
pub async fn update_packages(
    project_path: &Path,
    package_name: Option<&str>,
) -> Result<UpdateResult, UpdateError> {
    let manifest_path = project_path.join("zigroot.toml");
    let lock_path = project_path.join("zigroot.lock");

    // Load existing manifest
    let manifest_content =
        std::fs::read_to_string(&manifest_path).map_err(|e| UpdateError::ManifestError(e.to_string()))?;
    let mut manifest =
        Manifest::from_toml(&manifest_content).map_err(|e| UpdateError::ManifestError(e.to_string()))?;

    // Determine which packages to update
    let packages_to_update: Vec<String> = if let Some(name) = package_name {
        // Update specific package
        if !manifest.packages.contains_key(name) {
            return Err(UpdateError::PackageNotFound {
                name: name.to_string(),
            });
        }
        vec![name.to_string()]
    } else {
        // Update all packages
        manifest.packages.keys().cloned().collect()
    };

    if packages_to_update.is_empty() {
        return Ok(UpdateResult {
            checked: vec![],
            updated: vec![],
            up_to_date: vec![],
            lock_updated: false,
        });
    }

    // Load or create lock file
    let mut lock_file = if lock_path.exists() {
        LockFile::load(&lock_path).map_err(|e| UpdateError::LockError(e.to_string()))?
    } else {
        LockFile::new(env!("CARGO_PKG_VERSION"), "unknown")
    };

    // Try to fetch latest versions from registry
    let client = RegistryClient::new();
    let index = client.fetch_package_index().await.ok();

    let mut result = UpdateResult {
        checked: vec![],
        updated: vec![],
        up_to_date: vec![],
        lock_updated: false,
    };

    for pkg_name in &packages_to_update {
        result.checked.push(pkg_name.clone());

        let pkg_ref = manifest.packages.get(pkg_name).unwrap();
        let current_version = pkg_ref.version.clone().unwrap_or_else(|| "latest".to_string());

        // Try to find latest version from registry
        let latest_version = if let Some(ref idx) = index {
            idx.packages
                .iter()
                .find(|p| &p.name == pkg_name)
                .map(|p| p.latest.clone())
        } else {
            None
        };

        if let Some(latest) = latest_version {
            if is_newer_version(&latest, &current_version) {
                // Update manifest with new version
                if let Some(pkg) = manifest.packages.get_mut(pkg_name) {
                    pkg.version = Some(latest.clone());
                }

                // Update lock file
                let locked_pkg = LockedPackageBuilder::new(pkg_name, &latest, "pending").build();
                lock_file.add_package(locked_pkg);

                result.updated.push((pkg_name.clone(), current_version, latest));
                result.lock_updated = true;
            } else {
                result.up_to_date.push(pkg_name.clone());
            }
        } else {
            // No registry info available, keep current version
            result.up_to_date.push(pkg_name.clone());
        }
    }

    // Save manifest if any packages were updated
    if result.lock_updated {
        let new_manifest_content =
            manifest.to_toml().map_err(|e| UpdateError::ManifestError(e.to_string()))?;
        std::fs::write(&manifest_path, new_manifest_content)
            .map_err(|e| UpdateError::IoError(e.to_string()))?;

        // Save lock file
        lock_file
            .save(&lock_path)
            .map_err(|e| UpdateError::LockError(e.to_string()))?;
    }

    Ok(result)
}

/// Compare two semver-like version strings
/// Returns true if `new_version` is newer than `current_version`
fn is_newer_version(new_version: &str, current_version: &str) -> bool {
    // Handle "latest" as always up-to-date
    if current_version == "latest" {
        return false;
    }

    // Parse versions into components
    let parse_version = |v: &str| -> Vec<u32> {
        v.split('.')
            .filter_map(|s| s.parse::<u32>().ok())
            .collect()
    };

    let new_parts = parse_version(new_version);
    let current_parts = parse_version(current_version);

    // Compare component by component
    for i in 0..new_parts.len().max(current_parts.len()) {
        let new_part = new_parts.get(i).copied().unwrap_or(0);
        let current_part = current_parts.get(i).copied().unwrap_or(0);

        if new_part > current_part {
            return true;
        } else if new_part < current_part {
            return false;
        }
    }

    false // Equal versions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_newer_version_major() {
        assert!(is_newer_version("2.0.0", "1.0.0"));
        assert!(!is_newer_version("1.0.0", "2.0.0"));
    }

    #[test]
    fn test_is_newer_version_minor() {
        assert!(is_newer_version("1.2.0", "1.1.0"));
        assert!(!is_newer_version("1.1.0", "1.2.0"));
    }

    #[test]
    fn test_is_newer_version_patch() {
        assert!(is_newer_version("1.0.2", "1.0.1"));
        assert!(!is_newer_version("1.0.1", "1.0.2"));
    }

    #[test]
    fn test_is_newer_version_equal() {
        assert!(!is_newer_version("1.0.0", "1.0.0"));
    }

    #[test]
    fn test_is_newer_version_latest() {
        assert!(!is_newer_version("2.0.0", "latest"));
    }

    #[test]
    fn test_is_newer_version_different_lengths() {
        assert!(is_newer_version("1.36.1", "1.36.0"));
        assert!(is_newer_version("1.37", "1.36.1"));
    }
}
