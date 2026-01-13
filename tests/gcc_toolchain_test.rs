//! Integration tests for GCC toolchain support
//!
//! Tests for Requirements 26.2-26.7:
//! - Auto-resolves bootlin.com URLs from target
//! - Supports explicit URLs per host platform
//! - Downloads and caches toolchains
//!
//! **Validates: Requirements 26.2-26.7**

#[allow(dead_code)]
mod common;

// ============================================
// Unit Tests for GCC Toolchain URL Resolution
// ============================================

/// Test: Auto-resolves bootlin.com URLs from target
/// **Validates: Requirement 26.3**
#[test]
fn test_gcc_toolchain_auto_resolves_bootlin_url_arm() {
    use zigroot::infra::gcc_toolchain::{resolve_bootlin_url, HostPlatform};

    let host = HostPlatform::LinuxX86_64;
    let target = "arm-linux-gnueabihf";

    let result = resolve_bootlin_url(&host, target, None, None);

    assert!(result.is_ok(), "Should resolve bootlin URL for ARM target");
    let url = result.unwrap();
    assert!(
        url.contains("bootlin.com"),
        "URL should be from bootlin.com: {url}"
    );
    assert!(
        url.contains("armv7-eabihf") || url.contains("arm"),
        "URL should contain ARM architecture: {url}"
    );
    assert!(url.contains("glibc"), "URL should use default glibc: {url}");
}

/// Test: Auto-resolves bootlin.com URLs for aarch64 target
/// **Validates: Requirement 26.4**
#[test]
fn test_gcc_toolchain_auto_resolves_bootlin_url_aarch64() {
    use zigroot::infra::gcc_toolchain::{resolve_bootlin_url, HostPlatform};

    let host = HostPlatform::LinuxX86_64;
    let target = "aarch64-linux-gnu";

    let result = resolve_bootlin_url(&host, target, None, None);

    assert!(
        result.is_ok(),
        "Should resolve bootlin URL for aarch64 target"
    );
    let url = result.unwrap();
    assert!(
        url.contains("bootlin.com"),
        "URL should be from bootlin.com: {url}"
    );
    assert!(
        url.contains("aarch64"),
        "URL should contain aarch64 architecture: {url}"
    );
}

/// Test: Auto-resolves bootlin.com URLs for x86_64 target
/// **Validates: Requirement 26.4**
#[test]
fn test_gcc_toolchain_auto_resolves_bootlin_url_x86_64() {
    use zigroot::infra::gcc_toolchain::{resolve_bootlin_url, HostPlatform};

    let host = HostPlatform::LinuxX86_64;
    let target = "x86_64-linux-gnu";

    let result = resolve_bootlin_url(&host, target, None, None);

    assert!(
        result.is_ok(),
        "Should resolve bootlin URL for x86_64 target"
    );
    let url = result.unwrap();
    assert!(
        url.contains("bootlin.com"),
        "URL should be from bootlin.com: {url}"
    );
    assert!(
        url.contains("x86-64") || url.contains("x86_64"),
        "URL should contain x86-64 architecture: {url}"
    );
}

/// Test: Auto-resolves bootlin.com URLs for riscv64 target
/// **Validates: Requirement 26.4**
#[test]
fn test_gcc_toolchain_auto_resolves_bootlin_url_riscv64() {
    use zigroot::infra::gcc_toolchain::{resolve_bootlin_url, HostPlatform};

    let host = HostPlatform::LinuxX86_64;
    let target = "riscv64-linux-gnu";

    let result = resolve_bootlin_url(&host, target, None, None);

    assert!(
        result.is_ok(),
        "Should resolve bootlin URL for riscv64 target"
    );
    let url = result.unwrap();
    assert!(
        url.contains("bootlin.com"),
        "URL should be from bootlin.com: {url}"
    );
    assert!(
        url.contains("riscv64"),
        "URL should contain riscv64 architecture: {url}"
    );
}

