//! CLI command implementation for `zigroot flash`
//!
//! **Validates: Requirements 7.1-7.12**

use anyhow::{bail, Context, Result};
use std::path::Path;

use crate::core::flash::{load_board_definition, FlashExecutor, FlashOptions};
use crate::core::manifest::Manifest;

/// Execute the flash command
pub async fn execute(
    project_root: &Path,
    method: Option<String>,
    device: Option<String>,
    yes: bool,
    list: bool,
) -> Result<()> {
    // Load manifest
    let manifest_path = project_root.join("zigroot.toml");
    if !manifest_path.exists() {
        bail!(
            "No zigroot.toml found. Run 'zigroot init' to create a project."
        );
    }

    let manifest_content = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("Failed to read manifest: {}", manifest_path.display()))?;

    let manifest = Manifest::from_toml(&manifest_content)
        .with_context(|| "Failed to parse zigroot.toml")?;

    // Load board definition if configured
    let board = if let Some(board_name) = &manifest.board.name {
        match load_board_definition(project_root, board_name) {
            Ok(b) => Some(b),
            Err(e) => {
                tracing::warn!("Could not load board definition: {}", e);
                None
            }
        }
    } else {
        None
    };

    // Create flash options
    let options = FlashOptions {
        method,
        device,
        yes,
        list,
    };

    // Execute flash
    let executor = FlashExecutor::new(project_root, manifest, board);
    let result = executor.execute(&options)?;

    // Print result
    if result.success {
        println!("{}", result.message);
    } else {
        eprintln!("{}", result.message);
        bail!("Flash operation failed");
    }

    Ok(())
}
