//! CLI implementation for `zigroot add` command
//!
//! This module handles the CLI interface for adding packages to a project.

use std::path::Path;

use anyhow::{Context, Result};

use crate::core::add::{add_package, AddOptions};

/// Execute the add command
pub async fn execute(
    path: &Path,
    package: &str,
    git: Option<String>,
    registry: Option<String>,
) -> Result<()> {
    // Check if manifest exists
    let manifest_path = path.join("zigroot.toml");
    if !manifest_path.exists() {
        anyhow::bail!(
            "No zigroot.toml found in {}. Run 'zigroot init' first.",
            path.display()
        );
    }

    let options = AddOptions { git, registry };

    let result = add_package(path, package, &options)
        .await
        .with_context(|| format!("Failed to add package '{package}'"))?;

    // Print success message
    println!("✓ Added {} v{}", result.package_name, result.version);

    if !result.dependencies.is_empty() {
        println!("  Dependencies:");
        for dep in &result.dependencies {
            println!("    + {dep}");
        }
    }

    if result.lock_updated {
        println!("  Updated zigroot.lock");
    }

    Ok(())
}