/// Test: Custom libc and release can be specified
/// **Validates: Requirement 26.5**
#[test]
fn test_gcc_toolchain_custom_libc_and_release() {
    use zigroot::infra::gcc_toolchain::{resolve_bootlin_url, HostPlatform};

    let host = HostPlatform::LinuxX86_64;
    let target = "arm-linux-gnueabihf";
    let libc = Some("musl");
    let release = Some("stable-2023.11-1");

    let result = resolve_bootlin_url(&host, target, libc, release);

    assert!(
        result.is_ok(),
        "Should resolve bootlin URL with custom libc/release"
    );
    let url = result.unwrap();
    assert!(
        url.contains("musl"),
        "URL should contain custom libc: {url}"
    );
    assert!(
        url.contains("stable-2023.11-1"),
        "URL should contain custom release: {url}"
    );
}

/// Test: Bootlin not available for macOS host
/// **Validates: Requirement 26.8**
#[test]
fn test_gcc_toolchain_bootlin_not_available_for_macos() {
    use zigroot::infra::gcc_toolchain::{resolve_bootlin_url, HostPlatform};

    let host = HostPlatform::DarwinX86_64;
    let target = "arm-linux-gnueabihf";

    let result = resolve_bootlin_url(&host, target, None, None);

    assert!(
        result.is_err(),
        "Should fail for macOS host (bootlin only provides Linux toolchains)"
    );
    let err = result.unwrap_err();
    let err_msg = err.to_string();
    assert!(
        err_msg.contains("not available")
            || err_msg.contains("Docker")
            || err_msg.contains("explicit"),
        "Error should suggest alternatives: {err_msg}"
    );
}

/// Test: Unsupported target returns error
/// **Validates: Requirement 26.4**
#[test]
fn test_gcc_toolchain_unsupported_target() {
    use zigroot::infra::gcc_toolchain::{resolve_bootlin_url, HostPlatform};

    let host = HostPlatform::LinuxX86_64;
    let target = "unsupported-unknown-target";

    let result = resolve_bootlin_url(&host, target, None, None);

    assert!(result.is_err(), "Should fail for unsupported target");
}

// ============================================
// Tests for Explicit URL Support
// ============================================

/// Test: Supports explicit URLs per host platform
/// **Validates: Requirement 26.6**
#[test]
fn test_gcc_toolchain_explicit_urls() {
    use std::collections::HashMap;
    use zigroot::infra::gcc_toolchain::{GccToolchainSpec, HostPlatform};

    let mut urls = HashMap::new();
    urls.insert(
        HostPlatform::LinuxX86_64,
        "https://example.com/toolchain-linux-x86_64.tar.gz".to_string(),
    );
    urls.insert(
        HostPlatform::DarwinAarch64,
        "https://example.com/toolchain-darwin-aarch64.tar.gz".to_string(),
    );

    let spec = GccToolchainSpec::Explicit { urls };

    // Get URL for Linux x86_64
    let linux_url = spec.get_url_for_host(&HostPlatform::LinuxX86_64);
    assert!(linux_url.is_some(), "Should have URL for Linux x86_64");
    assert_eq!(
        linux_url.unwrap(),
        "https://example.com/toolchain-linux-x86_64.tar.gz"
    );

    // Get URL for Darwin aarch64
    let darwin_url = spec.get_url_for_host(&HostPlatform::DarwinAarch64);
    assert!(darwin_url.is_some(), "Should have URL for Darwin aarch64");
    assert_eq!(
        darwin_url.unwrap(),
        "https://example.com/toolchain-darwin-aarch64.tar.gz"
    );

    // Missing platform should return None
    let missing_url = spec.get_url_for_host(&HostPlatform::LinuxAarch64);
    assert!(
        missing_url.is_none(),
        "Should return None for missing platform"
    );
}

// ============================================
// Tests for Toolchain Caching
// ============================================

