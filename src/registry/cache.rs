//! Registry cache implementation
//!
//! Caches registry index and package metadata locally.

use std::path::PathBuf;

/// Local cache for registry data
#[derive(Debug)]
pub struct RegistryCache {
    /// Cache directory path
    cache_dir: PathBuf,
}

impl RegistryCache {
    /// Create a new registry cache
    pub fn new(cache_dir: PathBuf) -> Self {
        Self { cache_dir }
    }

    /// Get the cache directory
    pub fn cache_dir(&self) -> &PathBuf {
        &self.cache_dir
    }
}

impl Default for RegistryCache {
    fn default() -> Self {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from(".cache"))
            .join("zigroot");
        Self::new(cache_dir)
    }
}
