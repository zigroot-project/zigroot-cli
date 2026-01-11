//! Lock file handling
//!
//! The lock file (zigroot.lock) records exact versions and checksums
//! for reproducible builds.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Lock file structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LockFile {
    /// Lock file format version
    pub version: u32,

    /// Zig compiler version used
    pub zig_version: String,

    /// Locked package versions
    pub packages: HashMap<String, LockedPackage>,
}

/// A locked package entry
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LockedPackage {
    /// Exact version
    pub version: String,

    /// Source URI (registry, git:<url>#<sha>, path:<path>)
    pub source: String,

    /// SHA256 checksum of source
    pub checksum: String,

    /// Dependencies (package names)
    #[serde(default)]
    pub dependencies: Vec<String>,
}

impl LockFile {
    /// Create a new lock file
    pub fn new(zig_version: String) -> Self {
        Self {
            version: 1,
            zig_version,
            packages: HashMap::new(),
        }
    }

    /// Parse from TOML string
    pub fn from_toml(content: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(content)
    }

    /// Serialize to TOML string
    pub fn to_toml(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }

    /// Add a locked package
    pub fn add_package(&mut self, name: String, package: LockedPackage) {
        self.packages.insert(name, package);
    }
}

impl Default for LockFile {
    fn default() -> Self {
        Self::new("unknown".to_string())
    }
}
