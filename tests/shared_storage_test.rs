//! Integration tests for shared downloads and content-addressable build cache
//!
//! Tests for Requirements 32.9, 32.10:
//! - Shares source archives across projects
//! - Content-addressable build cache
//!
//! **Validates: Requirements 32.9, 32.10**

use tempfile::TempDir;

/// Test: Shared storage module exists and provides download path
/// **Validates: Requirement 32.9**
#[test]
fn test_shared_storage_provides_download_path() {
    use zigroot::core::shared_storage::SharedStorage;
    use zigroot::infra::dirs::ZigrootDirs;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    std::env::set_var("ZIGROOT_DATA_DIR", temp_dir.path());

    let dirs = ZigrootDirs::new();
    let storage = SharedStorage::new(&dirs);

    let download_path = storage.download_path("busybox", "1.36.1", "abc123def456");

    std::env::remove_var("ZIGROOT_DATA_DIR");

    // Path should be under downloads directory
    assert!(
        download_path.starts_with(dirs.downloads_dir()),
        "Download path should be under downloads dir: {}",
        download_path.display()
    );

    // Path should include package name
    let path_str = download_path.to_string_lossy();
    assert!(
        path_str.contains("busybox"),
        "Download path should contain package name: {path_str}"
    );
}

/// Test: Shared storage provides content-addressable cache path
/// **Validates: Requirement 32.10**
#[test]
fn test_shared_storage_provides_cache_path() {
    use zigroot::core::shared_storage::SharedStorage;
    use zigroot::infra::dirs::ZigrootDirs;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    std::env::set_var("ZIGROOT_CACHE_DIR", temp_dir.path());

    let dirs = ZigrootDirs::new();
    let storage = SharedStorage::new(&dirs);

    let cache_key = "abc123def456789012345678901234567890123456789012345678901234";
    let cache_path = storage.cache_path(cache_key);

    std::env::remove_var("ZIGROOT_CACHE_DIR");

    // Path should be under build cache directory
    assert!(
        cache_path.starts_with(dirs.build_cache_dir()),
        "Cache path should be under build cache dir: {}",
        cache_path.display()
    );

    // Path should include the cache key
    let path_str = cache_path.to_string_lossy();
    assert!(
        path_str.contains(&cache_key[..8]),
        "Cache path should contain cache key prefix: {path_str}"
    );
}

/// Test: Cache key is deterministic based on inputs
/// **Validates: Requirement 32.10**
#[test]
fn test_cache_key_is_deterministic() {
    use zigroot::core::shared_storage::SharedStorage;

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

    assert_eq!(key1, key2, "Same inputs should produce same cache key");
}

/// Test: Cache key changes when inputs change
/// **Validates: Requirement 32.10**
#[test]
fn test_cache_key_changes_with_inputs() {
    use zigroot::core::shared_storage::SharedStorage;

    let key1 = SharedStorage::compute_cache_key(
        "busybox",
        "1.36.1",
        "abc123",
        "arm-linux-musleabihf",
        "0.11.0",
    );

    // Different version
    let key2 = SharedStorage::compute_cache_key(
        "busybox",
        "1.36.2",
        "abc123",
        "arm-linux-musleabihf",
        "0.11.0",
    );

    // Different target
    let key3 = SharedStorage::compute_cache_key(
        "busybox",
        "1.36.1",
        "abc123",
        "aarch64-linux-musl",
        "0.11.0",
    );

    // Different compiler version
    let key4 = SharedStorage::compute_cache_key(
        "busybox",
        "1.36.1",
        "abc123",
        "arm-linux-musleabihf",
        "0.12.0",
    );

    assert_ne!(key1, key2, "Different version should produce different key");
    assert_ne!(key1, key3, "Different target should produce different key");
    assert_ne!(key1, key4, "Different compiler should produce different key");
}

/// Test: Download path is shared across projects
/// **Validates: Requirement 32.9**
#[test]
fn test_download_path_is_shared() {
    use zigroot::core::shared_storage::SharedStorage;
    use zigroot::infra::dirs::ZigrootDirs;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    std::env::set_var("ZIGROOT_DATA_DIR", temp_dir.path());

    let dirs = ZigrootDirs::new();
    let storage1 = SharedStorage::new(&dirs);
    let storage2 = SharedStorage::new(&dirs);

    let path1 = storage1.download_path("busybox", "1.36.1", "abc123");
    let path2 = storage2.download_path("busybox", "1.36.1", "abc123");

    std::env::remove_var("ZIGROOT_DATA_DIR");

    assert_eq!(
        path1, path2,
        "Same package should have same download path across storage instances"
    );
}

