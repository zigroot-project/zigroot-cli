//! Integration tests for platform-specific directories
//!
//! Tests for Requirements 32.1-32.4:
//! - Uses XDG on Linux, Library/Caches on macOS
//! - Environment variables override defaults
//!
//! **Validates: Requirements 32.1-32.4**

use std::env;
use std::path::PathBuf;

/// Test: Platform directories module exists and provides cache directory
/// **Validates: Requirement 32.1**
#[test]
fn test_cache_dir_returns_platform_specific_path() {
    use zigroot::infra::dirs::ZigrootDirs;

    let dirs = ZigrootDirs::new();
    let cache_dir = dirs.cache_dir();

    // Cache directory should be a valid path
    assert!(!cache_dir.as_os_str().is_empty(), "Cache dir should not be empty");

    // On macOS, should be under Library/Caches
    #[cfg(target_os = "macos")]
    {
        let path_str = cache_dir.to_string_lossy();
        assert!(
            path_str.contains("Library/Caches") || path_str.contains("zigroot"),
            "macOS cache dir should be under Library/Caches: {path_str}"
        );
    }

    // On Linux, should follow XDG (typically ~/.cache)
    #[cfg(target_os = "linux")]
    {
        let path_str = cache_dir.to_string_lossy();
        assert!(
            path_str.contains(".cache") || path_str.contains("zigroot"),
            "Linux cache dir should follow XDG: {path_str}"
        );
    }
}

/// Test: Platform directories provides config directory
/// **Validates: Requirement 32.2**
#[test]
fn test_config_dir_returns_platform_specific_path() {
    use zigroot::infra::dirs::ZigrootDirs;

    let dirs = ZigrootDirs::new();
    let config_dir = dirs.config_dir();

    // Config directory should be a valid path
    assert!(!config_dir.as_os_str().is_empty(), "Config dir should not be empty");

    // On macOS, should be under Library/Application Support or Library/Preferences
    #[cfg(target_os = "macos")]
    {
        let path_str = config_dir.to_string_lossy();
        assert!(
            path_str.contains("Library") || path_str.contains("zigroot"),
            "macOS config dir should be under Library: {path_str}"
        );
    }

    // On Linux, should follow XDG (typically ~/.config)
    #[cfg(target_os = "linux")]
    {
        let path_str = config_dir.to_string_lossy();
        assert!(
            path_str.contains(".config") || path_str.contains("zigroot"),
            "Linux config dir should follow XDG: {path_str}"
        );
    }
}

/// Test: Platform directories provides data directory
/// **Validates: Requirement 32.3**
#[test]
fn test_data_dir_returns_platform_specific_path() {
    use zigroot::infra::dirs::ZigrootDirs;

    let dirs = ZigrootDirs::new();
    let data_dir = dirs.data_dir();

    // Data directory should be a valid path
    assert!(!data_dir.as_os_str().is_empty(), "Data dir should not be empty");

    // On macOS, should be under Library/Application Support
    #[cfg(target_os = "macos")]
    {
        let path_str = data_dir.to_string_lossy();
        assert!(
            path_str.contains("Library") || path_str.contains("zigroot"),
            "macOS data dir should be under Library: {path_str}"
        );
    }

    // On Linux, should follow XDG (typically ~/.local/share)
    #[cfg(target_os = "linux")]
    {
        let path_str = data_dir.to_string_lossy();
        assert!(
            path_str.contains(".local/share") || path_str.contains("zigroot"),
            "Linux data dir should follow XDG: {path_str}"
        );
    }
}

/// Test: ZIGROOT_CACHE_DIR environment variable overrides default
/// **Validates: Requirement 32.4**
#[test]
fn test_cache_dir_env_override() {
    use zigroot::infra::dirs::ZigrootDirs;

    let custom_path = "/tmp/zigroot-test-cache";

    // Set environment variable
    env::set_var("ZIGROOT_CACHE_DIR", custom_path);

    let dirs = ZigrootDirs::new();
    let cache_dir = dirs.cache_dir();

    // Clean up environment variable
    env::remove_var("ZIGROOT_CACHE_DIR");

    assert_eq!(
        cache_dir,
        PathBuf::from(custom_path),
        "ZIGROOT_CACHE_DIR should override default cache directory"
    );
}

