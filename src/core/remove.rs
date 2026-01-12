//! Package removal logic
//!
//! This module contains the business logic for removing packages from a project.
//! It handles removing packages from the manifest and updating the lock file.

use std::path::Path;

use crate::core::lock::LockFile;
use crate::core::manifest::Manifest;
use thiserror::Error;

/// Errors that can occur during package removal
#[derive(Error, Debug)]
pub enum RemoveError {
    /// Package not found in manifest
    #[error("Package '{name}' is not installed")]
    PackageNotFound { name: String },

    /// Manifest error
    #[error("Failed to read/write manifest: {0}")]
    ManifestError(String),

    /// Lock file error
    #[error("Failed to read/write lock file: {0}")]
    LockError(String),

    /// IO error
    #[error("IO error: {0}")]
    IoError(String),
}

/// Result of removing a package
#[derive(Debug)]
pub struct RemoveResult {
    /// Name of the removed package
    pub package_name: String,
    /// Version that was removed (if known)
    pub version: Option<String>,
    /// Whether the lock file was updated
    pub lock_updated: bool,
}

/// Remove a package from the project
pub fn remove_package(project_path: &Path, package_name: &str) -> Result<RemoveResult, RemoveError> {
    let manifest_path = project_path.join("zigroot.toml");
    let lock_path = project_path.join("zigroot.lock");

    // Load existing manifest
    let manifest_content =
        std::fs::read_to_string(&manifest_path).map_err(|e| RemoveError::ManifestError(e.to_string()))?;
    let mut manifest =
        Manifest::from_toml(&manifest_content).map_err(|e| RemoveError::ManifestError(e.to_string()))?;

    // Check if package exists in manifest
    let package_ref = manifest
        .packages
        .get(package_name)
        .ok_or_else(|| RemoveError::PackageNotFound {
            name: package_name.to_string(),
        })?;

    // Get version before removal (for reporting)
    let version = package_ref.version.clone();

    // Remove package from manifest
    manifest.packages.remove(package_name);

    // Save manifest
    let new_manifest_content =
        manifest.to_toml().map_err(|e| RemoveError::ManifestError(e.to_string()))?;
    std::fs::write(&manifest_path, new_manifest_content)
        .map_err(|e| RemoveError::IoError(e.to_string()))?;

    // Update lock file if it exists
    let lock_updated = if lock_path.exists() {
        let mut lock_file =
            LockFile::load(&lock_path).map_err(|e| RemoveError::LockError(e.to_string()))?;

        // Remove package from lock file
        lock_file.packages.retain(|p| p.name != package_name);

        // Save lock file
        lock_file
            .save(&lock_path)
            .map_err(|e| RemoveError::LockError(e.to_string()))?;

        true
    } else {
        false
    };

    Ok(RemoveResult {
        package_name: package_name.to_string(),
        version,
        lock_updated,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::manifest::{PackageRef, ProjectConfig};
    use std::collections::HashMap;
    use tempfile::TempDir;

    fn create_test_manifest(packages: Vec<(&str, &str)>) -> Manifest {
        let mut pkg_map = HashMap::new();
        for (name, version) in packages {
            pkg_map.insert(
                name.to_string(),
                PackageRef {
                    version: Some(version.to_string()),
                    git: None,
                    ref_: None,
                    registry: None,
                    options: HashMap::new(),
                },
            );
        }

        Manifest {
            project: ProjectConfig {
                name: "test-project".to_string(),
                version: "1.0.0".to_string(),
                description: None,
            },
            board: Default::default(),
            build: Default::default(),
            packages: pkg_map,
            external: HashMap::new(),
        }
    }

    #[test]
    fn test_remove_existing_package() {
        let temp = TempDir::new().unwrap();
        let manifest = create_test_manifest(vec![("busybox", "1.36.1"), ("dropbear", "2024.85")]);

        // Write manifest
        let manifest_path = temp.path().join("zigroot.toml");
        std::fs::write(&manifest_path, manifest.to_toml().unwrap()).unwrap();

        // Remove busybox
        let result = remove_package(temp.path(), "busybox").unwrap();

        assert_eq!(result.package_name, "busybox");
        assert_eq!(result.version, Some("1.36.1".to_string()));

        // Verify manifest was updated
        let updated_content = std::fs::read_to_string(&manifest_path).unwrap();
        let updated_manifest = Manifest::from_toml(&updated_content).unwrap();

        assert!(!updated_manifest.packages.contains_key("busybox"));
        assert!(updated_manifest.packages.contains_key("dropbear"));
    }

    #[test]
    fn test_remove_nonexistent_package() {
        let temp = TempDir::new().unwrap();
        let manifest = create_test_manifest(vec![("busybox", "1.36.1")]);

        // Write manifest
        let manifest_path = temp.path().join("zigroot.toml");
        std::fs::write(&manifest_path, manifest.to_toml().unwrap()).unwrap();

        // Try to remove nonexistent package
        let result = remove_package(temp.path(), "nonexistent");

        assert!(result.is_err());
        match result.unwrap_err() {
            RemoveError::PackageNotFound { name } => {
                assert_eq!(name, "nonexistent");
            }
            e => panic!("Expected PackageNotFound, got: {e:?}"),
        }
    }

    #[test]
    fn test_remove_updates_lock_file() {
        let temp = TempDir::new().unwrap();
        let manifest = create_test_manifest(vec![("busybox", "1.36.1")]);

        // Write manifest
        let manifest_path = temp.path().join("zigroot.toml");
        std::fs::write(&manifest_path, manifest.to_toml().unwrap()).unwrap();

        // Create lock file
        let lock_path = temp.path().join("zigroot.lock");
        let mut lock = LockFile::new("0.1.0", "0.13.0");
        lock.add_package(
            crate::core::lock::LockedPackageBuilder::new("busybox", "1.36.1", "abc123").build(),
        );
        lock.save(&lock_path).unwrap();

        // Remove busybox
        let result = remove_package(temp.path(), "busybox").unwrap();

        assert!(result.lock_updated);

        // Verify lock file was updated
        let updated_lock = LockFile::load(&lock_path).unwrap();
        assert!(updated_lock.get_package("busybox").is_none());
    }
}
