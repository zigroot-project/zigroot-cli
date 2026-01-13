//! Platform-specific directory management
//!
//! Provides platform-specific paths for cache, config, and data directories.
//! Follows XDG Base Directory Specification on Linux and standard locations on macOS.
//!
//! Environment variables can override default directories:
//! - `ZIGROOT_CACHE_DIR` - Override cache directory
//! - `ZIGROOT_CONFIG_DIR` - Override config directory
//! - `ZIGROOT_DATA_DIR` - Override data directory
//!
//! **Validates: Requirements 32.1-32.4**

use std::env;
use std::path::PathBuf;

/// Environment variable names for directory overrides
pub const ENV_CACHE_DIR: &str = "ZIGROOT_CACHE_DIR";
pub const ENV_CONFIG_DIR: &str = "ZIGROOT_CONFIG_DIR";
pub const ENV_DATA_DIR: &str = "ZIGROOT_DATA_DIR";

/// Application name used in directory paths
const APP_NAME: &str = "zigroot";

/// Subdirectory names
const DOWNLOADS_SUBDIR: &str = "downloads";
const BUILD_CACHE_SUBDIR: &str = "build-cache";

/// Platform-specific directory provider for zigroot
///
/// Provides paths to cache, config, and data directories following
/// platform conventions (XDG on Linux, Library on macOS).
#[derive(Debug, Clone)]
pub struct ZigrootDirs {
    cache_dir: PathBuf,
    config_dir: PathBuf,
    data_dir: PathBuf,
}

impl ZigrootDirs {
    /// Create a new `ZigrootDirs` instance
    ///
    /// Checks environment variables first, then falls back to platform defaults.
    #[must_use]
    pub fn new() -> Self {
        Self {
            cache_dir: Self::resolve_cache_dir(),
            config_dir: Self::resolve_config_dir(),
            data_dir: Self::resolve_data_dir(),
        }
    }

    /// Get the cache directory path
    ///
    /// Used for temporary cached data that can be regenerated.
    /// - Linux: `$XDG_CACHE_HOME/zigroot` or `~/.cache/zigroot`
    /// - macOS: `~/Library/Caches/zigroot`
    #[must_use]
    pub fn cache_dir(&self) -> PathBuf {
        self.cache_dir.clone()
    }

    /// Get the config directory path
    ///
    /// Used for user configuration files.
    /// - Linux: `$XDG_CONFIG_HOME/zigroot` or `~/.config/zigroot`
    /// - macOS: `~/Library/Application Support/zigroot`
    #[must_use]
    pub fn config_dir(&self) -> PathBuf {
        self.config_dir.clone()
    }

    /// Get the data directory path
    ///
    /// Used for persistent data like downloaded sources.
    /// - Linux: `$XDG_DATA_HOME/zigroot` or `~/.local/share/zigroot`
    /// - macOS: `~/Library/Application Support/zigroot`
    #[must_use]
    pub fn data_dir(&self) -> PathBuf {
        self.data_dir.clone()
    }

    /// Get the downloads directory path
    ///
    /// Used for shared source archives across projects.
    /// Located under the data directory.
    #[must_use]
    pub fn downloads_dir(&self) -> PathBuf {
        self.data_dir.join(DOWNLOADS_SUBDIR)
    }

    /// Get the build cache directory path
    ///
    /// Used for content-addressable build cache.
    /// Located under the cache directory.
    #[must_use]
    pub fn build_cache_dir(&self) -> PathBuf {
        self.cache_dir.join(BUILD_CACHE_SUBDIR)
    }

    /// Get the global config file path
    ///
    /// Returns the path to `config.toml` in the config directory.
    #[must_use]
    pub fn global_config_path(&self) -> PathBuf {
        self.config_dir.join("config.toml")
    }

    /// Resolve cache directory from environment or platform default
    fn resolve_cache_dir() -> PathBuf {
        if let Ok(path) = env::var(ENV_CACHE_DIR) {
            return PathBuf::from(path);
        }

        Self::platform_cache_dir()
    }

    /// Resolve config directory from environment or platform default
    fn resolve_config_dir() -> PathBuf {
        if let Ok(path) = env::var(ENV_CONFIG_DIR) {
            return PathBuf::from(path);
        }

        Self::platform_config_dir()
    }

    /// Resolve data directory from environment or platform default
    fn resolve_data_dir() -> PathBuf {
        if let Ok(path) = env::var(ENV_DATA_DIR) {
            return PathBuf::from(path);
        }

        Self::platform_data_dir()
    }

    /// Get platform-specific cache directory
    fn platform_cache_dir() -> PathBuf {
        dirs::cache_dir()
            .map(|p| p.join(APP_NAME))
            .unwrap_or_else(|| {
                // Fallback to home directory
                dirs::home_dir()
                    .map(|h| h.join(".cache").join(APP_NAME))
                    .unwrap_or_else(|| PathBuf::from(".").join(".cache").join(APP_NAME))
            })
    }

    /// Get platform-specific config directory
    fn platform_config_dir() -> PathBuf {
        dirs::config_dir()
            .map(|p| p.join(APP_NAME))
            .unwrap_or_else(|| {
                // Fallback to home directory
                dirs::home_dir()
                    .map(|h| h.join(".config").join(APP_NAME))
                    .unwrap_or_else(|| PathBuf::from(".").join(".config").join(APP_NAME))
            })
    }

    /// Get platform-specific data directory
    fn platform_data_dir() -> PathBuf {
        dirs::data_dir()
            .map(|p| p.join(APP_NAME))
            .unwrap_or_else(|| {
                // Fallback to home directory
                dirs::home_dir()
                    .map(|h| h.join(".local").join("share").join(APP_NAME))
                    .unwrap_or_else(|| {
                        PathBuf::from(".")
                            .join(".local")
                            .join("share")
                            .join(APP_NAME)
                    })
            })
    }
}

impl Default for ZigrootDirs {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dirs_new_creates_instance() {
        let dirs = ZigrootDirs::new();
        assert!(!dirs.cache_dir().as_os_str().is_empty());
        assert!(!dirs.config_dir().as_os_str().is_empty());
        assert!(!dirs.data_dir().as_os_str().is_empty());
    }

    #[test]
    fn test_downloads_dir_is_under_data_dir() {
        let dirs = ZigrootDirs::new();
        assert!(dirs.downloads_dir().starts_with(dirs.data_dir()));
    }

    #[test]
    fn test_build_cache_dir_is_under_cache_dir() {
        let dirs = ZigrootDirs::new();
        assert!(dirs.build_cache_dir().starts_with(dirs.cache_dir()));
    }

    #[test]
    fn test_global_config_path_is_under_config_dir() {
        let dirs = ZigrootDirs::new();
        assert!(dirs.global_config_path().starts_with(dirs.config_dir()));
        assert!(dirs.global_config_path().ends_with("config.toml"));
    }
}
