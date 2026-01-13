//! CLI implementation for `zigroot update` command
//!
//! This module handles the CLI interface for updating packages in a project
//! and checking for zigroot updates.

use std::path::Path;

use anyhow::{Context, Result};

use crate::core::update::update_packages;
use crate::core::version::{
    check_for_updates, detect_install_method, format_update_result, UpdateCheckResult,
};

/// Execute the update command for packages
pub async fn execute(path: &Path, package: Option<String>) -> Result<()> {
    // Check if manifest exists
    let manifest_path = path.join("zigroot.toml");
    if !manifest_path.exists() {
        anyhow::bail!(
            "No zigroot.toml found in {}. Run 'zigroot init' first.",
            path.display()
        );
    }

    let result = update_packages(path, package.as_deref())
        .await
        .with_context(|| "Failed to update packages")?;

    // Print results
    if result.checked.is_empty() {
        println!("No packages to update.");
        return Ok(());
    }

    println!(
        "Checking {} package(s) for updates...",
        result.checked.len()
    );

    if !result.updated.is_empty() {
        println!("\n✓ Updated packages:");
        for (name, old_ver, new_ver) in &result.updated {
            println!("  {name}: {old_ver} → {new_ver}");
        }
    }

    if !result.up_to_date.is_empty() && result.updated.is_empty() {
        println!("\nAll packages are up to date.");
    } else if !result.up_to_date.is_empty() {
        println!("\nAlready up to date:");
        for name in &result.up_to_date {
            println!("  {name}");
        }
    }

    if result.lock_updated {
        println!("\n  Updated zigroot.lock");
    }

    Ok(())
}

/// Execute the self-update command (zigroot update --self)
pub async fn execute_self_update() -> Result<()> {
    println!("Checking for zigroot updates...\n");

    let result = check_for_updates().await;
    let install_method = detect_install_method();

    let output = format_update_result(&result, &install_method);
    println!("{output}");

    // Return appropriate exit code
    match result {
        UpdateCheckResult::UpdateAvailable { .. } => {
            // Exit with code 0 but indicate update is available
            Ok(())
        }
        UpdateCheckResult::UpToDate { .. } => Ok(()),
        UpdateCheckResult::CheckFailed { reason } => {
            // Don't fail the command, just warn
            eprintln!("\n⚠ Warning: {reason}");
            Ok(())
        }
    }
}
