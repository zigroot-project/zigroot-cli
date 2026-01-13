//! CLI implementation for `zigroot init` command
//!
//! This module handles the CLI interface for project initialization.

use std::path::Path;

use anyhow::{Context, Result};

use crate::cli::output::{print_detail, print_success};
use crate::core::init::{
    append_gitignore_entries, create_project_structure, derive_project_name,
    generate_gitignore_content, generate_manifest_content, validate_init, InitOptions,
};

/// Execute the init command
pub async fn execute(path: &Path, board: Option<String>, force: bool) -> Result<()> {
    let options = InitOptions {
        board: board.clone(),
        force,
    };

    // Validate we can proceed
    validate_init(path, &options).with_context(|| "Failed to validate initialization")?;

    // Create directory structure
    create_project_structure(path).with_context(|| "Failed to create project structure")?;

    // Generate and write manifest
    let project_name = derive_project_name(path);
    let manifest_content = generate_manifest_content(&project_name, board.as_deref());
    let manifest_path = path.join("zigroot.toml");

    std::fs::write(&manifest_path, &manifest_content)
        .with_context(|| format!("Failed to write manifest to {}", manifest_path.display()))?;

    // Handle .gitignore
    let gitignore_path = path.join(".gitignore");
    let gitignore_existed = gitignore_path.exists();
    let gitignore_content = if gitignore_existed {
        let existing = std::fs::read_to_string(&gitignore_path)
            .with_context(|| format!("Failed to read {}", gitignore_path.display()))?;
        append_gitignore_entries(&existing)
    } else {
        generate_gitignore_content()
    };

    std::fs::write(&gitignore_path, &gitignore_content)
        .with_context(|| format!("Failed to write {}", gitignore_path.display()))?;

    // Print success message using output module
    print_success(&format!(
        "Initialized zigroot project in {}",
        path.display()
    ));
    print_detail("Created zigroot.toml");
    print_detail("Created directories: packages/, boards/, user/files/, user/scripts/");
    if gitignore_existed {
        print_detail("Updated .gitignore");
    } else {
        print_detail("Created .gitignore");
    }

    if let Some(board_name) = &board {
        print_detail(&format!("Configured board: {board_name}"));
    }

    Ok(())
}
