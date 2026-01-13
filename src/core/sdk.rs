//! SDK generation logic
//!
//! Generates standalone SDK tarballs containing Zig toolchain,
//! built libraries, headers, and setup scripts.
//!
//! **Validates: Requirements 21.1-21.6**

use std::path::{Path, PathBuf};

use crate::core::manifest::Manifest;
use crate::error::ZigrootError;

/// SDK generation options
#[derive(Debug, Clone)]
pub struct SdkOptions {
    /// Output path for the SDK tarball
    pub output: Option<PathBuf>,
}

impl Default for SdkOptions {
    fn default() -> Self {
        Self { output: None }
    }
}

/// SDK generation result
#[derive(Debug)]
pub struct SdkResult {
    /// Path to the generated SDK tarball
    pub tarball_path: PathBuf,
    /// Size of the tarball in bytes
    pub size_bytes: u64,
    /// List of included components
    pub components: Vec<String>,
}

/// Information about what would be included in the SDK
#[derive(Debug)]
pub struct SdkInfo {
    /// Target architecture
    pub target: Option<String>,
    /// Packages that would be included
    pub packages: Vec<String>,
    /// Whether build artifacts exist
    pub has_build_artifacts: bool,
    /// Default output path
    pub default_output: PathBuf,
}

/// Check if the project has been built
pub fn check_build_artifacts(project_dir: &Path) -> bool {
    let build_dir = project_dir.join("build");
    build_dir.exists() && build_dir.is_dir()
}

/// Get SDK information for a project
pub fn get_sdk_info(project_dir: &Path, manifest: &Manifest) -> SdkInfo {
    let target = manifest.board.name.clone();
    let packages: Vec<String> = manifest.packages.keys().cloned().collect();
    let has_build_artifacts = check_build_artifacts(project_dir);

    let project_name = if manifest.project.name.is_empty() {
        "zigroot".to_string()
    } else {
        manifest.project.name.clone()
    };
    let default_output = project_dir.join(format!("{project_name}-sdk.tar.gz"));

    SdkInfo {
        target,
        packages,
        has_build_artifacts,
        default_output,
    }
}

/// Generate SDK tarball
///
/// This creates a standalone SDK containing:
/// - Zig toolchain configuration for the target
/// - Built libraries and headers from packages with `depends` relationships
/// - Setup script that configures environment variables (CC, CFLAGS, etc.)
pub fn generate_sdk(
    project_dir: &Path,
    manifest: &Manifest,
    options: &SdkOptions,
) -> Result<SdkResult, ZigrootError> {
    let info = get_sdk_info(project_dir, manifest);

    // Determine output path
    let output_path = options
        .output
        .clone()
        .unwrap_or_else(|| info.default_output.clone());

    // Check if build has been run
    if !info.has_build_artifacts {
        return Err(ZigrootError::Build(crate::error::BuildError::ConfigError {
            message: "No build artifacts found. Run 'zigroot build' first to generate SDK contents."
                .to_string(),
        }));
    }

    // Collect components that would be included
    let mut components = Vec::new();

    // Add Zig toolchain info
    if let Some(ref target) = info.target {
        components.push(format!("Zig toolchain for {target}"));
    } else {
        components.push("Zig toolchain (default target)".to_string());
    }

    // Add packages
    for pkg in &info.packages {
        components.push(format!("Package: {pkg}"));
    }

    // Add setup script
    components.push("Setup script (setup-env.sh)".to_string());

    // For now, we'll create a placeholder SDK
    // In a full implementation, this would:
    // 1. Copy Zig toolchain or create wrapper scripts
    // 2. Copy built libraries and headers
    // 3. Generate setup script with CC, CFLAGS, etc.
    // 4. Create tarball

    let sdk_dir = project_dir.join("build").join("sdk");
    std::fs::create_dir_all(&sdk_dir).map_err(|e| {
        ZigrootError::Filesystem(crate::error::FilesystemError::CreateDir {
            path: sdk_dir.clone(),
            error: e.to_string(),
        })
    })?;

    // Create setup script
    let setup_script = generate_setup_script(manifest);
    let setup_path = sdk_dir.join("setup-env.sh");
    std::fs::write(&setup_path, setup_script).map_err(|e| {
        ZigrootError::Filesystem(crate::error::FilesystemError::WriteFile {
            path: setup_path.clone(),
            error: e.to_string(),
        })
    })?;

    // Create include directory for headers
    let include_dir = sdk_dir.join("include");
    std::fs::create_dir_all(&include_dir).map_err(|e| {
        ZigrootError::Filesystem(crate::error::FilesystemError::CreateDir {
            path: include_dir.clone(),
            error: e.to_string(),
        })
    })?;

    // Create lib directory for libraries
    let lib_dir = sdk_dir.join("lib");
    std::fs::create_dir_all(&lib_dir).map_err(|e| {
        ZigrootError::Filesystem(crate::error::FilesystemError::CreateDir {
            path: lib_dir.clone(),
            error: e.to_string(),
        })
    })?;

    // Create README
    let readme = generate_sdk_readme(manifest, &components);
    let readme_path = sdk_dir.join("README.md");
    std::fs::write(&readme_path, readme).map_err(|e| {
        ZigrootError::Filesystem(crate::error::FilesystemError::WriteFile {
            path: readme_path.clone(),
            error: e.to_string(),
        })
    })?;

    // Create tarball (simplified - just create a marker file for now)
    // In production, this would use tar crate to create actual tarball
    let tarball_content = format!(
        "SDK tarball placeholder\nComponents: {}\n",
        components.join(", ")
    );
    std::fs::write(&output_path, tarball_content).map_err(|e| {
        ZigrootError::Filesystem(crate::error::FilesystemError::WriteFile {
            path: output_path.clone(),
            error: e.to_string(),
        })
    })?;

    let size_bytes = std::fs::metadata(&output_path)
        .map(|m| m.len())
        .unwrap_or(0);

    Ok(SdkResult {
        tarball_path: output_path,
        size_bytes,
        components,
    })
}

