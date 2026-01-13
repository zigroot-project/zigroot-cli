//! GCC Toolchain management
//!
//! Handles GCC toolchain resolution, download, and caching for packages
//! that cannot be built with Zig (e.g., Linux kernel, bootloaders).
//!
//! # Overview
//!
//! Most packages in zigroot are built using Zig's built-in cross-compilation.
//! However, some packages (like the Linux kernel) require a traditional GCC
//! cross-toolchain. This module provides:
//!
//! - Automatic resolution of bootlin.com toolchain URLs
//! - Support for explicit per-platform URLs
//! - Toolchain caching for reuse across builds
//!
//! # Requirements
//!
//! - **26.2**: Packages can specify `[build.toolchain]` with `type = "gcc"`
//! - **26.3**: Auto-resolves bootlin.com URLs from target
//! - **26.4**: Supports common target triples
//! - **26.5**: Custom libc and release can be specified
//! - **26.6**: Explicit URLs per host platform supported
//! - **26.7**: Toolchains are cached for reuse

use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use thiserror::Error;

/// Errors related to GCC toolchain operations
#[derive(Error, Debug)]
pub enum GccToolchainError {
    /// Bootlin toolchains not available for this host platform
    #[error("Bootlin toolchains are not available for host platform '{host}'. {suggestion}")]
    BootlinNotAvailable { host: String, suggestion: String },

    /// Unsupported target triple
    #[error("Unsupported target triple '{target}' for GCC toolchain")]
    UnsupportedTarget { target: String },

    /// No URL available for host platform
    #[error("No toolchain URL available for host platform '{host}'. Supported platforms: {supported:?}")]
    NoUrlForHost {
        host: String,
        supported: Vec<String>,
    },

    /// Download error
    #[error("Failed to download toolchain from '{url}': {error}")]
    DownloadError { url: String, error: String },

