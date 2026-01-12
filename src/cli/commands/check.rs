//! Check command implementation
//!
//! Implements `zigroot check` to validate configuration without building.
//!
//! **Validates: Requirements 4.13**

use anyhow::{bail, Context, Result};
use std::fs;
use std::path::Path;

use crate::core::check;
use crate::core::manifest::Manifest;

/// Execute the check command
pub async fn execute(project_dir: &Path) -> Result<()> {
    let manifest_path = project_dir.join("zigroot.toml");

    // Check manifest exists
    if !manifest_path.exists() {
        bail!("No zigroot.toml found. Run 'zigroot init' to create a project.");
    }

    // Load and validate manifest
    let manifest_content = fs::read_to_string(&manifest_path)
        .with_context(|| format!("Failed to read manifest at {}", manifest_path.display()))?;

    let manifest =
        Manifest::from_toml(&manifest_content).with_context(|| "Failed to parse zigroot.toml")?;

    tracing::info!("Checking project: {}", manifest.project.name);

    // Perform check
    let result = check::check(project_dir, &manifest)
        .map_err(|e| anyhow::anyhow!("Check failed: {}", e))?;

    // Display results
    println!("Checking project configuration...\n");

    // Configuration status
    if result.config_valid {
        println!("✓ Configuration is valid");
    } else {
        println!("✗ Configuration has errors");
    }

    // Dependencies status
    if result.dependencies_valid {
        println!("✓ All dependencies are resolvable");
    } else {
        println!("✗ Dependency issues found");
        for dep in &result.missing_dependencies {
            println!("  - Missing dependency: {dep}");
        }
    }

    // Toolchain status
    if result.toolchains_available {
        println!("✓ Zig toolchain is available");
    } else {
        println!("⚠ Zig toolchain not found in PATH");
    }

    // Display warnings
    if !result.warnings.is_empty() {
        println!("\nWarnings:");
        for warning in &result.warnings {
            println!("  ⚠ {warning}");
        }
    }

    // Display what would be built
    println!("\nPackages that would be built:");
    if result.packages_to_build.is_empty() {
        println!("  (none)");
    } else {
        for pkg in &result.build_order {
            if result.packages_to_build.contains(pkg) {
                println!("  • {pkg}");
            }
        }
        // Also show packages not in build order (e.g., if dependency resolution failed)
        for pkg in &result.packages_to_build {
            if !result.build_order.contains(pkg) {
                println!("  • {pkg}");
            }
        }
    }

    // Board info
    if let Some(board_name) = &manifest.board.name {
        println!("\nTarget board: {board_name}");
    }

    // Build settings
    println!("\nBuild settings:");
    println!("  Image format: {}", manifest.build.image_format);
    println!("  Rootfs size: {}", manifest.build.rootfs_size);
    println!("  Hostname: {}", manifest.build.hostname);
    println!(
        "  Compression: {}",
        if manifest.build.compress {
            "enabled"
        } else {
            "disabled"
        }
    );

    // External artifacts
    if !manifest.external.is_empty() {
        println!("\nExternal artifacts:");
        for (name, artifact) in &manifest.external {
            let status = if artifact.path.is_some() {
                "local"
            } else if artifact.url.is_some() {
                "remote"
            } else {
                "undefined"
            };
            println!("  • {name} ({}) - {status}", artifact.artifact_type);
        }
    }

    // Final status
    println!();
    if result.is_valid() {
        println!("✓ Check passed - ready to build");
        Ok(())
    } else {
        bail!("Check failed - please fix the issues above before building");
    }
}
