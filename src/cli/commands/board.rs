//! Board subcommand implementations
//!
//! Implements `zigroot board list`, `zigroot board set`, `zigroot board info`, and `zigroot board new`.
//!
//! **Validates: Requirements 9.1-9.4, 29.1**

use anyhow::Result;
use std::path::Path;

use crate::core::board::BoardDefinition;
use crate::core::manifest::Manifest;
use crate::registry::client::RegistryClient;

/// Execute the board list command
///
/// Lists all available boards from the registry.
/// **Validates: Requirement 9.1**
pub async fn execute_list() -> Result<()> {
    let client = RegistryClient::new();

    tracing::info!("Fetching board list from registry...");

    let index = client
        .fetch_board_index()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to fetch board index: {}", e))?;

    if index.boards.is_empty() {
        println!("No boards available in registry.");
        return Ok(());
    }

    println!("Available boards:");
    println!();

    for board in &index.boards {
        println!(
            "  [board] {} ({}) - {}",
            board.name, board.arch, board.description
        );

        // Show target triple
        println!("    Target: {}", board.target);

        // Show keywords if any
        if !board.keywords.is_empty() {
            println!("    Keywords: {}", board.keywords.join(", "));
        }

        println!();
    }

    println!("{} board(s) available.", index.boards.len());

    Ok(())
}

/// Execute the board set command
///
/// Updates the manifest with a new board configuration.
/// **Validates: Requirements 9.2, 9.3**
pub async fn execute_set(project_dir: &Path, board_name: &str) -> Result<()> {
    let manifest_path = project_dir.join("zigroot.toml");

    if !manifest_path.exists() {
        anyhow::bail!("No zigroot.toml found. Run 'zigroot init' first.");
    }

    let client = RegistryClient::new();

    tracing::info!("Fetching board '{}' from registry...", board_name);

    // Fetch board definition from registry
    let board_toml = client
        .fetch_board(board_name)
        .await
        .map_err(|e| anyhow::anyhow!("Board '{}' not found: {}", board_name, e))?;

    // Parse the board definition
    let board_def: BoardDefinition = board_toml
        .try_into()
        .map_err(|e| anyhow::anyhow!("Failed to parse board definition: {}", e))?;

    // Read current manifest
    let content = std::fs::read_to_string(&manifest_path)?;
    let mut manifest = Manifest::from_toml(&content)
        .map_err(|e| anyhow::anyhow!("Failed to parse manifest: {}", e))?;

    // Validate compatibility with existing packages
    validate_board_compatibility(&manifest, &board_def)?;

    // Update manifest with new board
    manifest.board.name = Some(board_def.board.name.clone());

    // Update build defaults from board
    manifest.build.image_format = board_def.defaults.image_format.clone();
    manifest.build.rootfs_size = board_def.defaults.rootfs_size.clone();
    manifest.build.hostname = board_def.defaults.hostname.clone();

    // Write updated manifest
    let updated_content = manifest
        .to_toml()
        .map_err(|e| anyhow::anyhow!("Failed to serialize manifest: {}", e))?;

    std::fs::write(&manifest_path, updated_content)?;

    println!("✓ Board set to '{}'", board_name);
    println!("  Target: {}", board_def.board.target);
    println!("  CPU: {}", board_def.board.cpu);

    if !board_def.requires.is_empty() {
        println!();
        println!("Note: This board requires the following packages:");
        for pkg in &board_def.requires {
            println!("  - {}", pkg);
        }
        println!("Run 'zigroot add <package>' to install them.");
    }

    Ok(())
}

/// Execute the board info command
///
/// Displays detailed information about a specific board.
/// **Validates: Requirement 9.4**
pub async fn execute_info(board_name: &str) -> Result<()> {
    let client = RegistryClient::new();

    tracing::info!("Fetching board '{}' from registry...", board_name);

    // Fetch board definition from registry
    let board_toml = client
        .fetch_board(board_name)
        .await
        .map_err(|e| anyhow::anyhow!("Board '{}' not found: {}", board_name, e))?;

    // Parse the board definition
    let board_def: BoardDefinition = board_toml
        .try_into()
        .map_err(|e| anyhow::anyhow!("Failed to parse board definition: {}", e))?;

    // Display board information
    println!("Board: {}", board_def.board.name);
    println!();
    println!("  Description: {}", board_def.board.description);
    println!("  Target: {}", board_def.board.target);
    println!("  CPU: {}", board_def.board.cpu);

    // Features
    if !board_def.board.features.is_empty() {
        println!("  Features: {}", board_def.board.features.join(", "));
    }

    // Kernel
    if let Some(kernel) = &board_def.board.kernel {
        println!("  Kernel: {}", kernel);
    }

    // Minimum zigroot version
    if let Some(version) = &board_def.board.zigroot_version {
        println!("  Minimum zigroot version: {}", version);
    }

    println!();
    println!("Defaults:");
    println!("  Image format: {}", board_def.defaults.image_format);
    println!("  Rootfs size: {}", board_def.defaults.rootfs_size);
    println!("  Hostname: {}", board_def.defaults.hostname);

    // Required packages
    if !board_def.requires.is_empty() {
        println!();
        println!("Required packages:");
        for pkg in &board_def.requires {
            println!("  - {}", pkg);
        }
    }

    // Flash methods
    if !board_def.flash.is_empty() {
        println!();
        println!("Flash methods:");
        for flash in &board_def.flash {
            println!("  {} - {}", flash.name, flash.description);
            if let Some(tool) = &flash.tool {
                println!("    Tool: {}", tool);
            }
            if let Some(script) = &flash.script {
                println!("    Script: {}", script);
            }
            if !flash.requires.is_empty() {
                println!("    Requires: {}", flash.requires.join(", "));
            }
        }
    }

    // Board options
    if !board_def.options.is_empty() {
        println!();
        println!("Options:");
        for (name, opt) in &board_def.options {
            println!("  {} ({}) - {}", name, opt.option_type, opt.description);
            println!("    Default: {}", opt.default);
            if !opt.choices.is_empty() {
                println!("    Choices: {}", opt.choices.join(", "));
            }
        }
    }

    Ok(())
}

