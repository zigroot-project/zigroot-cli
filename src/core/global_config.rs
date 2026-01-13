//! Global configuration management
//!
//! Reads and manages global settings from `config.toml` in the config directory.
//! Global settings include registry URLs, cache TTL, default build options,
//! update check settings, and output preferences.
//!
//! **Validates: Requirements 32.5, 32.6**

use crate::infra::dirs::ZigrootDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use thiserror::Error;

/// Global configuration error types
#[derive(Error, Debug)]
pub enum GlobalConfigError {
    /// Failed to read config file
    #[error("Failed to read config file '{path}': {error}")]
    ReadError { path: String, error: String },

    /// Failed to parse config file
    #[error("Failed to parse config file '{path}': {error}")]
    ParseError { path: String, error: String },
}

/// Global configuration for zigroot
///
/// Contains all global settings that apply across projects.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GlobalConfig {
    /// Registry settings
    #[serde(default)]
    pub registry: RegistryConfig,

    /// Cache settings
    #[serde(default)]
    pub cache: CacheConfig,

    /// Default build options
    #[serde(default)]
    pub build: BuildConfig,

    /// Output preferences
    #[serde(default)]
    pub output: OutputConfig,

    /// Update check settings
    #[serde(default)]
    pub update: UpdateConfig,
}

/// Registry configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RegistryConfig {
    /// Custom packages registry URL
    pub packages_url: Option<String>,

    /// Custom boards registry URL
    pub boards_url: Option<String>,
}

/// Cache configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Cache TTL in seconds
    pub ttl: Option<u64>,
}

/// Default build options
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BuildConfig {
    /// Enable compression by default
    pub compress: Option<bool>,

    /// Default number of parallel jobs
    pub jobs: Option<usize>,

    /// Enable sandbox by default
    pub sandbox: Option<bool>,
}

/// Output preferences
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OutputConfig {
    /// Enable colored output
    pub color: Option<bool>,

    /// Enable quiet mode
    pub quiet: Option<bool>,

    /// Enable JSON output
    pub json: Option<bool>,
}

/// Update check settings
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateConfig {
    /// Enable automatic update checks
    pub check_enabled: Option<bool>,

    /// Update check interval in seconds
    pub check_interval: Option<u64>,
}

impl GlobalConfig {
    /// Load global configuration from the config directory
    ///
    /// If the config file doesn't exist, returns default configuration.
    /// If the config file exists but is invalid, returns an error.
    ///
    /// # Arguments
    ///
    /// * `dirs` - Platform-specific directory provider
    ///
    /// # Returns
    ///
    /// The loaded configuration or an error if parsing fails.
    ///
    /// # Errors
    ///
    /// Returns `GlobalConfigError::ParseError` if the config file exists but
    /// contains invalid TOML.
    pub fn load(dirs: &ZigrootDirs) -> Result<Self, GlobalConfigError> {
        let config_path = dirs.global_config_path();
        Self::load_from_path(&config_path)
    }

    /// Load global configuration from a specific path
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the config file
    ///
    /// # Returns
    ///
    /// The loaded configuration or an error if parsing fails.
    pub fn load_from_path(path: &Path) -> Result<Self, GlobalConfigError> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(path).map_err(|e| GlobalConfigError::ReadError {
            path: path.display().to_string(),
            error: e.to_string(),
        })?;

