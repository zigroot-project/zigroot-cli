//! Common test utilities and helpers
//!
//! This module provides shared utilities for integration tests.

use std::path::PathBuf;
use tempfile::TempDir;

/// Test project context
///
/// Creates a temporary directory for test projects and provides
/// utilities for setting up test scenarios.
pub struct TestProject {
    /// Temporary directory for the test project
    pub dir: TempDir,
}

impl TestProject {
    /// Create a new test project in a temporary directory
    pub fn new() -> Self {
        Self {
            dir: TempDir::new().expect("Failed to create temp directory"),
        }
    }

    /// Get the path to the test project directory
    pub fn path(&self) -> PathBuf {
        self.dir.path().to_path_buf()
    }

    /// Create a file in the test project
    pub fn create_file(&self, name: &str, content: &str) {
        let path = self.dir.path().join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("Failed to create parent directories");
        }
        std::fs::write(path, content).expect("Failed to write file");
    }

    /// Create a directory in the test project
    pub fn create_dir(&self, name: &str) {
        let path = self.dir.path().join(name);
        std::fs::create_dir_all(path).expect("Failed to create directory");
    }

    /// Check if a file exists in the test project
    pub fn file_exists(&self, name: &str) -> bool {
        self.dir.path().join(name).exists()
    }

    /// Read a file from the test project
    pub fn read_file(&self, name: &str) -> String {
        std::fs::read_to_string(self.dir.path().join(name)).expect("Failed to read file")
    }
}

impl Default for TestProject {
    fn default() -> Self {
        Self::new()
    }
}

/// Sample manifest TOML for testing
#[allow(dead_code)]
pub const SAMPLE_MANIFEST: &str = r#"
[project]
name = "test-project"
version = "1.0.0"
description = "A test project"

[board]
name = "test-board"

[build]
compress = false
image_format = "ext4"
rootfs_size = "256M"
hostname = "test"
"#;

/// Sample package definition TOML for testing
#[allow(dead_code)]
pub const SAMPLE_PACKAGE: &str = r#"
[package]
name = "test-package"
version = "1.0.0"
description = "A test package"

[source]
url = "https://example.com/test-1.0.0.tar.gz"
sha256 = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"

[build]
type = "make"
"#;

/// Sample board definition TOML for testing
#[allow(dead_code)]
pub const SAMPLE_BOARD: &str = r#"
[board]
name = "test-board"
description = "A test board"
target = "arm-linux-musleabihf"
cpu = "cortex-a7"

[defaults]
image_format = "ext4"
rootfs_size = "256M"
hostname = "test"
"#;
