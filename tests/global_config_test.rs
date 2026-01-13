//! Integration tests for global configuration
//!
//! Tests for Requirements 32.5, 32.6:
//! - Reads config.toml from config directory
//! - Global settings include registry URLs, cache TTL, default build options
//!
//! **Validates: Requirements 32.5, 32.6**

use std::path::PathBuf;
use tempfile::TempDir;

/// Test: Global config module exists and can load config
/// **Validates: Requirement 32.5**
#[test]
fn test_global_config_loads_from_path() {
    use zigroot::core::global_config::GlobalConfig;

    // Create a temp directory for config
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create a config.toml file
    let config_content = r#"
[registry]
packages_url = "https://custom.example.com/packages"
boards_url = "https://custom.example.com/boards"

[cache]
ttl = 7200

[build]
compress = true
jobs = 8

[output]
color = true
quiet = false
"#;
    let config_path = temp_dir.path().join("config.toml");
    std::fs::write(&config_path, config_content).expect("Failed to write config file");

    // Load global config directly from path
    let config = GlobalConfig::load_from_path(&config_path).expect("Failed to load global config");

    // Verify config was loaded
    assert_eq!(
        config.registry.packages_url,
        Some("https://custom.example.com/packages".to_string())
    );
    assert_eq!(
        config.registry.boards_url,
        Some("https://custom.example.com/boards".to_string())
    );
    assert_eq!(config.cache.ttl, Some(7200));
    assert_eq!(config.build.compress, Some(true));
    assert_eq!(config.build.jobs, Some(8));
    assert_eq!(config.output.color, Some(true));
    assert_eq!(config.output.quiet, Some(false));
}

/// Test: Global config returns defaults when no config file exists
/// **Validates: Requirement 32.5**
#[test]
fn test_global_config_returns_defaults_when_missing() {
    use zigroot::core::global_config::GlobalConfig;

    // Create a temp directory with no config file
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("config.toml");

    // Load global config (should return defaults)
    let config = GlobalConfig::load_from_path(&config_path).expect("Failed to load global config");

    // Verify defaults are used
    assert!(config.registry.packages_url.is_none());
    assert!(config.registry.boards_url.is_none());
    assert!(config.cache.ttl.is_none());
    assert!(config.build.compress.is_none());
    assert!(config.build.jobs.is_none());
}

/// Test: Global config supports registry URL settings
/// **Validates: Requirement 32.6**
#[test]
fn test_global_config_registry_urls() {
    use zigroot::core::global_config::GlobalConfig;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let config_content = r#"
[registry]
packages_url = "https://my-registry.example.com/packages"
boards_url = "https://my-registry.example.com/boards"
"#;
    let config_path = temp_dir.path().join("config.toml");
    std::fs::write(&config_path, config_content).expect("Failed to write config file");

    let config = GlobalConfig::load_from_path(&config_path).expect("Failed to load global config");

    assert_eq!(
        config.registry.packages_url,
        Some("https://my-registry.example.com/packages".to_string())
    );
    assert_eq!(
        config.registry.boards_url,
        Some("https://my-registry.example.com/boards".to_string())
    );
}

/// Test: Global config supports cache TTL setting
/// **Validates: Requirement 32.6**
#[test]
fn test_global_config_cache_ttl() {
    use zigroot::core::global_config::GlobalConfig;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let config_content = r#"
[cache]
ttl = 3600
"#;
    let config_path = temp_dir.path().join("config.toml");
    std::fs::write(&config_path, config_content).expect("Failed to write config file");

    let config = GlobalConfig::load_from_path(&config_path).expect("Failed to load global config");

    assert_eq!(config.cache.ttl, Some(3600));
}