        toml::from_str(&content).map_err(|e| GlobalConfigError::ParseError {
            path: path.display().to_string(),
            error: e.to_string(),
        })
    }

    /// Save global configuration to the config directory
    ///
    /// Creates the config directory if it doesn't exist.
    ///
    /// # Arguments
    ///
    /// * `dirs` - Platform-specific directory provider
    ///
    /// # Returns
    ///
    /// Ok(()) on success, or an error if saving fails.
    pub fn save(&self, dirs: &ZigrootDirs) -> Result<(), GlobalConfigError> {
        let config_path = dirs.global_config_path();
        self.save_to_path(&config_path)
    }

    /// Save global configuration to a specific path
    ///
    /// Creates parent directories if they don't exist.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to save the config file
    ///
    /// # Returns
    ///
    /// Ok(()) on success, or an error if saving fails.
    pub fn save_to_path(&self, path: &Path) -> Result<(), GlobalConfigError> {
        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| GlobalConfigError::ReadError {
                path: parent.display().to_string(),
                error: e.to_string(),
            })?;
        }

        let content = toml::to_string_pretty(self).map_err(|e| GlobalConfigError::ParseError {
            path: path.display().to_string(),
            error: e.to_string(),
        })?;

        fs::write(path, content).map_err(|e| GlobalConfigError::ReadError {
            path: path.display().to_string(),
            error: e.to_string(),
        })
    }

    /// Get the effective packages registry URL
    ///
    /// Returns the custom URL if set, otherwise returns the default.
    #[must_use]
    pub fn packages_url(&self) -> &str {
        self.registry
            .packages_url
            .as_deref()
            .unwrap_or(crate::config::urls::PACKAGE_REGISTRY)
    }

    /// Get the effective boards registry URL
    ///
    /// Returns the custom URL if set, otherwise returns the default.
    #[must_use]
    pub fn boards_url(&self) -> &str {
        self.registry
            .boards_url
            .as_deref()
            .unwrap_or(crate::config::urls::BOARD_REGISTRY)
    }

    /// Get the effective cache TTL
    ///
    /// Returns the custom TTL if set, otherwise returns the default.
    #[must_use]
    pub fn cache_ttl(&self) -> u64 {
        self.cache
            .ttl
            .unwrap_or(crate::config::defaults::REGISTRY_CACHE_TTL)
    }

    /// Get the effective number of build jobs
    ///
    /// Returns the custom value if set, otherwise returns the default.
    #[must_use]
    pub fn build_jobs(&self) -> usize {
        self.build
            .jobs
            .unwrap_or(crate::config::defaults::DEFAULT_BUILD_JOBS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_default_config() {
        let config = GlobalConfig::default();
        assert!(config.registry.packages_url.is_none());
        assert!(config.registry.boards_url.is_none());
        assert!(config.cache.ttl.is_none());
        assert!(config.build.compress.is_none());
        assert!(config.build.jobs.is_none());
    }

    #[test]
    fn test_load_missing_file_returns_default() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let config = GlobalConfig::load_from_path(&config_path).unwrap();
        assert!(config.registry.packages_url.is_none());
    }

    #[test]
    fn test_load_valid_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let content = r#"
[registry]
packages_url = "https://example.com/packages"

[cache]
ttl = 3600
"#;
        fs::write(&config_path, content).unwrap();

        let config = GlobalConfig::load_from_path(&config_path).unwrap();
        assert_eq!(
            config.registry.packages_url,
            Some("https://example.com/packages".to_string())
        );
        assert_eq!(config.cache.ttl, Some(3600));
    }

    #[test]
    fn test_load_invalid_toml_returns_error() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        fs::write(&config_path, "invalid toml [[[").unwrap();

        let result = GlobalConfig::load_from_path(&config_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let config = GlobalConfig {
            registry: RegistryConfig {
                packages_url: Some("https://test.com/packages".to_string()),
                boards_url: Some("https://test.com/boards".to_string()),
            },
            cache: CacheConfig { ttl: Some(7200) },
            build: BuildConfig {
                compress: Some(true),
                jobs: Some(8),
                sandbox: Some(false),
            },
            output: OutputConfig {
                color: Some(true),
                quiet: Some(false),
                json: Some(false),
            },
            update: UpdateConfig {
                check_enabled: Some(true),
                check_interval: Some(86400),
            },
        };

        config.save_to_path(&config_path).unwrap();
        let loaded = GlobalConfig::load_from_path(&config_path).unwrap();

        assert_eq!(loaded.registry.packages_url, config.registry.packages_url);
        assert_eq!(loaded.registry.boards_url, config.registry.boards_url);
        assert_eq!(loaded.cache.ttl, config.cache.ttl);
        assert_eq!(loaded.build.compress, config.build.compress);
        assert_eq!(loaded.build.jobs, config.build.jobs);
        assert_eq!(loaded.build.sandbox, config.build.sandbox);
        assert_eq!(loaded.output.color, config.output.color);
        assert_eq!(loaded.output.quiet, config.output.quiet);
        assert_eq!(loaded.output.json, config.output.json);
        assert_eq!(loaded.update.check_enabled, config.update.check_enabled);
        assert_eq!(loaded.update.check_interval, config.update.check_interval);
    }
}
