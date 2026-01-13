//! CLI command for `zigroot sdk`
//!
//! Generates standalone SDK tarballs.
//!
//! **Validates: Requirements 21.1-21.6**

use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::core::manifest::Manifest;
use crate::core::sdk::{generate_sdk, get_sdk_info, SdkOptions};
use crate::error::ZigrootError;

/// Execute the sdk command
pub async fn execute(project_dir: &Path, output: Option<String>) -> Result<()> {
    println!("📦 Generating SDK...\n");

    // Load manifest
    let manifest_path = project_dir.join("zigroot.toml");
    if !manifest_path.exists() {
        return Err(ZigrootError::ManifestNotFound {
            path: manifest_path.display().to_string(),
        }
        .into());
    }

    let manifest = Manifest::load(&manifest_path)?;

    // Get SDK info
    let info = get_sdk_info(project_dir, &manifest);

    // Display what will be included
    println!("SDK Configuration:");
    if let Some(ref target) = info.target {
        println!("  Target: {target}");
    }
    println!("  Packages: {}", info.packages.len());
    for pkg in &info.packages {
        println!("    • {pkg}");
    }
    println!();

    // Check for build artifacts
    if !info.has_build_artifacts {
        println!("⚠️  No build artifacts found.");
        println!("   Run 'zigroot build' first to generate SDK contents.");
        println!();
        println!("The SDK will include:");
        println!("  • Zig toolchain configuration for cross-compilation");
        println!("  • Built libraries and headers from packages");
        println!("  • Setup script with environment variables (CC, CFLAGS, etc.)");
        return Err(anyhow::anyhow!(
            "Build required before SDK generation. Run 'zigroot build' first."
        ));
    }

    // Generate SDK
    let options = SdkOptions {
        output: output.map(PathBuf::from),
    };

    match generate_sdk(project_dir, &manifest, &options) {
        Ok(result) => {
            println!("✅ SDK generated successfully!");
            println!();
            println!("Output: {}", result.tarball_path.display());
            println!("Size: {} bytes", result.size_bytes);
            println!();
            println!("Components included:");
            for component in &result.components {
                println!("  • {component}");
            }
            println!();
            println!("To use the SDK:");
            println!("  1. Extract: tar xzf {}", result.tarball_path.display());
            println!("  2. Source: source setup-env.sh");
            println!("  3. Build: $CC -o myapp myapp.c");
            Ok(())
        }
        Err(e) => {
            println!("❌ SDK generation failed: {e}");
            Err(e.into())
        }
    }
}
