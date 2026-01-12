//! CLI command implementation for `zigroot external`
//!
//! Manages external artifacts like bootloaders, kernels, partition tables,
//! DTBs, and firmware files.
//!
//! **Validates: Requirements 8.1, 8.2, 8.9-8.13**

use crate::core::external::{self, ArtifactStatus};
use anyhow::Result;
use std::path::Path;

/// Execute the `zigroot external list` command
///
/// Lists all configured external artifacts and their status.
///
/// **Validates: Requirement 8.9**
pub async fn execute_list(project_dir: &Path) -> Result<()> {
    let artifacts = external::list_artifacts(project_dir)?;

    if artifacts.is_empty() {
        println!("No external artifacts configured.");
        return Ok(());
    }

    println!("External Artifacts:");
    println!();

    for artifact in &artifacts {
        let status_icon = match artifact.status {
            ArtifactStatus::Local => "✓",
            ArtifactStatus::Downloaded => "✓",
            ArtifactStatus::Missing => "✗",
        };

        let status_text = match artifact.status {
            ArtifactStatus::Local => "local",
            ArtifactStatus::Downloaded => "downloaded",
            ArtifactStatus::Missing => "missing",
        };

        println!(
            "  {} {} [{}] - {}",
            status_icon, artifact.name, artifact.artifact_type, status_text
        );

        if let Some(ref url) = artifact.url {
            println!("      URL: {url}");
        }
        if let Some(ref path) = artifact.path {
            println!("      Path: {path}");
        }
        if let Some(ref format) = artifact.format {
            println!("      Format: {format}");
        }
    }

    Ok(())
}

/// Execute the `zigroot external add` command
///
/// Adds an external artifact to the manifest.
///
/// **Validates: Requirements 8.10, 8.11**
pub async fn execute_add(
    project_dir: &Path,
    name: &str,
    artifact_type: &str,
    url: Option<&str>,
    path: Option<&str>,
) -> Result<()> {
    external::add_artifact(project_dir, name, artifact_type, url, path)?;

    println!("✓ Added external artifact '{name}' ({artifact_type})");

    if let Some(url) = url {
        println!("  URL: {url}");
        if path.is_none() {
            println!("  Note: Consider adding a sha256 checksum for verification");
        }
    }
    if let Some(path) = path {
        println!("  Path: {path}");
    }

    Ok(())
}