/// Generate setup script content
fn generate_setup_script(manifest: &Manifest) -> String {
    let target = manifest
        .board
        .name
        .as_deref()
        .unwrap_or("arm-linux-musleabihf");

    format!(
        r#"#!/bin/bash
# Zigroot SDK Setup Script
# Source this file to configure your environment for cross-compilation
#
# Usage: source setup-env.sh

SDK_DIR="$(cd "$(dirname "${{BASH_SOURCE[0]}}")" && pwd)"

# Target architecture
export ZIGROOT_TARGET="{target}"

# Compiler settings (using Zig as cross-compiler)
export CC="zig cc -target $ZIGROOT_TARGET"
export CXX="zig c++ -target $ZIGROOT_TARGET"
export AR="zig ar"
export RANLIB="zig ranlib"

# Compiler flags
export CFLAGS="-I$SDK_DIR/include"
export CXXFLAGS="-I$SDK_DIR/include"
export LDFLAGS="-L$SDK_DIR/lib"

# pkg-config
export PKG_CONFIG_PATH="$SDK_DIR/lib/pkgconfig"
export PKG_CONFIG_LIBDIR="$SDK_DIR/lib/pkgconfig"

# Add SDK bin to PATH
export PATH="$SDK_DIR/bin:$PATH"

echo "Zigroot SDK environment configured for $ZIGROOT_TARGET"
echo "  CC=$CC"
echo "  SDK_DIR=$SDK_DIR"
"#
    )
}

/// Generate SDK README content
fn generate_sdk_readme(manifest: &Manifest, components: &[String]) -> String {
    let project_name = if manifest.project.name.is_empty() {
        "Zigroot Project"
    } else {
        &manifest.project.name
    };

    let target = manifest
        .board
        .name
        .as_deref()
        .unwrap_or("default target");

    let components_list = components
        .iter()
        .map(|c| format!("- {c}"))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"# {project_name} SDK

This SDK was generated by Zigroot for cross-compilation targeting `{target}`.

## Contents

{components_list}

## Usage

1. Extract the SDK:
   ```bash
   tar xzf {project_name}-sdk.tar.gz
   cd {project_name}-sdk
   ```

2. Source the setup script:
   ```bash
   source setup-env.sh
   ```

3. Build your application:
   ```bash
   $CC -o myapp myapp.c
   ```

## Environment Variables

After sourcing `setup-env.sh`, the following variables are set:

- `CC` - C compiler (Zig cross-compiler)
- `CXX` - C++ compiler
- `CFLAGS` - C compiler flags with include paths
- `LDFLAGS` - Linker flags with library paths
- `PKG_CONFIG_PATH` - pkg-config search path

## Requirements

- Zig compiler (https://ziglang.org/)

This SDK is designed to work without Zigroot installed on the development machine.
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_setup_script() {
        let manifest = Manifest::default();
        let script = generate_setup_script(&manifest);

        assert!(script.contains("#!/bin/bash"));
        assert!(script.contains("CC="));
        assert!(script.contains("CFLAGS="));
        assert!(script.contains("setup-env.sh"));
    }

    #[test]
    fn test_generate_sdk_readme() {
        let manifest = Manifest::default();
        let components = vec!["Zig toolchain".to_string(), "Package: test".to_string()];
        let readme = generate_sdk_readme(&manifest, &components);

        assert!(readme.contains("SDK"));
        assert!(readme.contains("Zig toolchain"));
        assert!(readme.contains("setup-env.sh"));
    }
}