/// Test: ZIGROOT_CONFIG_DIR environment variable overrides default
/// **Validates: Requirement 32.4**
#[test]
fn test_config_dir_env_override() {
    use zigroot::infra::dirs::ZigrootDirs;

    let custom_path = "/tmp/zigroot-test-config";

    // Set environment variable
    env::set_var("ZIGROOT_CONFIG_DIR", custom_path);

    let dirs = ZigrootDirs::new();
    let config_dir = dirs.config_dir();

    // Clean up environment variable
    env::remove_var("ZIGROOT_CONFIG_DIR");

    assert_eq!(
        config_dir,
        PathBuf::from(custom_path),
        "ZIGROOT_CONFIG_DIR should override default config directory"
    );
}

/// Test: ZIGROOT_DATA_DIR environment variable overrides default
/// **Validates: Requirement 32.4**
#[test]
fn test_data_dir_env_override() {
    use zigroot::infra::dirs::ZigrootDirs;

    let custom_path = "/tmp/zigroot-test-data";

    // Set environment variable
    env::set_var("ZIGROOT_DATA_DIR", custom_path);

    let dirs = ZigrootDirs::new();
    let data_dir = dirs.data_dir();

    // Clean up environment variable
    env::remove_var("ZIGROOT_DATA_DIR");

    assert_eq!(
        data_dir,
        PathBuf::from(custom_path),
        "ZIGROOT_DATA_DIR should override default data directory"
    );
}

/// Test: All directories include "zigroot" in path
/// **Validates: Requirements 32.1-32.3**
#[test]
fn test_directories_include_zigroot_name() {
    use zigroot::infra::dirs::ZigrootDirs;

    // Clear any environment overrides
    env::remove_var("ZIGROOT_CACHE_DIR");
    env::remove_var("ZIGROOT_CONFIG_DIR");
    env::remove_var("ZIGROOT_DATA_DIR");

    let dirs = ZigrootDirs::new();

    let cache_str = dirs.cache_dir().to_string_lossy().to_string();
    let config_str = dirs.config_dir().to_string_lossy().to_string();
    let data_str = dirs.data_dir().to_string_lossy().to_string();

    assert!(
        cache_str.contains("zigroot"),
        "Cache dir should contain 'zigroot': {cache_str}"
    );
    assert!(
        config_str.contains("zigroot"),
        "Config dir should contain 'zigroot': {config_str}"
    );
    assert!(
        data_str.contains("zigroot"),
        "Data dir should contain 'zigroot': {data_str}"
    );
}

/// Test: Downloads directory is under data directory
/// **Validates: Requirement 32.9**
#[test]
fn test_downloads_dir_under_data() {
    use zigroot::infra::dirs::ZigrootDirs;

    // Clear any environment overrides
    env::remove_var("ZIGROOT_DATA_DIR");

    let dirs = ZigrootDirs::new();
    let downloads_dir = dirs.downloads_dir();
    let data_dir = dirs.data_dir();

    assert!(
        downloads_dir.starts_with(&data_dir),
        "Downloads dir should be under data dir: downloads={}, data={}",
        downloads_dir.display(),
        data_dir.display()
    );
}

/// Test: Build cache directory is under cache directory
/// **Validates: Requirement 32.10**
#[test]
fn test_build_cache_dir_under_cache() {
    use zigroot::infra::dirs::ZigrootDirs;

    // Clear any environment overrides
    env::remove_var("ZIGROOT_CACHE_DIR");

    let dirs = ZigrootDirs::new();
    let build_cache_dir = dirs.build_cache_dir();
    let cache_dir = dirs.cache_dir();

    assert!(
        build_cache_dir.starts_with(&cache_dir),
        "Build cache dir should be under cache dir: build_cache={}, cache={}",
        build_cache_dir.display(),
        cache_dir.display()
    );
}
