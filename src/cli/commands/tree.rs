//! CLI command for displaying dependency tree
//!
//! Implements the `zigroot tree` command.

use std::path::Path;

use anyhow::Result;

use crate::core::tree;

/// Execute the tree command
pub async fn execute(project_dir: &Path, package: Option<String>, graph: bool) -> Result<()> {
    let output = tree::display_tree(project_dir, package.as_deref(), graph)?;
    println!("{output}");
    Ok(())
}
