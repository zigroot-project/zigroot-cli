//! Toolchain management
//!
//! Handles Zig and GCC toolchain setup and invocation.

use std::path::PathBuf;

/// Zig toolchain wrapper
#[derive(Debug)]
pub struct ZigToolchain {
    /// Path to zig binary
    zig_path: PathBuf,
}

impl ZigToolchain {
    /// Create a new Zig toolchain wrapper
    pub fn new(zig_path: PathBuf) -> Self {
        Self { zig_path }
    }

    /// Get the path to the zig binary
    pub fn zig_path(&self) -> &PathBuf {
        &self.zig_path
    }
}

impl Default for ZigToolchain {
    fn default() -> Self {
        Self::new(PathBuf::from("zig"))
    }
}