/// Test: Global config supports default build options
/// **Validates: Requirement 32.6**
#[test]
fn test_global_config_build_options() {
    use zigroot::core::global_config::GlobalConfig;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let config_content = r#"
[build]
compress = false
jobs = 4
sandbox = true
"#;
    let config_path = temp_dir.path().join("config.toml");
    std::fs::write(&config_path, config_content).expect("Failed to write config file");

    let config = GlobalConfig::load_from_path(&config_path).expect("Failed to load global config");

    assert_eq!(config.build.compress, Some(false));
    assert_eq!(config.build.jobs, Some(4));
    assert_eq!(config.build.sandbox, Some(true));
}

/// Test: Global config supports output preferences
/// **Validates: Requirement 32.6**
#[test]
fn test_global_config_output_preferences() {
    use zigroot::core::global_config::GlobalConfig;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let config_content = r#"
[output]
color = false
quiet = true
json = true
"#;
    let config_path = temp_dir.path().join("config.toml");
    std::fs::write(&config_path, config_content).expect("Failed to write config file");

    let config = GlobalConfig::load_from_path(&config_path).expect("Failed to load global config");

    assert_eq!(config.output.color, Some(false));
    assert_eq!(config.output.quiet, Some(true));
    assert_eq!(config.output.json, Some(true));
}

/// Test: Global config supports update check settings
/// **Validates: Requirement 32.6**
#[test]
fn test_global_config_update_settings() {
    use zigroot::core::global_config::GlobalConfig;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let config_content = r#"
[update]
check_enabled = false
check_interval = 86400
"#;
    let config_path = temp_dir.path().join("config.toml");
    std::fs::write(&config_path, config_content).expect("Failed to write config file");

    let config = GlobalConfig::load_from_path(&config_path).expect("Failed to load global config");

    assert_eq!(config.update.check_enabled, Some(false));
    assert_eq!(config.update.check_interval, Some(86400));
}

/// Test: Global config handles invalid TOML gracefully
/// **Validates: Requirement 32.5**
#[test]
fn test_global_config_handles_invalid_toml() {
    use zigroot::core::global_config::GlobalConfig;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Write invalid TOML
    let config_content = "this is not valid toml [[[";
    let config_path = temp_dir.path().join("config.toml");
    std::fs::write(&config_path, config_content).expect("Failed to write config file");

    let result = GlobalConfig::load_from_path(&config_path);

    // Should return an error for invalid TOML
    assert!(result.is_err(), "Should return error for invalid TOML");
}

/// Test: Global config path is correct
/// **Validates: Requirement 32.5**
#[test]
fn test_global_config_path() {
    use zigroot::infra::dirs::ZigrootDirs;

    // Use default dirs (no env override)
    let dirs = ZigrootDirs::new();
    let global_config_path = dirs.global_config_path();

    assert!(
        global_config_path.ends_with("config.toml"),
        "Global config path should end with config.toml: {}",
        global_config_path.display()
    );

    // Path should contain zigroot
    let path_str = global_config_path.to_string_lossy();
    assert!(
        path_str.contains("zigroot"),
        "Global config path should contain 'zigroot': {path_str}"
    );
}

/// Test: Global config save and load roundtrip
/// **Validates: Requirement 32.5**
#[test]
fn test_global_config_save_and_load_roundtrip() {
    use zigroot::core::global_config::{
        BuildConfig, CacheConfig, GlobalConfig, OutputConfig, RegistryConfig, UpdateConfig,
    };

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
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

    config.save_to_path(&config_path).expect("Failed to save config");
    let loaded = GlobalConfig::load_from_path(&config_path).expect("Failed to load config");

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

/// Test: Global config effective values with defaults
/// **Validates: Requirement 32.6**
#[test]
fn test_global_config_effective_values() {
    use zigroot::core::global_config::GlobalConfig;

    // Empty config should use defaults
    let config = GlobalConfig::default();

    // Should return default URLs
    assert!(!config.packages_url().is_empty());
    assert!(!config.boards_url().is_empty());

    // Should return default TTL
    assert!(config.cache_ttl() > 0);

    // Should return default jobs
    assert!(config.build_jobs() > 0);
}
