//! Git operations
//!
//! Handles cloning repositories and checking out refs.

use std::path::PathBuf;

/// Git repository operations
#[derive(Debug)]
pub struct GitOperations {
    /// Working directory for git operations
    work_dir: PathBuf,
}

impl GitOperations {
    /// Create a new git operations handler
    pub fn new(work_dir: PathBuf) -> Self {
        Self { work_dir }
    }

    /// Get the working directory
    pub fn work_dir(&self) -> &PathBuf {
        &self.work_dir
    }
}