/// Test: Downloads and caches toolchains
/// **Validates: Requirement 26.7**
#[test]
fn test_gcc_toolchain_cache_structure() {
    use std::path::PathBuf;
    use zigroot::infra::gcc_toolchain::GccToolchainCache;

    let cache_dir = PathBuf::from("/tmp/test-toolchain-cache");
    let cache = GccToolchainCache::new(cache_dir.clone());

    // Verify cache directory is set correctly
    assert_eq!(cache.cache_dir(), &cache_dir);

    // Verify cache key generation is deterministic
    let url = "https://toolchains.bootlin.com/downloads/releases/toolchains/armv7-eabihf/tarballs/armv7-eabihf--glibc--stable-2024.02-1.tar.bz2";
    let key1 = cache.compute_cache_key(url);
    let key2 = cache.compute_cache_key(url);
    assert_eq!(key1, key2, "Cache key should be deterministic");

    // Different URLs should have different keys
    let other_url = "https://example.com/other-toolchain.tar.gz";
    let other_key = cache.compute_cache_key(other_url);
    assert_ne!(
        key1, other_key,
        "Different URLs should have different cache keys"
    );
}

/// Test: Cache key is based on URL hash
/// **Validates: Requirement 26.7**
#[test]
fn test_gcc_toolchain_cache_key_determinism() {
    use std::path::PathBuf;
    use zigroot::infra::gcc_toolchain::GccToolchainCache;

    let cache = GccToolchainCache::new(PathBuf::from("/tmp/cache"));

    let url = "https://toolchains.bootlin.com/test.tar.bz2";

    // Multiple calls should return the same key
    let keys: Vec<_> = (0..10).map(|_| cache.compute_cache_key(url)).collect();
    assert!(
        keys.iter().all(|k| k == &keys[0]),
        "Cache key should be deterministic across multiple calls"
    );
}

// ============================================
// Tests for Host Platform Detection
// ============================================

/// Test: Host platform detection
/// **Validates: Requirement 26.3**
#[test]
fn test_detect_host_platform() {
    use zigroot::infra::gcc_toolchain::{detect_host_platform, HostPlatform};

    let host = detect_host_platform();

    // Should return a valid platform
    match host {
        HostPlatform::LinuxX86_64
        | HostPlatform::LinuxAarch64
        | HostPlatform::DarwinX86_64
        | HostPlatform::DarwinAarch64 => {
            // Valid platform detected
        }
        HostPlatform::Unknown(ref s) => {
            // Unknown is acceptable for unsupported platforms
            assert!(!s.is_empty(), "Unknown platform should have a description");
        }
    }
}

/// Test: Host platform string representation
#[test]
fn test_host_platform_to_string() {
    use zigroot::infra::gcc_toolchain::HostPlatform;

    assert_eq!(HostPlatform::LinuxX86_64.to_string(), "linux-x86_64");
    assert_eq!(HostPlatform::LinuxAarch64.to_string(), "linux-aarch64");
    assert_eq!(HostPlatform::DarwinX86_64.to_string(), "darwin-x86_64");
    assert_eq!(HostPlatform::DarwinAarch64.to_string(), "darwin-aarch64");
}

// ============================================
// Tests for GCC Toolchain Instance
// ============================================

/// Test: GCC toolchain prefix generation
#[test]
fn test_gcc_toolchain_prefix() {
    use std::path::PathBuf;
    use zigroot::infra::gcc_toolchain::GccToolchain;

    let toolchain = GccToolchain::new(
        PathBuf::from("/opt/toolchains/arm-gcc"),
        "arm-linux-gnueabihf".to_string(),
    );

    assert_eq!(toolchain.prefix(), "arm-linux-gnueabihf-");
    assert_eq!(toolchain.cc(), "arm-linux-gnueabihf-gcc");
    assert_eq!(toolchain.cxx(), "arm-linux-gnueabihf-g++");
    assert_eq!(toolchain.ar(), "arm-linux-gnueabihf-ar");
    assert_eq!(toolchain.ld(), "arm-linux-gnueabihf-ld");
}

/// Test: GCC toolchain path
#[test]
fn test_gcc_toolchain_path() {
    use std::path::PathBuf;
    use zigroot::infra::gcc_toolchain::GccToolchain;

    let path = PathBuf::from("/opt/toolchains/aarch64-gcc");
    let toolchain = GccToolchain::new(path.clone(), "aarch64-linux-gnu".to_string());

    assert_eq!(toolchain.path(), &path);
    assert_eq!(toolchain.bin_dir(), path.join("bin"));
}
