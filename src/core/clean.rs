//! Clean logic
//!
//! This module contains the business logic for cleaning build artifacts.
//! It removes the build/ and output/ directories.
//!
//! **Validates: Requirement 4.5**

use std::path::Path;

use crate::error::FilesystemError;

/// Directories to remove during clean
pub const CLEAN_DIRECTORIES: &[&str] = &["build", "output"];

/// Result of clean operation
#[derive(Debug, Default)]
pub struct CleanResult {
    /// Directories that were removed
    pub removed: Vec<String>,
    /// Directories that didn't exist (skipped)
    pub skipped: Vec<String>,
}

/// Clean build artifacts from a project
///
/// Removes the build/ and output/ directories if they exist.
///
/// # Arguments
///
/// * `project_path` - Path to the project root
///
/// # Returns
///
/// * `Ok(CleanResult)` - Information about what was cleaned
/// * `Err(FilesystemError)` - If removal fails
pub fn clean_project(project_path: &Path) -> Result<CleanResult, FilesystemError> {
    let mut result = CleanResult::default();

    for dir_name in CLEAN_DIRECTORIES {
        let dir_path = project_path.join(dir_name);

        if dir_path.exists() {
            // Remove the directory recursively
            std::fs::remove_dir_all(&dir_path).map_err(|e| FilesystemError::RemoveDir {
                path: dir_path.clone(),
                error: e.to_string(),
            })?;
            result.removed.push((*dir_name).to_string());
        } else {
            result.skipped.push((*dir_name).to_string());
        }
    }

    Ok(result)
}

/// Check if a project has any build artifacts
///
/// # Arguments
///
/// * `project_path` - Path to the project root
///
/// # Returns
///
/// * `true` if any clean directories exist
/// * `false` if no clean directories exist
pub fn has_build_artifacts(project_path: &Path) -> bool {
    CLEAN_DIRECTORIES
        .iter()
        .any(|dir| project_path.join(dir).exists())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_project() -> TempDir {
        TempDir::new().expect("Failed to create temp directory")
    }

    #[test]
    fn test_clean_removes_build_directory() {
        let project = create_test_project();
        let build_dir = project.path().join("build");
        std::fs::create_dir_all(&build_dir).unwrap();
        std::fs::write(build_dir.join("test.txt"), "test").unwrap();

        let result = clean_project(project.path()).unwrap();

        assert!(!build_dir.exists());
        assert!(result.removed.contains(&"build".to_string()));
    }

    #[test]
    fn test_clean_removes_output_directory() {
        let project = create_test_project();
        let output_dir = project.path().join("output");
        std::fs::create_dir_all(&output_dir).unwrap();
        std::fs::write(output_dir.join("rootfs.img"), "image").unwrap();

        let result = clean_project(project.path()).unwrap();

        assert!(!output_dir.exists());
        assert!(result.removed.contains(&"output".to_string()));
    }

    #[test]
    fn test_clean_removes_both_directories() {
        let project = create_test_project();
        let build_dir = project.path().join("build");
        let output_dir = project.path().join("output");
        std::fs::create_dir_all(&build_dir).unwrap();
        std::fs::create_dir_all(&output_dir).unwrap();

        let result = clean_project(project.path()).unwrap();

        assert!(!build_dir.exists());
        assert!(!output_dir.exists());
        assert!(result.removed.contains(&"build".to_string()));
        assert!(result.removed.contains(&"output".to_string()));
    }

    #[test]
    fn test_clean_succeeds_when_no_artifacts() {
        let project = create_test_project();

        let result = clean_project(project.path()).unwrap();

        assert!(result.removed.is_empty());
        assert!(result.skipped.contains(&"build".to_string()));
        assert!(result.skipped.contains(&"output".to_string()));
    }

    #[test]
    fn test_has_build_artifacts_true() {
        let project = create_test_project();
        std::fs::create_dir_all(project.path().join("build")).unwrap();

        assert!(has_build_artifacts(project.path()));
    }

    #[test]
    fn test_has_build_artifacts_false() {
        let project = create_test_project();

        assert!(!has_build_artifacts(project.path()));
    }
}
