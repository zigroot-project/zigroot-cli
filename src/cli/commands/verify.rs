//! Verify command implementation
//!
//! Implements `zigroot verify` for validating package and board definitions.
//!
//! **Validates: Requirements 28.2-28.5, 29.2-29.4**

use anyhow::Result;
use std::path::Path;

/// Known valid Zig target triples
const VALID_ZIG_TARGETS: &[&str] = &[
    "arm-linux-musleabihf",
    "arm-linux-musleabi",
    "aarch64-linux-musl",
    "aarch64-linux-gnu",
    "x86_64-linux-musl",
    "x86_64-linux-gnu",
    "riscv64-linux-musl",
    "riscv64-linux-gnu",
    "i386-linux-musl",
    "i386-linux-gnu",
    "mips-linux-musl",
    "mipsel-linux-musl",
    "powerpc-linux-musl",
    "powerpc64-linux-musl",
];

/// Execute the verify command
///
/// Validates package or board structure, required fields, and TOML syntax.
/// **Validates: Requirements 28.2-28.5, 29.2-29.4**
pub async fn execute(project_dir: &Path, path: &str, fetch: bool) -> Result<()> {
    let full_path = project_dir.join(path);

    // Check if path exists
    if !full_path.exists() {
        anyhow::bail!("Path '{}' does not exist", path);
    }

    // Detect if this is a package or board
    let is_package = full_path.join("metadata.toml").exists()
        || path.contains("packages")
        || full_path.join("1.0.0.toml").exists();
    let is_board = full_path.join("board.toml").exists() || path.contains("boards");

    if is_package {
        verify_package(&full_path, fetch).await?;
    } else if is_board {
        verify_board(&full_path).await?;
    } else {
        // Try to determine type from directory contents
        if full_path.is_dir() {
            let has_metadata = full_path.join("metadata.toml").exists();
            let has_board = full_path.join("board.toml").exists();

            if has_metadata {
                verify_package(&full_path, fetch).await?;
            } else if has_board {
                verify_board(&full_path).await?;
            } else {
                anyhow::bail!(
                    "Cannot determine type of '{}'. Expected metadata.toml (package) or board.toml (board)",
                    path
                );
            }
        } else {
            anyhow::bail!("Path '{}' is not a directory", path);
        }
    }

    Ok(())
}

/// Verify a package definition
async fn verify_package(pkg_path: &Path, fetch: bool) -> Result<()> {
    let pkg_name = pkg_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    println!("Verifying package '{}'...", pkg_name);

    // Check for metadata.toml
    let metadata_path = pkg_path.join("metadata.toml");
    if !metadata_path.exists() {
        anyhow::bail!(
            "Package '{}' is missing required metadata.toml file",
            pkg_name
        );
    }

    // Parse and validate metadata.toml
    let metadata_content = std::fs::read_to_string(&metadata_path)
        .map_err(|e| anyhow::anyhow!("Failed to read metadata.toml: {}", e))?;

    let metadata: toml::Value = toml::from_str(&metadata_content)
        .map_err(|e| anyhow::anyhow!("Failed to parse metadata.toml - TOML syntax error: {}", e))?;

    // Validate required fields in metadata.toml
    validate_package_metadata(&metadata, pkg_name)?;

    println!("  ✓ metadata.toml is valid");

    // Find and validate version files
    let version_files = find_version_files(pkg_path)?;
    if version_files.is_empty() {
        anyhow::bail!(
            "Package '{}' has no version files (e.g., 1.0.0.toml)",
            pkg_name
        );
    }

    for version_file in &version_files {
        let version_path = pkg_path.join(version_file);
        let version_content = std::fs::read_to_string(&version_path)
            .map_err(|e| anyhow::anyhow!("Failed to read {}: {}", version_file, e))?;

        let version: toml::Value = toml::from_str(&version_content)
            .map_err(|e| anyhow::anyhow!("Failed to parse {} - TOML syntax error: {}", version_file, e))?;

        validate_version_file(&version, version_file)?;
        println!("  ✓ {} is valid", version_file);

        // If --fetch, download and verify checksum
        if fetch {
            if let Some(source) = version.get("source") {
                if let (Some(url), Some(_sha256)) = (
                    source.get("url").and_then(|v| v.as_str()),
                    source.get("sha256").and_then(|v| v.as_str()),
                ) {
                    println!("  → Fetching source from {}...", url);
                    // Note: Actual download would happen here
                    // For now, we just acknowledge the flag
                    println!("  ⚠ Fetch verification not yet implemented");
                }
            }
        }
    }

    println!();
    println!("✓ Package '{}' is valid", pkg_name);

    Ok(())
}

/// Validate package metadata.toml required fields
fn validate_package_metadata(metadata: &toml::Value, pkg_name: &str) -> Result<()> {
    let package = metadata.get("package").ok_or_else(|| {
        anyhow::anyhow!(
            "Package '{}' metadata.toml is missing required [package] section",
            pkg_name
        )
    })?;

    // Check required fields
    if package.get("name").and_then(|v| v.as_str()).is_none() {
        anyhow::bail!(
            "Package '{}' metadata.toml is missing required field: name",
            pkg_name
        );
    }

    if package.get("description").and_then(|v| v.as_str()).is_none() {
        anyhow::bail!(
            "Package '{}' metadata.toml is missing required field: description",
            pkg_name
        );
    }

    if package.get("license").and_then(|v| v.as_str()).is_none() {
        anyhow::bail!(
            "Package '{}' metadata.toml is missing required field: license",
            pkg_name
        );
    }

    Ok(())
}