    /// Extraction error
    #[error("Failed to extract toolchain archive: {error}")]
    ExtractionError { error: String },

    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Host platform identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HostPlatform {
    /// Linux on x86_64
    LinuxX86_64,
    /// Linux on aarch64
    LinuxAarch64,
    /// macOS on x86_64 (Intel)
    DarwinX86_64,
    /// macOS on aarch64 (Apple Silicon)
    DarwinAarch64,
    /// Unknown/unsupported platform
    Unknown(String),
}

impl fmt::Display for HostPlatform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HostPlatform::LinuxX86_64 => write!(f, "linux-x86_64"),
            HostPlatform::LinuxAarch64 => write!(f, "linux-aarch64"),
            HostPlatform::DarwinX86_64 => write!(f, "darwin-x86_64"),
            HostPlatform::DarwinAarch64 => write!(f, "darwin-aarch64"),
            HostPlatform::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Detect the current host platform
pub fn detect_host_platform() -> HostPlatform {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    match (os, arch) {
        ("linux", "x86_64") => HostPlatform::LinuxX86_64,
        ("linux", "aarch64") => HostPlatform::LinuxAarch64,
        ("macos", "x86_64") => HostPlatform::DarwinX86_64,
        ("macos", "aarch64") => HostPlatform::DarwinAarch64,
        _ => HostPlatform::Unknown(format!("{os}-{arch}")),
    }
}

/// GCC toolchain specification
#[derive(Debug, Clone)]
pub enum GccToolchainSpec {
    /// Auto-resolve from bootlin.com based on target
    Auto {
        target: String,
        libc: Option<String>,
        release: Option<String>,
    },
    /// Explicit URLs per host platform
    Explicit {
        urls: HashMap<HostPlatform, String>,
    },
}

impl GccToolchainSpec {
    /// Get the URL for a specific host platform (for Explicit spec)
    pub fn get_url_for_host(&self, host: &HostPlatform) -> Option<&str> {
        match self {
            GccToolchainSpec::Explicit { urls } => urls.get(host).map(|s| s.as_str()),
            GccToolchainSpec::Auto { .. } => None,
        }
    }
}

/// Resolve bootlin.com toolchain URL for the given host and target
///
/// # Arguments
///
/// * `host` - The host platform to download for
/// * `target` - The target triple (e.g., "arm-linux-gnueabihf")
/// * `libc` - Optional libc variant (default: "glibc")
/// * `release` - Optional release version (default: "stable-2024.02-1")
///
/// # Returns
///
/// The URL to download the toolchain from bootlin.com
///
/// # Errors
///
/// Returns an error if:
/// - The host platform is not supported by bootlin (e.g., macOS)
/// - The target triple is not recognized
pub fn resolve_bootlin_url(
    host: &HostPlatform,
    target: &str,
    libc: Option<&str>,
    release: Option<&str>,
) -> Result<String, GccToolchainError> {
    let libc = libc.unwrap_or("glibc");
    let release = release.unwrap_or("stable-2024.02-1");

    // Map target triple to bootlin architecture name
    let bootlin_arch = match target {
        "arm-linux-gnueabihf" => "armv7-eabihf",
        "aarch64-linux-gnu" => "aarch64",
        "x86_64-linux-gnu" => "x86-64",
        "riscv64-linux-gnu" => "riscv64-lp64d",
        other => {
            return Err(GccToolchainError::UnsupportedTarget {
                target: other.to_string(),
            })
        }
    };

    // Bootlin provides Linux-hosted toolchains only
    match host {
        HostPlatform::LinuxX86_64 | HostPlatform::LinuxAarch64 => Ok(format!(
            "https://toolchains.bootlin.com/downloads/releases/toolchains/{bootlin_arch}/tarballs/{bootlin_arch}--{libc}--{release}.tar.bz2"
        )),
        HostPlatform::DarwinX86_64 | HostPlatform::DarwinAarch64 => {
            Err(GccToolchainError::BootlinNotAvailable {
                host: host.to_string(),
                suggestion: "Use [build.toolchain.url] with explicit URLs for this platform, or build in Docker".to_string(),
            })
        }
        HostPlatform::Unknown(s) => Err(GccToolchainError::BootlinNotAvailable {
            host: s.clone(),
            suggestion: "Use [build.toolchain.url] with explicit URLs for this platform".to_string(),
        }),
    }
}

/// A downloaded and extracted GCC toolchain instance
#[derive(Debug, Clone)]
pub struct GccToolchain {
    /// Path to the extracted toolchain directory
    path: PathBuf,
    /// Target triple (e.g., "arm-linux-gnueabihf")
    target: String,
}

impl GccToolchain {
    /// Create a new GCC toolchain instance
    pub fn new(path: PathBuf, target: String) -> Self {
        Self { path, target }
    }

    /// Get the path to the toolchain directory
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Get the bin directory path
    pub fn bin_dir(&self) -> PathBuf {
        self.path.join("bin")
    }

    /// Get the toolchain prefix (e.g., "arm-linux-gnueabihf-")
    pub fn prefix(&self) -> String {
        format!("{}-", self.target)
    }

    /// Get the C compiler command
    pub fn cc(&self) -> String {
        format!("{}gcc", self.prefix())
    }

    /// Get the C++ compiler command
    pub fn cxx(&self) -> String {
        format!("{}g++", self.prefix())
    }

    /// Get the archiver command
    pub fn ar(&self) -> String {
        format!("{}ar", self.prefix())
    }

    /// Get the linker command
    pub fn ld(&self) -> String {
        format!("{}ld", self.prefix())
    }

    /// Get the target triple
    pub fn target(&self) -> &str {
        &self.target
    }
}

/// Cache for downloaded GCC toolchains
///
/// Toolchains are cached by URL hash to avoid re-downloading.
#[derive(Debug)]
pub struct GccToolchainCache {
    /// Directory where toolchains are cached
    cache_dir: PathBuf,
}

impl GccToolchainCache {
    /// Create a new toolchain cache
    pub fn new(cache_dir: PathBuf) -> Self {
        Self { cache_dir }
    }

    /// Get the cache directory
    pub fn cache_dir(&self) -> &PathBuf {
        &self.cache_dir
    }

    /// Compute a cache key for a URL
    ///
    /// Uses SHA256 hash of the URL to create a deterministic key.
    pub fn compute_cache_key(&self, url: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(url.as_bytes());
        let result = hasher.finalize();
        // Use first 16 bytes (32 hex chars) for the key
        hex::encode(&result[..16])
    }

