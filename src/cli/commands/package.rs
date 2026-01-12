//! Package subcommand implementations
//!
//! Implements `zigroot package list` and `zigroot package info`.
//!
//! **Validates: Requirements 2.10, 2.11**

use anyhow::Result;
use std::path::Path;

use crate::core::manifest::Manifest;

/// Execute the package list command
///
/// Displays all installed packages with their versions and descriptions.
/// **Validates: Requirement 2.10**
pub async fn execute_list(project_dir: &Path) -> Result<()> {
    let manifest_path = project_dir.join("zigroot.toml");

    if !manifest_path.exists() {
        anyhow::bail!("No zigroot.toml found. Run 'zigroot init' first.");
    }

    let content = std::fs::read_to_string(&manifest_path)?;
    let manifest = Manifest::from_toml(&content)
        .map_err(|e| anyhow::anyhow!("Failed to parse manifest: {}", e))?;

    if manifest.packages.is_empty() {
        println!("No packages installed.");
        return Ok(());
    }

    println!("Installed packages:");
    println!();

    for (name, pkg_ref) in &manifest.packages {
        let version = pkg_ref.version.as_deref().unwrap_or("latest");
        let source = get_source_info(pkg_ref);
        let description = get_package_description(name);

        println!("  {} @ {}", name, version);
        if !source.is_empty() {
            println!("    Source: {}", source);
        }
        if !description.is_empty() {
            println!("    Description: {}", description);
        }
        println!();
    }

    println!("{} package(s) installed.", manifest.packages.len());

    Ok(())
}

/// Execute the package info command
///
/// Displays detailed information about a specific package.
/// **Validates: Requirement 2.11**
pub async fn execute_info(project_dir: &Path, package_name: &str) -> Result<()> {
    let manifest_path = project_dir.join("zigroot.toml");

    if !manifest_path.exists() {
        anyhow::bail!("No zigroot.toml found. Run 'zigroot init' first.");
    }

    let content = std::fs::read_to_string(&manifest_path)?;
    let manifest = Manifest::from_toml(&content)
        .map_err(|e| anyhow::anyhow!("Failed to parse manifest: {}", e))?;

    let pkg_ref = manifest.packages.get(package_name).ok_or_else(|| {
        anyhow::anyhow!(
            "Package '{}' not found in manifest. Use 'zigroot add {}' to install it.",
            package_name,
            package_name
        )
    })?;

    // Display package information
    println!("Package: {}", package_name);
    println!();

    // Version
    let version = pkg_ref.version.as_deref().unwrap_or("latest");
    println!("  Version: {}", version);

    // Source
    let source = get_source_info(pkg_ref);
    if !source.is_empty() {
        println!("  Source: {}", source);
    }

    // Description (from registry or local)
    let description = get_package_description(package_name);
    if !description.is_empty() {
        println!("  Description: {}", description);
    }

    // License
    let license = get_package_license(package_name);
    if !license.is_empty() {
        println!("  License: {}", license);
    }

    // Homepage
    let homepage = get_package_homepage(package_name);
    if !homepage.is_empty() {
        println!("  Homepage: {}", homepage);
    }

    // Dependencies
    let dependencies = get_package_dependencies(package_name);
    if !dependencies.is_empty() {
        println!("  Dependencies: {}", dependencies.join(", "));
    } else {
        println!("  Dependencies: none");
    }

    // Git info if applicable
    if let Some(git) = &pkg_ref.git {
        println!("  Git: {}", git);
        if let Some(ref_) = &pkg_ref.ref_ {
            println!("  Ref: {}", ref_);
        }
    }

    // Registry info if applicable
    if let Some(registry) = &pkg_ref.registry {
        println!("  Registry: {}", registry);
    }

    // Options if any
    if !pkg_ref.options.is_empty() {
        println!("  Options:");
        for (key, value) in &pkg_ref.options {
            println!("    {}: {}", key, value);
        }
    }

    Ok(())
}

