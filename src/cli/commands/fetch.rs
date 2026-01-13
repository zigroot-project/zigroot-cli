//! CLI implementation for `zigroot fetch` command
//!
//! This module handles the CLI interface for downloading package sources.

use std::path::Path;

use anyhow::{Context, Result};

use crate::core::fetch::{fetch_packages, FetchOptions};

/// Execute the fetch command
pub async fn execute(path: &Path, parallel: usize, force: bool) -> Result<()> {
    // Check if manifest exists
    let manifest_path = path.join("zigroot.toml");
    if !manifest_path.exists() {
        anyhow::bail!(
            "No zigroot.toml found in {}. Run 'zigroot init' first.",
            path.display()
        );
    }

    let options = FetchOptions {
        parallel: if parallel == 0 { 4 } else { parallel },
        force,
    };

    let result = fetch_packages(path, &options)
        .await
        .with_context(|| "Failed to fetch packages")?;

    // Print summary
    if result.downloaded.is_empty()
        && result.skipped.is_empty()
        && result.external_downloaded.is_empty()
    {
        println!("✓ Nothing to fetch");
    } else {
        if !result.downloaded.is_empty() {
            println!("✓ Downloaded {} package(s):", result.downloaded.len());
            for pkg in &result.downloaded {
                println!("    {} v{}", pkg.name, pkg.version);
            }
        }

        if !result.skipped.is_empty() {
            println!(
                "  Skipped {} package(s) (already downloaded)",
                result.skipped.len()
            );
        }

        if !result.external_downloaded.is_empty() {
            println!(
                "✓ Downloaded {} external artifact(s):",
                result.external_downloaded.len()
            );
            for name in &result.external_downloaded {
                println!("    {name}");
            }
        }

        if !result.external_skipped.is_empty() {
            println!(
                "  Skipped {} external artifact(s) (already downloaded)",
                result.external_skipped.len()
            );
        }

        if !result.failed.is_empty() {
            println!("✗ Failed to download {} item(s):", result.failed.len());
            for (name, error) in &result.failed {
                println!("    {name}: {error}");
            }
        }
    }

    Ok(())
}
