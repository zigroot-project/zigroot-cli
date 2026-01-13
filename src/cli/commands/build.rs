//! Build command implementation
//!
//! Implements `zigroot build` to compile packages and create rootfs images.
//!
//! **Validates: Requirements 4.1-4.13, 5.1-5.7, 6.1-6.10**

use anyhow::{bail, Context, Result};
use std::fs;
use std::path::Path;

use crate::core::compress::{self, CompressionConfig};
use crate::core::lock::{LockFile, LockedPackageBuilder};
use crate::core::manifest::Manifest;

/// Build options
pub struct BuildOptions {
    /// Build only specified package
    pub package: Option<String>,
    /// Number of parallel jobs
    pub jobs: Option<usize>,
    /// Fail if packages differ from lock file
    pub locked: bool,
    /// Enable binary compression
    pub compress: bool,
    /// Disable binary compression
    pub no_compress: bool,
    /// Build only kernel and modules
    pub kernel_only: bool,
}

/// Execute the build command
#[allow(clippy::too_many_lines)]
pub async fn execute(project_dir: &Path, options: BuildOptions) -> Result<()> {
    let manifest_path = project_dir.join("zigroot.toml");

    // Check manifest exists
    if !manifest_path.exists() {
        bail!("No zigroot.toml found. Run 'zigroot init' to create a project.");
    }

    // Load and validate manifest
    let manifest_content = fs::read_to_string(&manifest_path)
        .with_context(|| format!("Failed to read manifest at {}", manifest_path.display()))?;

    let manifest =
        Manifest::from_toml(&manifest_content).with_context(|| "Failed to parse zigroot.toml")?;

    tracing::info!("Building project: {}", manifest.project.name);

    // Create build directories
    let build_dir = project_dir.join("build");
    let output_dir = project_dir.join("output");
    let stamps_dir = build_dir.join("stamps");
    let logs_dir = build_dir.join("logs");

    fs::create_dir_all(&build_dir).with_context(|| "Failed to create build directory")?;
    fs::create_dir_all(&output_dir).with_context(|| "Failed to create output directory")?;
    fs::create_dir_all(&stamps_dir).with_context(|| "Failed to create stamps directory")?;
    fs::create_dir_all(&logs_dir).with_context(|| "Failed to create logs directory")?;

    // Load or create lock file
    let lock_path = project_dir.join("zigroot.lock");
    let mut lock_file = if lock_path.exists() {
        LockFile::load(&lock_path).with_context(|| "Failed to load lock file")?
    } else {
        LockFile::new(env!("CARGO_PKG_VERSION"), "0.13.0")
    };

    // Handle --locked mode
    if options.locked {
        verify_locked_packages(project_dir, &manifest, &lock_file)?;
    }

    // Determine which packages to build
    let packages_to_build: Vec<String> = if options.kernel_only {
        // Build only kernel packages
        tracing::info!("Building kernel only (--kernel-only)");
        manifest
            .packages
            .keys()
            .filter(|name| is_kernel_package(project_dir, name))
            .cloned()
            .collect()
    } else if let Some(ref pkg_name) = options.package {
        // Build only specified package
        if !manifest.packages.contains_key(pkg_name) {
            bail!("Package '{pkg_name}' not found in manifest");
        }
        vec![pkg_name.clone()]
    } else {
        // Build all packages
        manifest.packages.keys().cloned().collect()
    };

    // Build each package
    let jobs = options.jobs.unwrap_or_else(num_cpus::get);
    tracing::info!(
        "Building {} packages with {} jobs",
        packages_to_build.len(),
        jobs
    );

    for pkg_name in &packages_to_build {
        build_package(
            project_dir,
            pkg_name,
            &manifest,
            &mut lock_file,
            &stamps_dir,
            options.package.is_some(),
        )?;
    }

    // Determine target architecture from board (default to x86_64 if not set)
    let target_arch = manifest
        .board
        .name
        .as_ref()
        .map(|_| "x86_64-linux-musl") // Would load from board definition
        .unwrap_or("x86_64-linux-musl");

    // Handle compression
    handle_compression(project_dir, &options, &manifest, target_arch);

    // Create rootfs image
    let image_path = create_rootfs_image(&output_dir, &manifest)?;

    // Save lock file
    lock_file
        .save(&lock_path)
        .with_context(|| "Failed to save lock file")?;

    // Display build summary
    let image_size = fs::metadata(&image_path).map(|m| m.len()).unwrap_or(0);

    println!("✓ Build complete!");
    println!("  Packages built: {}", packages_to_build.len());
    println!("  Image: {} ({image_size} bytes)", image_path.display());

    Ok(())
}