/// Get source information for a package reference
fn get_source_info(pkg_ref: &crate::core::manifest::PackageRef) -> String {
    if let Some(git) = &pkg_ref.git {
        let ref_info = pkg_ref.ref_.as_deref().unwrap_or("HEAD");
        format!("git: {}#{}", git, ref_info)
    } else if let Some(registry) = &pkg_ref.registry {
        format!("registry: {}", registry)
    } else {
        "registry: default".to_string()
    }
}

/// Get package description from registry or local cache
/// This is a placeholder - in a real implementation, this would query the registry
fn get_package_description(package_name: &str) -> String {
    // Common package descriptions for well-known packages
    match package_name {
        "busybox" => "Swiss army knife of embedded Linux - provides many common UNIX utilities".to_string(),
        "dropbear" => "Lightweight SSH server and client".to_string(),
        "nginx" => "High-performance HTTP server and reverse proxy".to_string(),
        "zlib" => "General-purpose lossless data compression library".to_string(),
        "openssl" => "Cryptography and SSL/TLS toolkit".to_string(),
        "curl" => "Command-line tool for transferring data with URLs".to_string(),
        _ => String::new(),
    }
}

/// Get package license from registry or local cache
/// This is a placeholder - in a real implementation, this would query the registry
fn get_package_license(package_name: &str) -> String {
    match package_name {
        "busybox" => "GPL-2.0".to_string(),
        "dropbear" => "MIT".to_string(),
        "nginx" => "BSD-2-Clause".to_string(),
        "zlib" => "Zlib".to_string(),
        "openssl" => "Apache-2.0".to_string(),
        "curl" => "MIT".to_string(),
        _ => String::new(),
    }
}

/// Get package homepage from registry or local cache
/// This is a placeholder - in a real implementation, this would query the registry
fn get_package_homepage(package_name: &str) -> String {
    match package_name {
        "busybox" => "https://busybox.net".to_string(),
        "dropbear" => "https://matt.ucc.asn.au/dropbear/dropbear.html".to_string(),
        "nginx" => "https://nginx.org".to_string(),
        "zlib" => "https://zlib.net".to_string(),
        "openssl" => "https://www.openssl.org".to_string(),
        "curl" => "https://curl.se".to_string(),
        _ => String::new(),
    }
}

/// Get package dependencies from registry or local cache
/// This is a placeholder - in a real implementation, this would query the registry
fn get_package_dependencies(package_name: &str) -> Vec<String> {
    match package_name {
        "nginx" => vec!["zlib".to_string(), "openssl".to_string()],
        "curl" => vec!["zlib".to_string(), "openssl".to_string()],
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_source_info_git() {
        let pkg_ref = crate::core::manifest::PackageRef {
            version: None,
            git: Some("https://github.com/example/repo".to_string()),
            ref_: Some("v1.0.0".to_string()),
            registry: None,
            options: std::collections::HashMap::new(),
        };

        let source = get_source_info(&pkg_ref);
        assert!(source.contains("git:"));
        assert!(source.contains("github.com"));
        assert!(source.contains("v1.0.0"));
    }

    #[test]
    fn test_get_source_info_registry() {
        let pkg_ref = crate::core::manifest::PackageRef {
            version: Some("1.0.0".to_string()),
            git: None,
            ref_: None,
            registry: Some("https://custom.registry.com".to_string()),
            options: std::collections::HashMap::new(),
        };

        let source = get_source_info(&pkg_ref);
        assert!(source.contains("registry:"));
        assert!(source.contains("custom.registry.com"));
    }

    #[test]
    fn test_get_source_info_default() {
        let pkg_ref = crate::core::manifest::PackageRef {
            version: Some("1.0.0".to_string()),
            git: None,
            ref_: None,
            registry: None,
            options: std::collections::HashMap::new(),
        };

        let source = get_source_info(&pkg_ref);
        assert!(source.contains("default"));
    }

    #[test]
    fn test_get_package_description() {
        assert!(!get_package_description("busybox").is_empty());
        assert!(get_package_description("unknown-pkg").is_empty());
    }

    #[test]
    fn test_get_package_license() {
        assert_eq!(get_package_license("busybox"), "GPL-2.0");
        assert!(get_package_license("unknown-pkg").is_empty());
    }
}
