//! Filesystem operations
//!
//! Handles file and directory operations.

use std::path::Path;

use crate::error::FilesystemError;

/// Create a directory and all parent directories
pub fn create_dir_all(path: &Path) -> Result<(), FilesystemError> {
    std::fs::create_dir_all(path).map_err(|e| FilesystemError::CreateDir {
        path: path.to_path_buf(),
        error: e.to_string(),
    })
}

/// Remove a directory and all its contents
pub fn remove_dir_all(path: &Path) -> Result<(), FilesystemError> {
    if path.exists() {
        std::fs::remove_dir_all(path).map_err(|e| FilesystemError::RemoveDir {
            path: path.to_path_buf(),
            error: e.to_string(),
        })?;
    }
    Ok(())
}

/// Write content to a file
pub fn write_file(path: &Path, content: &str) -> Result<(), FilesystemError> {
    if let Some(parent) = path.parent() {
        create_dir_all(parent)?;
    }
    std::fs::write(path, content).map_err(|e| FilesystemError::WriteFile {
        path: path.to_path_buf(),
        error: e.to_string(),
    })
}

/// Read content from a file
pub fn read_file(path: &Path) -> Result<String, FilesystemError> {
    std::fs::read_to_string(path).map_err(|e| FilesystemError::ReadFile {
        path: path.to_path_buf(),
        error: e.to_string(),
    })
}
