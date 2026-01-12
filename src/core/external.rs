//! External artifact management logic
//!
//! This module handles external artifacts like bootloaders, kernels,
//! partition tables, DTBs, and firmware files.
//!
//! **Validates: Requirements 8.1, 8.2, 8.9-8.13**

use crate::core::manifest::{ExternalArtifact, Manifest};
use anyhow::{Context, Result};
use std::path::Path;

/// Status of an external artifact
#[derive(Debug, Clone, PartialEq)]
pub enum ArtifactStatus {
    /// Artifact is present locally
    Local,
    /// Artifact has been downloaded
    Downloaded,
    /// Artifact is missing (needs download or local file doesn't exist)
    Missing,
}

impl std::fmt::Display for ArtifactStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Local => write!(f, "local"),
            Self::Downloaded => write!(f, "downloaded"),
            Self::Missing => write!(f, "missing"),
        }
    }
}

/// Information about an external artifact for display
#[derive(Debug, Clone)]
pub struct ArtifactInfo {
    /// Artifact name
    pub name: String,
    /// Artifact type
    pub artifact_type: String,
    /// URL if remote
    pub url: Option<String>,
    /// Local path
    pub path: Option<String>,
    /// SHA256 checksum
    pub sha256: Option<String>,
    /// Partition table format (if applicable)
    pub format: Option<String>,
    /// Current status
    pub status: ArtifactStatus,
}

/// List all external artifacts from the manifest
///
/// Returns information about each artifact including its status.
///
/// **Validates: Requirement 8.9**
pub fn list_artifacts(project_dir: &Path) -> Result<Vec<ArtifactInfo>> {
    let manifest_path = project_dir.join("zigroot.toml");
    let manifest_content = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("Failed to read manifest at {}", manifest_path.display()))?;

    let manifest = Manifest::from_toml(&manifest_content)
        .with_context(|| "Failed to parse manifest")?;

    let mut artifacts = Vec::new();

    for (name, artifact) in &manifest.external {
        let status = determine_artifact_status(project_dir, artifact);

        artifacts.push(ArtifactInfo {
            name: name.clone(),
            artifact_type: artifact.artifact_type.clone(),
            url: artifact.url.clone(),
            path: artifact.path.clone(),
            sha256: artifact.sha256.clone(),
            format: artifact.format.clone(),
            status,
        });
    }

    // Sort by name for consistent output
    artifacts.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(artifacts)
}

/// Determine the status of an artifact
fn determine_artifact_status(project_dir: &Path, artifact: &ExternalArtifact) -> ArtifactStatus {
    // If there's a local path, check if the file exists
    if let Some(ref path) = artifact.path {
        let full_path = project_dir.join(path);
        if full_path.exists() {
            // If it has a URL, it was downloaded; otherwise it's local
            if artifact.url.is_some() {
                return ArtifactStatus::Downloaded;
            }
            return ArtifactStatus::Local;
        }
    }

    // If there's only a URL and no path, check the default download location
    if artifact.url.is_some() && artifact.path.is_none() {
        // Default download location would be in external/ directory
        // For now, assume it's missing if no path is specified
        return ArtifactStatus::Missing;
    }

    ArtifactStatus::Missing
}

/// Valid artifact types
pub const VALID_ARTIFACT_TYPES: &[&str] = &[
    "bootloader",
    "kernel",
    "partition_table",
    "dtb",
    "firmware",
    "other",
];

/// Add an external artifact to the manifest
///
/// **Validates: Requirements 8.10, 8.11**
pub fn add_artifact(
    project_dir: &Path,
    name: &str,
    artifact_type: &str,
    url: Option<&str>,
    path: Option<&str>,
) -> Result<()> {
    // Validate artifact type
    if !VALID_ARTIFACT_TYPES.contains(&artifact_type) {
        anyhow::bail!(
            "Invalid artifact type '{}'. Valid types are: {}",
            artifact_type,
            VALID_ARTIFACT_TYPES.join(", ")
        );
    }

    // At least one of url or path must be specified
    if url.is_none() && path.is_none() {
        anyhow::bail!("Either --url or --path must be specified for external artifact");
    }

    let manifest_path = project_dir.join("zigroot.toml");
    let manifest_content = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("Failed to read manifest at {}", manifest_path.display()))?;

    let mut manifest = Manifest::from_toml(&manifest_content)
        .with_context(|| "Failed to parse manifest")?;

    // Create the new artifact
    let artifact = ExternalArtifact {
        artifact_type: artifact_type.to_string(),
        url: url.map(String::from),
        path: path.map(String::from),
        sha256: None, // User should add this manually for URL sources
        format: None,
    };

    // Add to manifest
    manifest.external.insert(name.to_string(), artifact);

    // Write back to file
    let new_content = manifest.to_toml()
        .with_context(|| "Failed to serialize manifest")?;

    std::fs::write(&manifest_path, new_content)
        .with_context(|| format!("Failed to write manifest at {}", manifest_path.display()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_project() -> TempDir {
        let dir = TempDir::new().unwrap();
        let manifest = r#"
[project]
name = "test-project"
version = "1.0.0"

[board]
name = "test-board"

[build]
image_format = "ext4"
rootfs_size = "64M"
hostname = "test"
"#;
        std::fs::write(dir.path().join("zigroot.toml"), manifest).unwrap();
        dir
    }

    #[test]
    fn test_list_artifacts_empty() {
        let dir = create_test_project();
        let artifacts = list_artifacts(dir.path()).unwrap();
        assert!(artifacts.is_empty());
    }

    #[test]
    fn test_add_artifact_url() {
        let dir = create_test_project();
        add_artifact(
            dir.path(),
            "bootloader",
            "bootloader",
            Some("https://example.com/boot.bin"),
            None,
        )
        .unwrap();

        let artifacts = list_artifacts(dir.path()).unwrap();
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].name, "bootloader");
        assert_eq!(artifacts[0].artifact_type, "bootloader");
        assert!(artifacts[0].url.is_some());
    }

    #[test]
    fn test_add_artifact_path() {
        let dir = create_test_project();
        add_artifact(
            dir.path(),
            "kernel",
            "kernel",
            None,
            Some("external/kernel.img"),
        )
        .unwrap();

        let artifacts = list_artifacts(dir.path()).unwrap();
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].name, "kernel");
        assert_eq!(artifacts[0].artifact_type, "kernel");
        assert!(artifacts[0].path.is_some());
    }

    #[test]
    fn test_add_artifact_invalid_type() {
        let dir = create_test_project();
        let result = add_artifact(
            dir.path(),
            "test",
            "invalid_type",
            Some("https://example.com/test.bin"),
            None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_add_artifact_requires_url_or_path() {
        let dir = create_test_project();
        let result = add_artifact(dir.path(), "test", "bootloader", None, None);
        assert!(result.is_err());
    }
}