/// Find version files in a package directory
fn find_version_files(pkg_path: &Path) -> Result<Vec<String>> {
    let mut versions = Vec::new();

    for entry in std::fs::read_dir(pkg_path)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();

        // Version files are *.toml but not metadata.toml
        if name.ends_with(".toml") && name != "metadata.toml" {
            versions.push(name);
        }
    }

    Ok(versions)
}

/// Validate version file required fields
fn validate_version_file(version: &toml::Value, filename: &str) -> Result<()> {
    // Check for version field (can be in [release] or top-level)
    let has_version = version
        .get("release")
        .and_then(|r| r.get("version"))
        .is_some()
        || version.get("version").is_some();

    if !has_version {
        anyhow::bail!(
            "Version file '{}' is missing required field: version",
            filename
        );
    }

    // Check for source section
    let source = version.get("source").ok_or_else(|| {
        anyhow::anyhow!(
            "Version file '{}' is missing required [source] section",
            filename
        )
    })?;

    // Check for url
    if source.get("url").and_then(|v| v.as_str()).is_none() {
        anyhow::bail!(
            "Version file '{}' is missing required field: source.url",
            filename
        );
    }

    // Check for sha256
    if source.get("sha256").and_then(|v| v.as_str()).is_none() {
        anyhow::bail!(
            "Version file '{}' is missing required field: source.sha256",
            filename
        );
    }

    Ok(())
}

/// Verify a board definition
async fn verify_board(board_path: &Path) -> Result<()> {
    let board_name = board_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    println!("Verifying board '{}'...", board_name);

    // Check for board.toml
    let board_toml_path = board_path.join("board.toml");
    if !board_toml_path.exists() {
        anyhow::bail!(
            "Board '{}' is missing required board.toml file",
            board_name
        );
    }

    // Parse and validate board.toml
    let board_content = std::fs::read_to_string(&board_toml_path)
        .map_err(|e| anyhow::anyhow!("Failed to read board.toml: {}", e))?;

    let board: toml::Value = toml::from_str(&board_content)
        .map_err(|e| anyhow::anyhow!("Failed to parse board.toml - TOML syntax error: {}", e))?;

    // Validate required fields
    validate_board_definition(&board, board_name)?;

    println!("  ✓ board.toml is valid");
    println!();
    println!("✓ Board '{}' is valid", board_name);

    Ok(())
}

/// Validate board.toml required fields
fn validate_board_definition(board: &toml::Value, board_name: &str) -> Result<()> {
    let board_section = board.get("board").ok_or_else(|| {
        anyhow::anyhow!(
            "Board '{}' board.toml is missing required [board] section",
            board_name
        )
    })?;

    // Check required fields
    if board_section.get("name").and_then(|v| v.as_str()).is_none() {
        anyhow::bail!(
            "Board '{}' board.toml is missing required field: name",
            board_name
        );
    }

    if board_section
        .get("description")
        .and_then(|v| v.as_str())
        .is_none()
    {
        anyhow::bail!(
            "Board '{}' board.toml is missing required field: description",
            board_name
        );
    }

    let target = board_section
        .get("target")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Board '{}' board.toml is missing required field: target",
                board_name
            )
        })?;

    // Validate target is a known Zig target triple
    if !is_valid_zig_target(target) {
        eprintln!(
            "  ⚠ Warning: target '{}' is not a recognized Zig target triple",
            target
        );
    }

    if board_section.get("cpu").and_then(|v| v.as_str()).is_none() {
        anyhow::bail!(
            "Board '{}' board.toml is missing required field: cpu",
            board_name
        );
    }

    // Check for defaults section
    let defaults = board.get("defaults").ok_or_else(|| {
        anyhow::anyhow!(
            "Board '{}' board.toml is missing required [defaults] section",
            board_name
        )
    })?;

    if defaults
        .get("image_format")
        .and_then(|v| v.as_str())
        .is_none()
    {
        anyhow::bail!(
            "Board '{}' board.toml is missing required field: defaults.image_format",
            board_name
        );
    }

    if defaults
        .get("rootfs_size")
        .and_then(|v| v.as_str())
        .is_none()
    {
        anyhow::bail!(
            "Board '{}' board.toml is missing required field: defaults.rootfs_size",
            board_name
        );
    }

    if defaults.get("hostname").and_then(|v| v.as_str()).is_none() {
        anyhow::bail!(
            "Board '{}' board.toml is missing required field: defaults.hostname",
            board_name
        );
    }

    Ok(())
}

/// Check if a target is a valid Zig target triple
fn is_valid_zig_target(target: &str) -> bool {
    // Check against known targets
    if VALID_ZIG_TARGETS.contains(&target) {
        return true;
    }

    // Also accept targets that follow the pattern: arch-os-abi
    let parts: Vec<&str> = target.split('-').collect();
    if parts.len() >= 3 {
        let arch = parts[0];
        let os = parts[1];

        // Valid architectures
        let valid_archs = [
            "arm", "aarch64", "x86_64", "i386", "riscv64", "mips", "mipsel", "powerpc", "powerpc64",
        ];

        // Valid OS
        let valid_os = ["linux"];

        return valid_archs.contains(&arch) && valid_os.contains(&os);
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_zig_target() {
        assert!(is_valid_zig_target("arm-linux-musleabihf"));
        assert!(is_valid_zig_target("aarch64-linux-musl"));
        assert!(is_valid_zig_target("x86_64-linux-gnu"));
        assert!(!is_valid_zig_target("not-a-valid-target"));
        assert!(!is_valid_zig_target("invalid"));
    }
}
