//! Shared storage for downloads and build cache
//!
//! Provides shared storage for source archives across projects and
//! content-addressable build cache for artifact sharing.
//!
//! **Validates: Requirements 32.9, 32.10**

use crate::infra::dirs::ZigrootDirs;
use sha2::{Digest, Sha256};
use std::path::PathBuf;

/// Shared storage manager for downloads and build cache
///
/// Manages shared storage locations for:
/// - Source archives (shared across projects)
/// - Build cache (content-addressable)
#[derive(Debug, Clone)]
pub struct SharedStorage {
    /// Downloads directory for source archives
    downloads_dir: PathBuf,
    /// Build cache directory for compiled artifacts
    build_cache_dir: PathBuf,
}

impl SharedStorage {
    /// Create a new shared storage manager
    ///
    /// # Arguments
    ///
    /// * `dirs` - Platform-specific directory provider
    #[must_use]
    pub fn new(dirs: &ZigrootDirs) -> Self {
        Self {
            downloads_dir: dirs.downloads_dir(),
            build_cache_dir: dirs.build_cache_dir(),
        }
    }

    /// Get the path for a downloaded source archive
    ///
    /// The path is structured as:
    /// `<downloads_dir>/<package>/<version>/<sha256_prefix>/<filename>`
    ///
    /// This allows sharing downloads across projects while avoiding
    /// conflicts between different versions or sources.
    ///
    /// # Arguments
    ///
    /// * `package` - Package name
    /// * `version` - Package version
    /// * `sha256` - SHA256 checksum of the source archive
    ///
    /// # Returns
    ///
    /// Path to the downloaded source archive
    #[must_use]
    pub fn download_path(&self, package: &str, version: &str, sha256: &str) -> PathBuf {
        let sha_prefix = &sha256[..8.min(sha256.len())];
        self.downloads_dir
            .join(package)
            .join(version)
            .join(sha_prefix)
            .join(format!("{package}-{version}.tar.gz"))
    }

    /// Check if a download exists
    ///
    /// # Arguments
    ///
    /// * `package` - Package name
    /// * `version` - Package version
    /// * `sha256` - SHA256 checksum of the source archive
    ///
    /// # Returns
    ///
    /// `true` if the download exists, `false` otherwise
    #[must_use]
    pub fn download_exists(&self, package: &str, version: &str, sha256: &str) -> bool {
        self.download_path(package, version, sha256).exists()
    }

    /// Get the path for a cached build artifact
    ///
    /// The path is structured as:
    /// `<build_cache_dir>/<key_prefix>/<key>`
    ///
    /// This uses content-addressable storage where the key is a hash
    /// of the build inputs (package, version, sha256, target, compiler).
    ///
    /// # Arguments
    ///
    /// * `cache_key` - The cache key (typically a SHA256 hash)
    ///
    /// # Returns
    ///
    /// Path to the cached build artifact directory
    #[must_use]
    pub fn cache_path(&self, cache_key: &str) -> PathBuf {
        // Use first 2 characters as subdirectory for better filesystem distribution
        let prefix = &cache_key[..2.min(cache_key.len())];
        self.build_cache_dir.join(prefix).join(cache_key)
    }

    /// Check if a cache entry exists
    ///
    /// # Arguments
    ///
    /// * `cache_key` - The cache key
    ///
    /// # Returns
    ///
    /// `true` if the cache entry exists, `false` otherwise
    #[must_use]
    pub fn cache_exists(&self, cache_key: &str) -> bool {
        self.cache_path(cache_key).exists()
    }

