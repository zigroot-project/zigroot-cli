//! Project initialization logic
//!
//! This module contains the business logic for initializing a new zigroot project.
//! It handles creating the project structure, manifest, and .gitignore.

use std::path::Path;

use crate::core::manifest::Manifest;
use crate::error::InitError;

/// Directories that should be created during init
pub const REQUIRED_DIRECTORIES: &[&str] = &["packages", "boards", "user/files", "user/scripts"];

/// Entries to add to .gitignore
pub const GITIGNORE_ENTRIES: &[&str] = &["build/", "downloads/", "output/", "external/"];

/// Marker comment for zigroot section in .gitignore
pub const GITIGNORE_MARKER: &str = "# zigroot";

/// Options for project initialization
#[derive(Debug, Clone, Default)]
pub struct InitOptions {
    /// Target board name (optional)
    pub board: Option<String>,
    /// Force initialization in non-empty directory
    pub force: bool,
}

/// Result of initialization
#[derive(Debug)]
pub struct InitResult {
    /// Path to created manifest
    pub manifest_path: std::path::PathBuf,
    /// Whether .gitignore was created or updated
    pub gitignore_updated: bool,
    /// Board that was configured (if any)
    pub board: Option<String>,
}

/// Check if a directory is empty (ignoring hidden files like .git)
pub fn is_directory_empty(path: &Path) -> std::io::Result<bool> {
    let entries: Vec<_> = std::fs::read_dir(path)?
        .filter_map(Result::ok)
        .filter(|e| {
            // Ignore hidden files/directories
            !e.file_name()
                .to_str()
                .map(|s| s.starts_with('.'))
                .unwrap_or(false)
        })
        .collect();
    Ok(entries.is_empty())
}

/// Generate the default manifest content with comments
pub fn generate_manifest_content(project_name: &str, board: Option<&str>) -> String {
    let board_section = if let Some(board_name) = board {
        format!(
            r#"
[board]
name = "{board_name}"
# Board-specific options can be overridden here:
# [board.options]
# option_name = "value"
"#
        )
    } else {
        r#"
[board]
# Uncomment and set your target board:
# name = "luckfox-pico"
# Board-specific options can be overridden here:
# [board.options]
# option_name = "value"
"#
        .to_string()
    };

    format!(
        r#"# Zigroot Project Configuration
# See https://github.com/zigroot-project/zigroot-cli for documentation

[project]
name = "{project_name}"
version = "0.1.0"
# description = "My embedded Linux project"
{board_section}
[build]
# Enable binary compression with UPX
compress = false
# Image format: ext4, squashfs, or initramfs
image_format = "ext4"
# Root filesystem size
rootfs_size = "256M"
# Target hostname
hostname = "zigroot"
# Number of parallel build jobs (defaults to CPU count)
# jobs = 4

# Package dependencies
# [packages.busybox]
# version = "1.36.1"
#
# [packages.dropbear]
# git = "https://github.com/example/dropbear"
# ref_ = "v2024.85"

# External artifacts (bootloader, kernel, etc.)
# [external.bootloader]
# type = "bootloader"
# url = "https://example.com/uboot.bin"
# sha256 = "..."
"#
    )
}

/// Generate .gitignore content for zigroot
pub fn generate_gitignore_content() -> String {
    let mut content = String::from(GITIGNORE_MARKER);
    content.push('\n');
    for entry in GITIGNORE_ENTRIES {
        content.push_str(entry);
        content.push('\n');
    }
    content
}

/// Check if .gitignore already has zigroot entries
pub fn gitignore_has_zigroot_entries(content: &str) -> bool {
    content.contains(GITIGNORE_MARKER)
}

