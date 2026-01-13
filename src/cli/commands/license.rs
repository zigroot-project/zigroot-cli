//! CLI command for `zigroot license`
//!
//! Displays license information and generates reports.
//!
//! **Validates: Requirements 22.1-22.6**

use anyhow::Result;
use std::path::Path;

use crate::core::license::collect_licenses;
use crate::core::manifest::Manifest;
use crate::error::ZigrootError;

/// Execute the license command
pub async fn execute(
    project_dir: &Path,
    export: Option<String>,
    sbom: bool,
) -> Result<()> {
    // Load manifest
    let manifest_path = project_dir.join("zigroot.toml");
    if !manifest_path.exists() {
        return Err(ZigrootError::ManifestNotFound {
            path: manifest_path.display().to_string(),
        }
        .into());
    }

    let manifest = Manifest::load(&manifest_path)?;

    println!("📜 License Information\n");

    // Collect license information
    let report = collect_licenses(project_dir, &manifest);

    if report.packages.is_empty() {
        println!("No packages found in project.");
        return Ok(());
    }

    // Handle SBOM generation
    if sbom {
        let project_name = if manifest.project.name.is_empty() {
            "zigroot-project"
        } else {
            &manifest.project.name
        };

        let sbom_content = report.generate_sbom(project_name);
        let sbom_path = project_dir.join(format!("{project_name}-sbom.spdx"));

        std::fs::write(&sbom_path, &sbom_content).map_err(|e| {
            ZigrootError::Filesystem(crate::error::FilesystemError::WriteFile {
                path: sbom_path.clone(),
                error: e.to_string(),
            })
        })?;

        println!("✅ SPDX SBOM generated: {}", sbom_path.display());
        println!("\nSoftware Bill of Materials contains {} packages.", report.packages.len());
        return Ok(());
    }

    // Handle export
    if let Some(export_path) = export {
        let export_content = report.export_report();
        let path = Path::new(&export_path);

        std::fs::write(path, &export_content).map_err(|e| {
            ZigrootError::Filesystem(crate::error::FilesystemError::WriteFile {
                path: path.to_path_buf(),
                error: e.to_string(),
            })
        })?;

        println!("✅ License report exported to: {export_path}");
        println!("\nReport contains {} packages.", report.packages.len());

        if report.has_warnings() {
            println!();
            if !report.copyleft_packages.is_empty() {
                println!(
                    "⚠️  Copyleft licenses detected: {}",
                    report.copyleft_packages.join(", ")
                );
            }
            if !report.missing_licenses.is_empty() {
                println!(
                    "⚠️  Missing license info: {}",
                    report.missing_licenses.join(", ")
                );
            }
        }

        return Ok(());
    }

    // Display summary
    println!("{}", report.summary());

    Ok(())
}
