//! Binary compression using UPX
//!
//! This module handles compressing ELF binaries using UPX to reduce
//! rootfs image size.
//!
//! **Validates: Requirements 6.1-6.10**

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Architectures supported by UPX
const UPX_SUPPORTED_ARCHS: &[&str] = &["x86_64", "x86", "i386", "i686", "arm", "aarch64", "arm64"];

/// ELF magic bytes
const ELF_MAGIC: &[u8] = &[0x7f, b'E', b'L', b'F'];

/// Compression statistics
#[derive(Debug, Default, Clone)]
pub struct CompressionStats {
    /// Number of files compressed
    pub files_compressed: usize,
    /// Number of files skipped
    pub files_skipped: usize,
    /// Number of files that failed compression
    pub files_failed: usize,
    /// Total original size in bytes
    pub original_size: u64,
    /// Total compressed size in bytes
    pub compressed_size: u64,
}

impl CompressionStats {
    /// Calculate compression ratio as a percentage
    pub fn ratio(&self) -> f64 {
        if self.original_size == 0 {
            0.0
        } else {
            (1.0 - (self.compressed_size as f64 / self.original_size as f64)) * 100.0
        }
    }

    /// Calculate bytes saved
    pub fn bytes_saved(&self) -> u64 {
        self.original_size.saturating_sub(self.compressed_size)
    }
}

/// Compression configuration
#[derive(Debug, Clone)]
pub struct CompressionConfig {
    /// Global compression setting from manifest
    pub global_enabled: bool,
    /// CLI --compress flag
    pub cli_compress: bool,
    /// CLI --no-compress flag
    pub cli_no_compress: bool,
    /// Target architecture
    pub target_arch: String,
}

impl CompressionConfig {
    /// Determine if compression is enabled based on priority
    pub fn is_enabled(&self) -> bool {
        if self.cli_no_compress {
            return false;
        }
        if self.cli_compress {
            return true;
        }
        self.global_enabled
    }

    /// Check if compression should be enabled for a specific package
    pub fn is_enabled_for_package(&self, package_compress: Option<bool>) -> bool {
        // CLI flags always take precedence
        if self.cli_no_compress {
            return false;
        }
        if self.cli_compress {
            return true;
        }
        // Package setting overrides global
        package_compress.unwrap_or(self.global_enabled)
    }

    /// Check if the target architecture is supported by UPX
    pub fn is_arch_supported(&self) -> bool {
        let arch = self.target_arch.to_lowercase();
        UPX_SUPPORTED_ARCHS
            .iter()
            .any(|&supported| arch.contains(supported) || supported.contains(&arch))
    }
}

/// Check if UPX is installed on the system
pub fn is_upx_available() -> bool {
    which::which("upx").is_ok()
}

/// Check if a file is an ELF binary
pub fn is_elf_binary(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }

    // Read first 4 bytes to check ELF magic
    match std::fs::read(path) {
        Ok(data) if data.len() >= 4 => data.starts_with(ELF_MAGIC),
        _ => false,
    }
}

/// Find all ELF binaries in a directory recursively
pub fn find_elf_binaries(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut binaries = Vec::new();

    if !dir.exists() {
        return Ok(binaries);
    }

    for entry in walkdir::WalkDir::new(dir)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if is_elf_binary(path) {
            binaries.push(path.to_path_buf());
        }
    }

    Ok(binaries)
}