/// Append zigroot entries to existing .gitignore content
pub fn append_gitignore_entries(existing: &str) -> String {
    if gitignore_has_zigroot_entries(existing) {
        // Already has zigroot entries, return as-is (idempotent)
        return existing.to_string();
    }

    let mut result = existing.to_string();
    if !result.is_empty() && !result.ends_with('\n') {
        result.push('\n');
    }
    if !result.is_empty() {
        result.push('\n');
    }
    result.push_str(&generate_gitignore_content());
    result
}

/// Validate initialization can proceed
pub fn validate_init(path: &Path, options: &InitOptions) -> Result<(), InitError> {
    // Check if directory exists
    if !path.exists() {
        return Err(InitError::DirectoryNotFound {
            path: path.to_path_buf(),
        });
    }

    // Check if directory is empty (unless --force)
    if !options.force {
        let is_empty = is_directory_empty(path).map_err(|e| InitError::IoError {
            path: path.to_path_buf(),
            error: e.to_string(),
        })?;

        if !is_empty {
            return Err(InitError::DirectoryNotEmpty {
                path: path.to_path_buf(),
            });
        }
    }

    Ok(())
}

/// Create the project structure (directories only, no files)
pub fn create_project_structure(path: &Path) -> Result<(), InitError> {
    for dir in REQUIRED_DIRECTORIES {
        let dir_path = path.join(dir);
        std::fs::create_dir_all(&dir_path).map_err(|e| InitError::IoError {
            path: dir_path,
            error: e.to_string(),
        })?;
    }
    Ok(())
}

/// Derive project name from directory
pub fn derive_project_name(path: &Path) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "my-project".to_string())
}

/// Parse the manifest from generated content (for validation)
pub fn parse_manifest(content: &str) -> Result<Manifest, InitError> {
    // For parsing, we need to handle the commented-out sections
    // Create a minimal manifest for validation
    Manifest::from_toml(content).map_err(|e| InitError::ManifestError {
        error: e.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_gitignore_content() {
        let content = generate_gitignore_content();
        assert!(content.contains(GITIGNORE_MARKER));
        assert!(content.contains("build/"));
        assert!(content.contains("downloads/"));
        assert!(content.contains("output/"));
        assert!(content.contains("external/"));
    }

    #[test]
    fn test_gitignore_has_zigroot_entries() {
        assert!(gitignore_has_zigroot_entries("# zigroot\nbuild/\n"));
        assert!(!gitignore_has_zigroot_entries("*.log\nnode_modules/\n"));
    }

    #[test]
    fn test_append_gitignore_entries_to_empty() {
        let result = append_gitignore_entries("");
        assert!(result.contains(GITIGNORE_MARKER));
        assert!(result.contains("build/"));
    }

    #[test]
    fn test_append_gitignore_entries_to_existing() {
        let existing = "*.log\nnode_modules/\n";
        let result = append_gitignore_entries(existing);
        assert!(result.contains("*.log"));
        assert!(result.contains("node_modules/"));
        assert!(result.contains(GITIGNORE_MARKER));
        assert!(result.contains("build/"));
    }

    #[test]
    fn test_append_gitignore_entries_idempotent() {
        let existing = "*.log\n";
        let first = append_gitignore_entries(existing);
        let second = append_gitignore_entries(&first);
        assert_eq!(first, second, "Appending should be idempotent");
    }

    #[test]
    fn test_generate_manifest_content() {
        let content = generate_manifest_content("test-project", None);
        assert!(content.contains("test-project"));
        assert!(content.contains("[project]"));
        assert!(content.contains("[board]"));
        assert!(content.contains("[build]"));
        assert!(content.contains('#')); // Has comments
    }

    #[test]
    fn test_generate_manifest_content_with_board() {
        let content = generate_manifest_content("test-project", Some("luckfox-pico"));
        assert!(content.contains("luckfox-pico"));
        assert!(content.contains("name = \"luckfox-pico\""));
    }

    #[test]
    fn test_derive_project_name() {
        let path = std::path::Path::new("/home/user/my-project");
        assert_eq!(derive_project_name(path), "my-project");
    }
}