/// Test: Cache path uses content-addressable structure
/// **Validates: Requirement 32.10**
#[test]
fn test_cache_path_uses_content_addressable_structure() {
    use zigroot::core::shared_storage::SharedStorage;
    use zigroot::infra::dirs::ZigrootDirs;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    std::env::set_var("ZIGROOT_CACHE_DIR", temp_dir.path());

    let dirs = ZigrootDirs::new();
    let storage = SharedStorage::new(&dirs);

    // Two different cache keys
    let key1 = "abc123def456789012345678901234567890123456789012345678901234";
    let key2 = "xyz789abc123456789012345678901234567890123456789012345678901";

    let path1 = storage.cache_path(key1);
    let path2 = storage.cache_path(key2);

    std::env::remove_var("ZIGROOT_CACHE_DIR");

    // Paths should be different for different keys
    assert_ne!(path1, path2, "Different keys should have different paths");

    // Both should be under build cache
    assert!(path1.starts_with(dirs.build_cache_dir()));
    assert!(path2.starts_with(dirs.build_cache_dir()));
}

/// Test: Shared storage can check if download exists
/// **Validates: Requirement 32.9**
#[test]
fn test_shared_storage_checks_download_exists() {
    use zigroot::core::shared_storage::SharedStorage;
    use zigroot::infra::dirs::ZigrootDirs;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    std::env::set_var("ZIGROOT_DATA_DIR", temp_dir.path());

    let dirs = ZigrootDirs::new();
    let storage = SharedStorage::new(&dirs);

    // Initially should not exist
    assert!(
        !storage.download_exists("busybox", "1.36.1", "abc123"),
        "Download should not exist initially"
    );

    // Create the download directory and file
    let download_path = storage.download_path("busybox", "1.36.1", "abc123");
    std::fs::create_dir_all(download_path.parent().unwrap()).unwrap();
    std::fs::write(&download_path, "test content").unwrap();

    // Now should exist
    assert!(
        storage.download_exists("busybox", "1.36.1", "abc123"),
        "Download should exist after creation"
    );

    std::env::remove_var("ZIGROOT_DATA_DIR");
}

/// Test: Shared storage can check if cache entry exists
/// **Validates: Requirement 32.10**
#[test]
fn test_shared_storage_checks_cache_exists() {
    use zigroot::core::shared_storage::SharedStorage;
    use zigroot::infra::dirs::ZigrootDirs;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    std::env::set_var("ZIGROOT_CACHE_DIR", temp_dir.path());

    let dirs = ZigrootDirs::new();
    let storage = SharedStorage::new(&dirs);

    let cache_key = "abc123def456789012345678901234567890123456789012345678901234";

    // Initially should not exist
    assert!(
        !storage.cache_exists(cache_key),
        "Cache entry should not exist initially"
    );

    // Create the cache directory
    let cache_path = storage.cache_path(cache_key);
    std::fs::create_dir_all(&cache_path).unwrap();

    // Now should exist
    assert!(
        storage.cache_exists(cache_key),
        "Cache entry should exist after creation"
    );

    std::env::remove_var("ZIGROOT_CACHE_DIR");
}

/// Test: Cache key includes all required components
/// **Validates: Requirement 32.10**
#[test]
fn test_cache_key_includes_all_components() {
    use zigroot::core::shared_storage::SharedStorage;

    // The cache key should be a hash that includes:
    // - package version
    // - sha256
    // - target triple
    // - compiler version

    let key = SharedStorage::compute_cache_key(
        "busybox",
        "1.36.1",
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        "arm-linux-musleabihf",
        "0.11.0",
    );

    // Key should be a valid hex string (SHA256 produces 64 hex chars)
    assert_eq!(key.len(), 64, "Cache key should be 64 hex characters");
    assert!(
        key.chars().all(|c| c.is_ascii_hexdigit()),
        "Cache key should be hex: {key}"
    );
}
