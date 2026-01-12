//! CLI implementation for `zigroot clean` command
//!
//! This module handles the CLI interface for cleaning build artifacts.
//!
//! **Validates: Requirement 4.5**

use std::path::Path;

use anyhow::{Context, Result};

use crate::core::clean::{clean_project, has_build_artifacts};
use crate::core::manifest::Manifest;

/// Execute the clean command
pub async fn execute(path: &Path) -> Result<()> {
    // Verify we're in a zigroot project
    let manifest_path = path.join("zigroot.toml");
    if !manifest_path.exists() {
        anyhow::bail!(
            "No zigroot.toml found in {}. Run 'zigroot init' to create a project.",
            path.display()
        );
    }

    // Validate manifest is readable (basic check)
    let manifest_content = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("Failed to read manifest from {}", manifest_path.display()))?;
    let _manifest = Manifest::from_toml(&manifest_content)
        .with_context(|| format!("Failed to parse manifest from {}", manifest_path.display()))?;

    // Check if there's anything to clean
    if !has_build_artifacts(path) {
        println!("✓ Nothing to clean");
        return Ok(());
    }

    // Perform the clean
    let result = clean_project(path).with_context(|| "Failed to clean build artifacts")?;

    // Report what was cleaned
    if result.removed.is_empty() {
        println!("✓ Nothing to clean");
    } else {
        println!("✓ Cleaned build artifacts:");
        for dir in &result.removed {
            println!("  Removed {dir}/");
        }
    }

    Ok(())
}
