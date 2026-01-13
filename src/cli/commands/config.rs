//! CLI command for `zigroot config`
//!
//! Launches interactive TUI configuration interface.
//!
//! **Validates: Requirements 25.1-25.17**

use anyhow::Result;
use std::path::Path;

use crate::cli::tui::ConfigTui;
use crate::core::config::{
    get_available_packages, is_terminal_interactive, load_manifest_for_config, ConfigState,
};

/// Execute config command
pub async fn execute(project_dir: &Path, board_only: bool, packages_only: bool) -> Result<()> {
    let is_interactive = is_terminal_interactive();

    // Try to load manifest
    let manifest = load_manifest_for_config(project_dir).ok();

    if manifest.is_none() {
        println!("⚠️  No zigroot.toml manifest found in current directory.");
        println!("   Run 'zigroot init' to create a new project first.");
        println!();
        println!("The config TUI allows you to:");
        println!("  • Select target board");
        println!("  • Browse and select packages");
        println!("  • Configure build options");
        println!("  • Save changes to zigroot.toml");
        return Ok(());
    }

    let state = ConfigState::new(manifest, is_interactive);

    if !is_interactive {
        print_non_interactive_info(&state, project_dir);
        return Ok(());
    }

    // Launch the TUI
    let mut tui = ConfigTui::new(project_dir, board_only, packages_only)?;
    tui.run()?;

    Ok(())
}

/// Print information when running in non-interactive mode
fn print_non_interactive_info(state: &ConfigState, project_dir: &Path) {
    println!("🔧 Zigroot Configuration (TUI)");
    println!();
    println!("⚠️  Interactive terminal required for TUI mode.");
    println!("   The config command requires an interactive terminal to display the menu.");
    println!();
    println!("TUI Features:");
    println!("  • Board selection - choose target hardware");
    println!("  • Package selection - browse and select packages with dependencies");
    println!("  • Build options - configure compression, image format, rootfs size");
    println!("  • Save changes - write configuration to zigroot.toml");
    println!();

    // Show current configuration summary
    if let Some(ref manifest) = state.manifest {
        println!("Current Configuration:");
        println!(
            "  Board: {}",
            manifest.board.name.as_deref().unwrap_or("not set")
        );
        println!(
            "  Packages: {}",
            if manifest.packages.is_empty() {
                "none".to_string()
            } else {
                manifest.packages.len().to_string()
            }
        );
    }

    // Show available packages
    let packages = get_available_packages(project_dir);
    if !packages.is_empty() {
        println!();
        println!("Available local packages: {}", packages.len());
    }

    println!();
    println!("To use the interactive TUI, run this command in a terminal that supports");
    println!("interactive input (not in a dumb terminal or piped context).");
}
