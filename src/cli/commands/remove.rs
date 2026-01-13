//! CLI implementation for `zigroot remove` command
//!
//! This module handles the CLI interface for removing packages from a project.

use std::path::Path;

use anyhow::{Context, Result};

use crate::core::remove::remove_package;

/// Execute the remove command
pub async fn execute(path: &Path, package: &str) -> Result<()> {
    // Check if manifest exists
    let manifest_path = path.join("zigroot.toml");
    if !manifest_path.exists() {
        anyhow::bail!(
            "No zigroot.toml found in {}. Run 'zigroot init' first.",
            path.display()
        );
    }

    let result = remove_package(path, package)
        .with_context(|| format!("Failed to remove package '{package}'"))?;

    // Print success message
    if let Some(version) = &result.version {
        println!("✓ Removed {} v{}", result.package_name, version);
    } else {
        println!("✓ Removed {}", result.package_name);
    }

    if result.lock_updated {
        println!("  Updated zigroot.lock");
    }

    Ok(())
}