    /// Get the path where a toolchain would be cached
    pub fn get_cache_path(&self, url: &str) -> PathBuf {
        let key = self.compute_cache_key(url);
        self.cache_dir.join(&key)
    }

    /// Get the path to the archive file
    pub fn get_archive_path(&self, url: &str) -> PathBuf {
        let key = self.compute_cache_key(url);
        // Determine extension from URL
        let ext = if url.ends_with(".tar.bz2") {
            "tar.bz2"
        } else if url.ends_with(".tar.gz") {
            "tar.gz"
        } else if url.ends_with(".tar.xz") {
            "tar.xz"
        } else {
            "tar.gz"
        };
        self.cache_dir.join(format!("{key}.{ext}"))
    }

    /// Check if a toolchain is already cached
    pub fn is_cached(&self, url: &str) -> bool {
        self.get_cache_path(url).exists()
    }

    /// Get a cached toolchain, or None if not cached
    pub fn get_cached(&self, url: &str, target: &str) -> Option<GccToolchain> {
        let path = self.get_cache_path(url);
        if path.exists() {
            Some(GccToolchain::new(path, target.to_string()))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_host_platform_display() {
        assert_eq!(HostPlatform::LinuxX86_64.to_string(), "linux-x86_64");
        assert_eq!(HostPlatform::LinuxAarch64.to_string(), "linux-aarch64");
        assert_eq!(HostPlatform::DarwinX86_64.to_string(), "darwin-x86_64");
        assert_eq!(HostPlatform::DarwinAarch64.to_string(), "darwin-aarch64");
        assert_eq!(
            HostPlatform::Unknown("freebsd-x86_64".to_string()).to_string(),
            "freebsd-x86_64"
        );
    }

    #[test]
    fn test_resolve_bootlin_url_arm() {
        let url = resolve_bootlin_url(
            &HostPlatform::LinuxX86_64,
            "arm-linux-gnueabihf",
            None,
            None,
        )
        .unwrap();

        assert!(url.contains("bootlin.com"));
        assert!(url.contains("armv7-eabihf"));
        assert!(url.contains("glibc"));
        assert!(url.contains("stable-2024.02-1"));
    }

    #[test]
    fn test_resolve_bootlin_url_custom_libc() {
        let url = resolve_bootlin_url(
            &HostPlatform::LinuxX86_64,
            "arm-linux-gnueabihf",
            Some("musl"),
            Some("stable-2023.11-1"),
        )
        .unwrap();

        assert!(url.contains("musl"));
        assert!(url.contains("stable-2023.11-1"));
    }

    #[test]
    fn test_resolve_bootlin_url_macos_fails() {
        let result = resolve_bootlin_url(
            &HostPlatform::DarwinX86_64,
            "arm-linux-gnueabihf",
            None,
            None,
        );

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, GccToolchainError::BootlinNotAvailable { .. }));
    }

    #[test]
    fn test_gcc_toolchain_commands() {
        let toolchain = GccToolchain::new(
            PathBuf::from("/opt/toolchain"),
            "arm-linux-gnueabihf".to_string(),
        );

        assert_eq!(toolchain.prefix(), "arm-linux-gnueabihf-");
        assert_eq!(toolchain.cc(), "arm-linux-gnueabihf-gcc");
        assert_eq!(toolchain.cxx(), "arm-linux-gnueabihf-g++");
        assert_eq!(toolchain.ar(), "arm-linux-gnueabihf-ar");
        assert_eq!(toolchain.ld(), "arm-linux-gnueabihf-ld");
    }

    #[test]
    fn test_cache_key_determinism() {
        let cache = GccToolchainCache::new(PathBuf::from("/tmp/cache"));
        let url = "https://example.com/toolchain.tar.gz";

        let key1 = cache.compute_cache_key(url);
        let key2 = cache.compute_cache_key(url);

        assert_eq!(key1, key2);
    }

    #[test]
    fn test_cache_key_uniqueness() {
        let cache = GccToolchainCache::new(PathBuf::from("/tmp/cache"));

        let key1 = cache.compute_cache_key("https://example.com/toolchain1.tar.gz");
        let key2 = cache.compute_cache_key("https://example.com/toolchain2.tar.gz");

        assert_ne!(key1, key2);
    }
}