/// Compress a single binary using UPX
pub fn compress_binary(path: &Path) -> Result<(u64, u64)> {
    let original_size = std::fs::metadata(path)
        .with_context(|| format!("Failed to get size of {}", path.display()))?
        .len();

    // Run UPX with best compression
    let output = Command::new("upx")
        .args(["--best", "--quiet"])
        .arg(path)
        .output()
        .with_context(|| format!("Failed to run UPX on {}", path.display()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("UPX failed for {}: {}", path.display(), stderr);
    }

    let compressed_size = std::fs::metadata(path)
        .with_context(|| format!("Failed to get compressed size of {}", path.display()))?
        .len();

    Ok((original_size, compressed_size))
}

/// Compress all binaries in a rootfs directory
pub fn compress_rootfs(rootfs_dir: &Path, config: &CompressionConfig) -> Result<CompressionStats> {
    let mut stats = CompressionStats::default();

    // Check if compression is enabled
    if !config.is_enabled() {
        tracing::info!("Compression disabled");
        return Ok(stats);
    }

    // Check if architecture is supported
    if !config.is_arch_supported() {
        tracing::warn!(
            "Architecture '{}' is not supported by UPX, skipping compression",
            config.target_arch
        );
        return Ok(stats);
    }

    // Check if UPX is available
    if !is_upx_available() {
        tracing::warn!(
            "UPX not found, skipping compression. Install UPX to enable binary compression."
        );
        return Ok(stats);
    }

    tracing::info!("Compressing binaries in {}", rootfs_dir.display());

    // Find all ELF binaries
    let binaries = find_elf_binaries(rootfs_dir)?;

    if binaries.is_empty() {
        tracing::info!("No ELF binaries found to compress");
        return Ok(stats);
    }

    tracing::info!("Found {} ELF binaries to compress", binaries.len());

    // Compress each binary
    for binary in &binaries {
        match compress_binary(binary) {
            Ok((original, compressed)) => {
                stats.files_compressed += 1;
                stats.original_size += original;
                stats.compressed_size += compressed;

                let ratio = if original > 0 {
                    (1.0 - (compressed as f64 / original as f64)) * 100.0
                } else {
                    0.0
                };

                tracing::debug!(
                    "Compressed {}: {} -> {} ({:.1}% reduction)",
                    binary.display(),
                    original,
                    compressed,
                    ratio
                );
            }
            Err(e) => {
                // Log warning but continue with uncompressed binary
                tracing::warn!("Failed to compress {}: {}", binary.display(), e);
                stats.files_failed += 1;

                // Add original size to stats (file remains uncompressed)
                if let Ok(meta) = std::fs::metadata(binary) {
                    let size = meta.len();
                    stats.original_size += size;
                    stats.compressed_size += size;
                }
            }
        }
    }

    Ok(stats)
}

/// Display compression statistics
pub fn display_stats(stats: &CompressionStats) {
    if stats.files_compressed == 0 && stats.files_failed == 0 {
        return;
    }

    println!("Compression Statistics:");
    println!("  Files compressed: {}", stats.files_compressed);

    if stats.files_failed > 0 {
        println!("  Files failed: {}", stats.files_failed);
    }

    if stats.files_skipped > 0 {
        println!("  Files skipped: {}", stats.files_skipped);
    }

    if stats.original_size > 0 {
        let original_kb = stats.original_size as f64 / 1024.0;
        let compressed_kb = stats.compressed_size as f64 / 1024.0;
        let saved_kb = stats.bytes_saved() as f64 / 1024.0;

        println!("  Original size: {:.1} KB", original_kb);
        println!("  Compressed size: {:.1} KB", compressed_kb);
        println!("  Space saved: {:.1} KB ({:.1}%)", saved_kb, stats.ratio());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use tempfile::TempDir;

    #[test]
    fn test_compression_config_cli_no_compress_overrides_all() {
        let config = CompressionConfig {
            global_enabled: true,
            cli_compress: true,
            cli_no_compress: true,
            target_arch: "x86_64".to_string(),
        };
        assert!(!config.is_enabled());
    }

    #[test]
    fn test_compression_config_cli_compress_overrides_global() {
        let config = CompressionConfig {
            global_enabled: false,
            cli_compress: true,
            cli_no_compress: false,
            target_arch: "x86_64".to_string(),
        };
        assert!(config.is_enabled());
    }

    #[test]
    fn test_compression_config_global_fallback() {
        let config = CompressionConfig {
            global_enabled: true,
            cli_compress: false,
            cli_no_compress: false,
            target_arch: "x86_64".to_string(),
        };
        assert!(config.is_enabled());
    }

    #[test]
    fn test_compression_config_package_overrides_global() {
        let config = CompressionConfig {
            global_enabled: false,
            cli_compress: false,
            cli_no_compress: false,
            target_arch: "x86_64".to_string(),
        };
        assert!(config.is_enabled_for_package(Some(true)));
        assert!(!config.is_enabled_for_package(Some(false)));
        assert!(!config.is_enabled_for_package(None));
    }

    #[test]
    fn test_arch_supported() {
        let supported = [
            "x86_64-linux-musl",
            "arm-linux-musleabihf",
            "aarch64-linux-musl",
        ];
        for arch in supported {
            let config = CompressionConfig {
                global_enabled: true,
                cli_compress: false,
                cli_no_compress: false,
                target_arch: arch.to_string(),
            };
            assert!(config.is_arch_supported(), "Should support {arch}");
        }
    }

    #[test]
    fn test_arch_not_supported() {
        let unsupported = [
            "riscv64-linux-musl",
            "mips-linux-musl",
            "powerpc-linux-musl",
        ];
        for arch in unsupported {
            let config = CompressionConfig {
                global_enabled: true,
                cli_compress: false,
                cli_no_compress: false,
                target_arch: arch.to_string(),
            };
            assert!(!config.is_arch_supported(), "Should not support {arch}");
        }
    }

    #[test]
    fn test_is_elf_binary_false_for_text() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "hello world").unwrap();
        assert!(!is_elf_binary(&file));
    }

    #[test]
    fn test_is_elf_binary_false_for_nonexistent() {
        assert!(!is_elf_binary(Path::new("/nonexistent/file")));
    }

    #[test]
    fn test_compression_stats_ratio() {
        let stats = CompressionStats {
            files_compressed: 1,
            files_skipped: 0,
            files_failed: 0,
            original_size: 1000,
            compressed_size: 400,
        };
        assert!((stats.ratio() - 60.0).abs() < 0.1);
    }

    #[test]
    fn test_compression_stats_bytes_saved() {
        let stats = CompressionStats {
            files_compressed: 1,
            files_skipped: 0,
            files_failed: 0,
            original_size: 1000,
            compressed_size: 400,
        };
        assert_eq!(stats.bytes_saved(), 600);
    }

    // Property-based tests
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 9: Compression Toggle Consistency
        /// CLI flags always take precedence over other settings
        #[test]
        fn prop_cli_no_compress_always_disables(
            global in prop::bool::ANY,
            cli_compress in prop::bool::ANY
        ) {
            let config = CompressionConfig {
                global_enabled: global,
                cli_compress,
                cli_no_compress: true,
                target_arch: "x86_64".to_string(),
            };
            prop_assert!(!config.is_enabled(), "cli_no_compress should always disable");
        }

        #[test]
        fn prop_cli_compress_enables_when_no_no_compress(
            global in prop::bool::ANY
        ) {
            let config = CompressionConfig {
                global_enabled: global,
                cli_compress: true,
                cli_no_compress: false,
                target_arch: "x86_64".to_string(),
            };
            prop_assert!(config.is_enabled(), "cli_compress should enable when no cli_no_compress");
        }

        #[test]
        fn prop_package_overrides_global(
            global in prop::bool::ANY,
            package in prop::bool::ANY
        ) {
            let config = CompressionConfig {
                global_enabled: global,
                cli_compress: false,
                cli_no_compress: false,
                target_arch: "x86_64".to_string(),
            };
            prop_assert_eq!(
                config.is_enabled_for_package(Some(package)),
                package,
                "Package setting should override global"
            );
        }

        #[test]
        fn prop_compression_ratio_valid(
            original in 1u64..1_000_000,
            compressed in 1u64..1_000_000
        ) {
            let stats = CompressionStats {
                files_compressed: 1,
                files_skipped: 0,
                files_failed: 0,
                original_size: original,
                compressed_size: compressed.min(original),
            };
            let ratio = stats.ratio();
            prop_assert!(ratio >= 0.0 && ratio <= 100.0, "Ratio should be 0-100%");
        }
    }
}
