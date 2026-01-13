//! CLI implementation for `zigroot add` command
//!
//! This module handles the CLI interface for adding packages to a project.

use std::path::Path;

use anyhow::{Context, Result};

use crate::cli::output::{print_detail, print_success};
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
    print_success(&format!("Added {} v{}", result.package_name, result.version));

    if !result.dependencies.is_empty() {
        print_detail("Dependencies:");
        for dep in &result.dependencies {
            print_detail(&format!("  + {dep}"));
        }
    }

    if result.lock_updated {
        print_detail("Updated zigroot.lock");
    }

    Ok(())
}