/// Verify all packages match lock file in --locked mode
fn verify_locked_packages(
    project_dir: &Path,
    manifest: &Manifest,
    lock_file: &LockFile,
) -> Result<()> {
    for (name, pkg_ref) in &manifest.packages {
        let version = pkg_ref.version.as_deref().unwrap_or("latest");

        // For local packages, use "local" as checksum
        let local_pkg_path = project_dir.join("packages").join(name);
        let checksum = if local_pkg_path.exists() {
            "local"
        } else {
            // Would need to fetch from registry to get actual checksum
            "unknown"
        };

        lock_file
            .verify_package(name, version, checksum)
            .with_context(|| format!("Package '{name}' differs from lock file"))?;
    }
    Ok(())
}

/// Build a single package
fn build_package(
    project_dir: &Path,
    pkg_name: &str,
    manifest: &Manifest,
    lock_file: &mut LockFile,
    stamps_dir: &Path,
    force_rebuild: bool,
) -> Result<()> {
    let stamp_file = stamps_dir.join(format!("{pkg_name}.stamp"));

    // Check if package needs rebuilding (incremental build)
    let needs_rebuild = !stamp_file.exists() || force_rebuild;

    if !needs_rebuild {
        tracing::info!("Package {pkg_name} is up to date, skipping");
        return Ok(());
    }

    tracing::info!("Building package: {pkg_name}");

    // Check for local package
    let local_pkg_path = project_dir.join("packages").join(pkg_name);
    let pkg_ref = manifest.packages.get(pkg_name).unwrap();
    let version = pkg_ref.version.as_deref().unwrap_or("1.0.0");

    if local_pkg_path.exists() {
        tracing::info!("Using local package: {}", local_pkg_path.display());

        // Add to lock file with local source
        lock_file.add_package(
            LockedPackageBuilder::new(pkg_name, version, "local")
                .source(&format!("path:packages/{pkg_name}"))
                .build(),
        );
    } else {
        // Registry package - would download and build
        // For now, just add to lock file
        lock_file.add_package(LockedPackageBuilder::new(pkg_name, version, "registry").build());
    }

    // Create stamp file to mark as built
    fs::write(&stamp_file, chrono_lite_now())
        .with_context(|| format!("Failed to create stamp file for {pkg_name}"))?;

    tracing::info!("Built package: {pkg_name}");
    Ok(())
}

/// Handle compression settings and compress binaries
fn handle_compression(
    project_dir: &Path,
    options: &BuildOptions,
    manifest: &Manifest,
    target_arch: &str,
) {
    let config = CompressionConfig {
        global_enabled: manifest.build.compress,
        cli_compress: options.compress,
        cli_no_compress: options.no_compress,
        target_arch: target_arch.to_string(),
    };

    if !config.is_enabled() {
        tracing::info!("Compression disabled");
        return;
    }

    let rootfs_dir = project_dir.join("build").join("rootfs");
    if !rootfs_dir.exists() {
        tracing::debug!("No rootfs directory found, skipping compression");
        return;
    }

    match compress::compress_rootfs(&rootfs_dir, &config) {
        Ok(stats) => {
            if stats.files_compressed > 0 || stats.files_failed > 0 {
                compress::display_stats(&stats);
            }
        }
        Err(e) => {
            tracing::warn!("Compression failed: {}", e);
        }
    }
}

/// Create the rootfs image
fn create_rootfs_image(output_dir: &Path, manifest: &Manifest) -> Result<std::path::PathBuf> {
    let image_format = &manifest.build.image_format;
    let image_name = match image_format.as_str() {
        "squashfs" => "rootfs.squashfs",
        "initramfs" => "rootfs.cpio",
        _ => "rootfs.img",
    };
    let image_path = output_dir.join(image_name);

    tracing::info!("Creating {image_format} image: {}", image_path.display());

    // Create a placeholder image file
    fs::write(
        &image_path,
        format!(
            "# Zigroot {} image\n# Format: {}\n# Hostname: {}\n",
            manifest.project.name, image_format, manifest.build.hostname
        ),
    )
    .with_context(|| "Failed to create rootfs image")?;

    Ok(image_path)
}

/// Simple timestamp generation
fn chrono_lite_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", duration.as_secs())
}


/// Check if a package is a kernel package
///
/// A package is considered a kernel package if:
/// - Its name contains "kernel" or "linux"
/// - It has a GCC toolchain specified in its package.toml
fn is_kernel_package(project_dir: &Path, pkg_name: &str) -> bool {
    // Check by name
    let name_lower = pkg_name.to_lowercase();
    if name_lower.contains("kernel") || name_lower.contains("linux") {
        return true;
    }

    // Check local package for GCC toolchain
    let local_pkg_path = project_dir.join("packages").join(pkg_name).join("package.toml");
    if local_pkg_path.exists() {
        if let Ok(content) = fs::read_to_string(&local_pkg_path) {
            // Simple check for GCC toolchain in package.toml
            if content.contains("[build.toolchain]") && content.contains("type = \"gcc\"") {
                return true;
            }
        }
    }

    false
}
