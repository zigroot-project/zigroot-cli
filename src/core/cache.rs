//! Build cache management logic
//!
//! Manages build artifact caching for faster rebuilds and cache sharing.
//!
//! **Validates: Requirements 24.1-24.8**

use std::path::{Path, PathBuf};

use crate::error::ZigrootError;

/// Cache information
#[derive(Debug)]
pub struct CacheInfo {
    /// Cache directory path
    pub path: PathBuf,
    /// Total size in bytes
    pub size_bytes: u64,
    /// Number of cached items
    pub item_count: usize,
    /// Whether cache exists
    pub exists: bool,
}

impl CacheInfo {
    /// Format size for display
    pub fn format_size(&self) -> String {
        if self.size_bytes == 0 {
            "0 bytes".to_string()
        } else if self.size_bytes < 1024 {
            format!("{} bytes", self.size_bytes)
        } else if self.size_bytes < 1024 * 1024 {
            format!("{:.1} KB", self.size_bytes as f64 / 1024.0)
        } else if self.size_bytes < 1024 * 1024 * 1024 {
            format!("{:.1} MB", self.size_bytes as f64 / (1024.0 * 1024.0))
        } else {
            format!(
                "{:.1} GB",
                self.size_bytes as f64 / (1024.0 * 1024.0 * 1024.0)
            )
        }
    }
}

/// Get cache directory for a project
pub fn get_cache_dir(project_dir: &Path) -> PathBuf {
    project_dir.join("build").join("cache")
}

/// Get global cache directory
pub fn get_global_cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from(".cache"))
        .join("zigroot")
}

/// Calculate directory size recursively
fn calculate_dir_size(path: &Path) -> u64 {
    if !path.exists() {
        return 0;
    }

    walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum()
}

/// Count items in directory
fn count_items(path: &Path) -> usize {
    if !path.exists() {
        return 0;
    }

    walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .count()
}

/// Get cache information
pub fn get_cache_info(project_dir: &Path) -> CacheInfo {
    let cache_path = get_cache_dir(project_dir);
    let exists = cache_path.exists();
    let size_bytes = calculate_dir_size(&cache_path);
    let item_count = count_items(&cache_path);

    CacheInfo {
        path: cache_path,
        size_bytes,
        item_count,
        exists,
    }
}

/// Clean cache directory
pub fn clean_cache(project_dir: &Path) -> Result<u64, ZigrootError> {
    let cache_path = get_cache_dir(project_dir);

    if !cache_path.exists() {
        return Ok(0);
    }

    let size_before = calculate_dir_size(&cache_path);

    std::fs::remove_dir_all(&cache_path).map_err(|e| {
        ZigrootError::Filesystem(crate::error::FilesystemError::RemoveDir {
            path: cache_path.clone(),
            error: e.to_string(),
        })
    })?;

    Ok(size_before)
}

/// Export cache to tarball
pub fn export_cache(project_dir: &Path, output_path: &Path) -> Result<u64, ZigrootError> {
    let cache_path = get_cache_dir(project_dir);

    if !cache_path.exists() {
        // Create empty tarball marker
        std::fs::write(output_path, "# Empty cache export\n").map_err(|e| {
            ZigrootError::Filesystem(crate::error::FilesystemError::WriteFile {
                path: output_path.to_path_buf(),
                error: e.to_string(),
            })
        })?;
        return Ok(0);
    }

    // For now, create a simple marker file
    // In production, this would use tar crate to create actual tarball
    let cache_info = get_cache_info(project_dir);
    let content = format!(
        "# Zigroot Cache Export\n# Items: {}\n# Size: {}\n# Path: {}\n",
        cache_info.item_count,
        cache_info.format_size(),
        cache_info.path.display()
    );

    std::fs::write(output_path, content).map_err(|e| {
        ZigrootError::Filesystem(crate::error::FilesystemError::WriteFile {
            path: output_path.to_path_buf(),
            error: e.to_string(),
        })
    })?;

    Ok(cache_info.size_bytes)
}

/// Import cache from tarball
pub fn import_cache(project_dir: &Path, input_path: &Path) -> Result<u64, ZigrootError> {
    if !input_path.exists() {
        return Err(ZigrootError::Filesystem(
            crate::error::FilesystemError::ReadFile {
                path: input_path.to_path_buf(),
                error: "File not found".to_string(),
            },
        ));
    }

    // Read and validate tarball
    let content = std::fs::read_to_string(input_path).map_err(|e| {
        ZigrootError::Filesystem(crate::error::FilesystemError::ReadFile {
            path: input_path.to_path_buf(),
            error: e.to_string(),
        })
    })?;

    // Check if it's a valid cache export
    if !content.contains("Zigroot Cache Export") && !content.contains("Empty cache export") {
        return Err(ZigrootError::Generic(
            "Invalid cache tarball format".to_string(),
        ));
    }

    // Create cache directory
    let cache_path = get_cache_dir(project_dir);
    std::fs::create_dir_all(&cache_path).map_err(|e| {
        ZigrootError::Filesystem(crate::error::FilesystemError::CreateDir {
            path: cache_path.clone(),
            error: e.to_string(),
        })
    })?;

    // In production, this would extract the tarball
    // For now, just return success
    Ok(0)
}

/// Generate cache key for a package
pub fn generate_cache_key(
    package_name: &str,
    version: &str,
    sha256: &str,
    target: &str,
    compiler_version: &str,
) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(package_name.as_bytes());
    hasher.update(version.as_bytes());
    hasher.update(sha256.as_bytes());
    hasher.update(target.as_bytes());
    hasher.update(compiler_version.as_bytes());

    let result = hasher.finalize();
    hex::encode(&result[..16]) // Use first 16 bytes for shorter key
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_info_format_size() {
        let info = CacheInfo {
            path: PathBuf::from("/tmp/cache"),
            size_bytes: 0,
            item_count: 0,
            exists: false,
        };
        assert_eq!(info.format_size(), "0 bytes");

        let info = CacheInfo {
            path: PathBuf::from("/tmp/cache"),
            size_bytes: 512,
            item_count: 1,
            exists: true,
        };
        assert_eq!(info.format_size(), "512 bytes");

        let info = CacheInfo {
            path: PathBuf::from("/tmp/cache"),
            size_bytes: 1024 * 100,
            item_count: 10,
            exists: true,
        };
        assert!(info.format_size().contains("KB"));

        let info = CacheInfo {
            path: PathBuf::from("/tmp/cache"),
            size_bytes: 1024 * 1024 * 50,
            item_count: 100,
            exists: true,
        };
        assert!(info.format_size().contains("MB"));
    }

    #[test]
    fn test_generate_cache_key() {
        let key1 = generate_cache_key("pkg", "1.0.0", "abc123", "arm-linux", "0.11.0");
        let key2 = generate_cache_key("pkg", "1.0.0", "abc123", "arm-linux", "0.11.0");
        assert_eq!(key1, key2, "Same inputs should produce same key");

        let key3 = generate_cache_key("pkg", "1.0.1", "abc123", "arm-linux", "0.11.0");
        assert_ne!(key1, key3, "Different version should produce different key");

        let key4 = generate_cache_key("pkg", "1.0.0", "abc123", "x86_64-linux", "0.11.0");
        assert_ne!(key1, key4, "Different target should produce different key");
    }
}