/// Validate that the board is compatible with existing packages
fn validate_board_compatibility(manifest: &Manifest, board_def: &BoardDefinition) -> Result<()> {
    // Check if any packages have architecture restrictions
    // This is a simplified check - a full implementation would query package metadata

    let target = &board_def.board.target;

    // Extract architecture from target triple (e.g., "arm" from "arm-linux-musleabihf")
    let arch = target.split('-').next().unwrap_or(target);

    tracing::debug!(
        "Validating board compatibility: target={}, arch={}",
        target,
        arch
    );

    // For now, we assume all packages are compatible
    // A full implementation would check each package's `arch` field
    for (pkg_name, _pkg_ref) in &manifest.packages {
        tracing::debug!("Checking package '{}' compatibility with {}", pkg_name, arch);
        // TODO: Fetch package metadata and check arch compatibility
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::manifest::{ProjectConfig, BoardConfig, BuildConfig};
    use std::collections::HashMap;

    #[test]
    fn test_validate_board_compatibility_empty_manifest() {
        let manifest = Manifest {
            project: ProjectConfig {
                name: "test-project".to_string(),
                version: "1.0.0".to_string(),
                description: None,
            },
            board: BoardConfig {
                name: None,
                options: HashMap::new(),
            },
            build: BuildConfig::default(),
            packages: HashMap::new(),
            external: HashMap::new(),
        };
        let board_def = BoardDefinition {
            board: crate::core::board::BoardMetadata {
                name: "test-board".to_string(),
                description: "Test board".to_string(),
                target: "arm-linux-musleabihf".to_string(),
                cpu: "cortex-a7".to_string(),
                features: vec![],
                kernel: None,
                zigroot_version: None,
            },
            defaults: crate::core::board::BoardDefaults {
                image_format: "ext4".to_string(),
                rootfs_size: "256M".to_string(),
                hostname: "test".to_string(),
            },
            requires: vec![],
            flash: vec![],
            options: std::collections::HashMap::new(),
        };

        let result = validate_board_compatibility(&manifest, &board_def);
        assert!(result.is_ok());
    }
}

/// Execute the board new command
///
/// Creates a new board template in boards/<name>/ with board.toml.
/// **Validates: Requirement 29.1**
pub async fn execute_new(project_dir: &Path, name: &str) -> Result<()> {
    let boards_dir = project_dir.join("boards");
    let board_dir = boards_dir.join(name);

    // Check if board already exists
    if board_dir.exists() {
        anyhow::bail!(
            "Board '{}' already exists at {}",
            name,
            board_dir.display()
        );
    }

    // Create boards directory if it doesn't exist
    std::fs::create_dir_all(&boards_dir)?;

    // Create board directory
    std::fs::create_dir_all(&board_dir)?;

    // Generate board.toml content
    let board_content = generate_board_template(name);
    let board_path = board_dir.join("board.toml");
    std::fs::write(&board_path, board_content)?;

    println!("✓ Created board template for '{}'", name);
    println!("  Directory: {}", board_dir.display());
    println!("  Files:");
    println!("    - board.toml (board definition)");
    println!();
    println!("Next steps:");
    println!("  1. Edit board.toml with your board's target triple and CPU");
    println!("  2. Configure flash methods if applicable");
    println!("  3. Run 'zigroot verify boards/{}' to validate", name);

    Ok(())
}

/// Generate board.toml template content
fn generate_board_template(name: &str) -> String {
    format!(
        r#"# Board definition for {name}

[board]
name = "{name}"
description = "TODO: Add board description"
target = "arm-linux-musleabihf"
cpu = "cortex-a7"
# features = ["neon", "vfpv4"]
# kernel = "linux-luckfox"
# zigroot_version = ">=0.1.0"

[defaults]
image_format = "ext4"
rootfs_size = "256M"
hostname = "{name}"

# Required packages for this board (optional)
# requires = ["busybox"]

# Flash methods (optional)
# [[flash]]
# name = "sd"
# description = "Flash to SD card"
# tool = "dd"
# script = "flash-sd.sh"
# requires = ["bootloader", "kernel"]

# Board options (optional)
# [options.uart_console]
# type = "bool"
# default = true
# description = "Enable UART console"
"#
    )
}