    /// Compute a cache key from build inputs
    ///
    /// The cache key is a SHA256 hash of:
    /// - Package name
    /// - Package version
    /// - Source SHA256 checksum
    /// - Target triple
    /// - Compiler version
    ///
    /// This ensures that cached artifacts are only reused when all
    /// inputs match exactly.
    ///
    /// # Arguments
    ///
    /// * `package` - Package name
    /// * `version` - Package version
    /// * `sha256` - SHA256 checksum of the source
    /// * `target` - Target triple (e.g., "arm-linux-musleabihf")
    /// * `compiler_version` - Compiler version (e.g., "0.11.0")
    ///
    /// # Returns
    ///
    /// A 64-character hex string representing the cache key
    #[must_use]
    pub fn compute_cache_key(
        package: &str,
        version: &str,
        sha256: &str,
        target: &str,
        compiler_version: &str,
    ) -> String {
        let mut hasher = Sha256::new();
        hasher.update(package.as_bytes());
        hasher.update(b"\0");
        hasher.update(version.as_bytes());
        hasher.update(b"\0");
        hasher.update(sha256.as_bytes());
        hasher.update(b"\0");
        hasher.update(target.as_bytes());
        hasher.update(b"\0");
        hasher.update(compiler_version.as_bytes());

        hex::encode(hasher.finalize())
    }

    /// Get the downloads directory
    #[must_use]
    pub fn downloads_dir(&self) -> &PathBuf {
        &self.downloads_dir
    }

    /// Get the build cache directory
    #[must_use]
    pub fn build_cache_dir(&self) -> &PathBuf {
        &self.build_cache_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_storage() -> (SharedStorage, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let dirs = ZigrootDirs::new();
        // Override with temp paths
        let storage = SharedStorage {
            downloads_dir: temp_dir.path().join("downloads"),
            build_cache_dir: temp_dir.path().join("cache"),
        };
        (storage, temp_dir)
    }

    #[test]
    fn test_download_path_structure() {
        let (storage, _temp) = create_test_storage();
        let path = storage.download_path("busybox", "1.36.1", "abc123def456");

        let path_str = path.to_string_lossy();
        assert!(path_str.contains("busybox"));
        assert!(path_str.contains("1.36.1"));
        assert!(path_str.contains("abc123de")); // First 8 chars of sha
    }

    #[test]
    fn test_cache_path_structure() {
        let (storage, _temp) = create_test_storage();
        let key = "abc123def456789012345678901234567890123456789012345678901234";
        let path = storage.cache_path(key);

        let path_str = path.to_string_lossy();
        assert!(path_str.contains("ab")); // First 2 chars as prefix
        assert!(path_str.contains(key));
    }

    #[test]
    fn test_cache_key_determinism() {
        let key1 = SharedStorage::compute_cache_key(
            "busybox",
            "1.36.1",
            "abc123",
            "arm-linux-musleabihf",
            "0.11.0",
        );
        let key2 = SharedStorage::compute_cache_key(
            "busybox",
            "1.36.1",
            "abc123",
            "arm-linux-musleabihf",
            "0.11.0",
        );
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_cache_key_uniqueness() {
        let key1 = SharedStorage::compute_cache_key(
            "busybox",
            "1.36.1",
            "abc123",
            "arm-linux-musleabihf",
            "0.11.0",
        );
        let key2 = SharedStorage::compute_cache_key(
            "busybox",
            "1.36.2", // Different version
            "abc123",
            "arm-linux-musleabihf",
            "0.11.0",
        );
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_cache_key_format() {
        let key = SharedStorage::compute_cache_key(
            "busybox",
            "1.36.1",
            "abc123",
            "arm-linux-musleabihf",
            "0.11.0",
        );
        assert_eq!(key.len(), 64);
        assert!(key.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_download_exists() {
        let (storage, temp) = create_test_storage();

        // Initially doesn't exist
        assert!(!storage.download_exists("busybox", "1.36.1", "abc123"));

        // Create the file
        let path = storage.download_path("busybox", "1.36.1", "abc123");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "test").unwrap();

        // Now exists
        assert!(storage.download_exists("busybox", "1.36.1", "abc123"));

        drop(temp);
    }

    #[test]
    fn test_cache_exists() {
        let (storage, temp) = create_test_storage();
        let key = "abc123def456789012345678901234567890123456789012345678901234";

        // Initially doesn't exist
        assert!(!storage.cache_exists(key));

        // Create the directory
        let path = storage.cache_path(key);
        std::fs::create_dir_all(&path).unwrap();

        // Now exists
        assert!(storage.cache_exists(key));

        drop(temp);
    }
}
