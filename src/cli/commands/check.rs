//! Check command implementation
//!
//! Implements `zigroot check` to validate configuration without building.
//!
//! **Validates: Requirements 4.13**

use anyhow::{bail, Context, Result};
use std::fs;
use std::path::Path;

use crate::cli::output::{
    is_json, is_quiet, print_detail, print_info, print_success, print_warning, status,
};
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
    let result =
        check::check(project_dir, &manifest).map_err(|e| anyhow::anyhow!("Check failed: {}", e))?;

    // JSON output mode
    if is_json() {
        let json_result = serde_json::json!({
            "status": if result.is_valid() { "success" } else { "error" },
            "config_valid": result.config_valid,
            "dependencies_valid": result.dependencies_valid,
            "toolchains_available": result.toolchains_available,
            "missing_dependencies": result.missing_dependencies,
            "warnings": result.warnings,
            "packages_to_build": result.packages_to_build,
            "build_order": result.build_order,
            "board": manifest.board.name,
            "build_settings": {
                "image_format": manifest.build.image_format,
                "rootfs_size": manifest.build.rootfs_size,
                "hostname": manifest.build.hostname,
                "compress": manifest.build.compress
            }
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&json_result).unwrap_or_default()
        );

        if result.is_valid() {
            return Ok(());
        } else {
            bail!("Check failed");
        }
    }

    // Quiet mode - only show errors
    if is_quiet() {
        if !result.is_valid() {
            if !result.config_valid {
                eprintln!("{} Configuration has errors", status::ERROR);
            }
            if !result.dependencies_valid {
                for dep in &result.missing_dependencies {
                    eprintln!("{} Missing dependency: {dep}", status::ERROR);
                }
            }
            bail!("Check failed");
        }
        return Ok(());
    }

    // Normal output mode
    print_info("Checking project configuration...");
    println!();

    // Configuration status
    if result.config_valid {
        println!("{} Configuration is valid", status::SUCCESS);
    } else {
        println!("{} Configuration has errors", status::ERROR);
    }

    // Dependencies status
    if result.dependencies_valid {
        println!("{} All dependencies are resolvable", status::SUCCESS);
    } else {
        println!("{} Dependency issues found", status::ERROR);
        for dep in &result.missing_dependencies {
            print_detail(&format!("Missing dependency: {dep}"));
        }
    }

    // Toolchain status
    if result.toolchains_available {
        println!("{} Zig toolchain is available", status::SUCCESS);
    } else {
        println!("{} Zig toolchain not found in PATH", status::WARNING);
    }

    // Display warnings
    if !result.warnings.is_empty() {
        println!("\nWarnings:");
        for warning in &result.warnings {
            print_warning(warning);
        }
    }

    // Display what would be built
    println!("\nPackages that would be built:");
    if result.packages_to_build.is_empty() {
        print_detail("(none)");
    } else {
        for pkg in &result.build_order {
            if result.packages_to_build.contains(pkg) {
                print_detail(&format!("• {pkg}"));
            }
        }
        // Also show packages not in build order (e.g., if dependency resolution failed)
        for pkg in &result.packages_to_build {
            if !result.build_order.contains(pkg) {
                print_detail(&format!("• {pkg}"));
            }
        }
    }

    // Board info
    if let Some(board_name) = &manifest.board.name {
        println!("\nTarget board: {board_name}");
    }

    // Build settings
    println!("\nBuild settings:");
    print_detail(&format!("Image format: {}", manifest.build.image_format));
    print_detail(&format!("Rootfs size: {}", manifest.build.rootfs_size));
    print_detail(&format!("Hostname: {}", manifest.build.hostname));
    print_detail(&format!(
        "Compression: {}",
        if manifest.build.compress {
            "enabled"
        } else {
            "disabled"
        }
    ));

    // External artifacts
    if !manifest.external.is_empty() {
        println!("\nExternal artifacts:");
        for (name, artifact) in &manifest.external {
            let artifact_status = if artifact.path.is_some() {
                "local"
            } else if artifact.url.is_some() {
                "remote"
            } else {
                "undefined"
            };
            print_detail(&format!(
                "• {name} ({}) - {artifact_status}",
                artifact.artifact_type
            ));
        }
    }

    // Final status
    println!();
    if result.is_valid() {
        print_success("Check passed - ready to build");
        Ok(())
    } else {
        bail!("Check failed - please fix the issues above before building");
    }
}
