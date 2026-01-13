//! Package fetch logic
//!
//! This module contains the business logic for downloading package sources
//! and external artifacts. It handles checksum verification, parallel downloads,
//! and caching of already downloaded files.

use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::core::lock::LockFile;
use crate::core::manifest::Manifest;
use crate::infra::download::{verify_checksum, DownloadManager};

/// Errors that can occur during fetch
#[derive(Error, Debug)]
pub enum FetchError {
    /// Manifest error
    #[error("Failed to read manifest: {0}")]
    ManifestError(String),

    /// Lock file error
    #[error("Failed to read lock file: {0}")]
    LockError(String),

    /// Download error
    #[error("Failed to download '{name}': {error}")]
    DownloadError { name: String, error: String },

    /// Checksum error
    #[error("Checksum verification failed for '{name}'")]
    ChecksumError { name: String },

    /// IO error
    #[error("IO error: {0}")]
    IoError(String),
}

/// Options for fetching packages
#[derive(Debug, Clone)]
pub struct FetchOptions {
    /// Number of parallel downloads
    pub parallel: usize,
    /// Force re-download even if files exist
    pub force: bool,
}

impl Default for FetchOptions {
    fn default() -> Self {
        Self {
            parallel: 4,
            force: false,
        }
    }
}

/// Information about a downloaded package
#[derive(Debug, Clone)]
pub struct DownloadedPackage {
    /// Package name
    pub name: String,
    /// Package version
    pub version: String,
    /// Path to downloaded file
    pub path: PathBuf,
}

/// Result of fetching packages
#[derive(Debug, Default)]
pub struct FetchResult {
    /// Successfully downloaded packages
    pub downloaded: Vec<DownloadedPackage>,
    /// Packages that were skipped (already downloaded)
    pub skipped: Vec<String>,
    /// External artifacts that were downloaded
    pub external_downloaded: Vec<String>,
    /// External artifacts that were skipped
    pub external_skipped: Vec<String>,
    /// Failed downloads with error messages
    pub failed: Vec<(String, String)>,
}

/// Fetch all packages and external artifacts for a project
pub async fn fetch_packages(
    project_path: &Path,
    options: &FetchOptions,
) -> Result<FetchResult, FetchError> {
    let manifest_path = project_path.join("zigroot.toml");
    let lock_path = project_path.join("zigroot.lock");
    let downloads_dir = project_path.join("downloads");
    let external_dir = project_path.join("external");

    // Load manifest
    let manifest_content = std::fs::read_to_string(&manifest_path)
        .map_err(|e| FetchError::ManifestError(e.to_string()))?;
    let manifest = Manifest::from_toml(&manifest_content)
        .map_err(|e| FetchError::ManifestError(e.to_string()))?;

    // Load lock file if it exists
    let lock_file = if lock_path.exists() {
        Some(LockFile::load(&lock_path).map_err(|e| FetchError::LockError(e.to_string()))?)
    } else {
        None
    };

    // Create downloads directory
    std::fs::create_dir_all(&downloads_dir).map_err(|e| FetchError::IoError(e.to_string()))?;

    let mut result = FetchResult::default();
    let download_manager = DownloadManager::new();

    // Fetch packages
    for (package_name, package_ref) in &manifest.packages {
        let download_result = fetch_single_package(
            &download_manager,
            project_path,
            &downloads_dir,
            package_name,
            package_ref,
            lock_file.as_ref(),
            options,
        )
        .await;

        match download_result {
            Ok(Some(downloaded)) => {
                result.downloaded.push(downloaded);
            }
            Ok(None) => {
                result.skipped.push(package_name.clone());
            }
            Err(e) => {
                result.failed.push((package_name.clone(), e.to_string()));
            }
        }
    }

    // Fetch external artifacts
    for (artifact_name, artifact) in &manifest.external {
        let artifact_result = fetch_external_artifact(
            &download_manager,
            project_path,
            &external_dir,
            artifact_name,
            artifact,
            options,
        )
        .await;

        match artifact_result {
            Ok(true) => {
                result.external_downloaded.push(artifact_name.clone());
            }
            Ok(false) => {
                result.external_skipped.push(artifact_name.clone());
            }
            Err(e) => {
                result.failed.push((artifact_name.clone(), e.to_string()));
            }
        }
    }

    Ok(result)
}

/// Fetch a single package
async fn fetch_single_package(
    download_manager: &DownloadManager,
    project_path: &Path,
    downloads_dir: &Path,
    package_name: &str,
    package_ref: &crate::core::manifest::PackageRef,
    lock_file: Option<&LockFile>,
    options: &FetchOptions,
) -> Result<Option<DownloadedPackage>, FetchError> {
    // Check if this is a local package
    let local_package_path = project_path.join("packages").join(package_name);
    if local_package_path.exists() {
        // Local package, no download needed
        return Ok(None);
    }

    // Get version from package ref or lock file
    let version = package_ref
        .version
        .clone()
        .or_else(|| {
            lock_file
                .and_then(|lf| lf.get_package(package_name))
                .map(|p| p.version.clone())
        })
        .unwrap_or_else(|| "latest".to_string());

    // Determine download URL and checksum
    let (url, expected_checksum) =
        get_package_download_info(package_name, &version, package_ref, lock_file);

    // Determine destination path
    let filename = format!("{package_name}-{version}.tar.gz");
    let dest_path = downloads_dir.join(&filename);

    // Check if already downloaded with valid checksum
    if !options.force && dest_path.exists() {
        if let Some(ref checksum) = expected_checksum {
            if verify_checksum(&dest_path, checksum).unwrap_or(false) {
                return Ok(None); // Already downloaded and valid
            }
            // Checksum mismatch, delete and re-download
            let _ = std::fs::remove_file(&dest_path);
        } else {
            // No checksum to verify, assume valid
            return Ok(None);
        }
    }

    // Download the package
    if let Some(download_url) = url {
        let download_result = if let Some(ref checksum) = expected_checksum {
            download_manager
                .download_verified(&download_url, &dest_path, checksum, None)
                .await
        } else {
            download_manager
                .download(&download_url, &dest_path, None)
                .await
        };

        match download_result {
            Ok(_) => Ok(Some(DownloadedPackage {
                name: package_name.to_string(),
                version,
                path: dest_path,
            })),
            Err(e) => Err(FetchError::DownloadError {
                name: package_name.to_string(),
                error: e.to_string(),
            }),
        }
    } else {
        // No URL available, might be a git source or registry package
        // For now, we'll skip these as they require different handling
        Ok(None)
    }
}

/// Get download URL and checksum for a package
fn get_package_download_info(
    package_name: &str,
    version: &str,
    package_ref: &crate::core::manifest::PackageRef,
    lock_file: Option<&LockFile>,
) -> (Option<String>, Option<String>) {
    // Check if it's a git source
    if package_ref.git.is_some() {
        // Git sources are handled differently (clone, not download)
        return (None, None);
    }

    // Try to get info from lock file
    if let Some(lf) = lock_file {
        if let Some(locked_pkg) = lf.get_package(package_name) {
            // Check if source is a URL
            if let Some(ref source) = locked_pkg.source {
                if source.starts_with("http://") || source.starts_with("https://") {
                    let checksum = if locked_pkg.sha256 != "pending" && locked_pkg.sha256 != "local"
                    {
                        Some(locked_pkg.sha256.clone())
                    } else {
                        None
                    };
                    return (Some(source.clone()), checksum);
                }
            }

            // Use default registry URL pattern
            let checksum = if locked_pkg.sha256 != "pending" && locked_pkg.sha256 != "local" {
                Some(locked_pkg.sha256.clone())
            } else {
                None
            };

            // Construct URL from registry pattern
            let url = format!(
                "https://raw.githubusercontent.com/zigroot-project/zigroot-packages/main/packages/{package_name}/{version}.tar.gz"
            );
            return (Some(url), checksum);
        }
    }

    // Construct default URL from registry pattern
    let url = format!(
        "https://raw.githubusercontent.com/zigroot-project/zigroot-packages/main/packages/{package_name}/{version}.tar.gz"
    );
    (Some(url), None)
}

/// Fetch an external artifact
async fn fetch_external_artifact(
    download_manager: &DownloadManager,
    project_path: &Path,
    external_dir: &Path,
    artifact_name: &str,
    artifact: &crate::core::manifest::ExternalArtifact,
    options: &FetchOptions,
) -> Result<bool, FetchError> {
    // If artifact has a local path, check if it exists
    if let Some(ref local_path) = artifact.path {
        let full_path = project_path.join(local_path);
        if full_path.exists() {
            // Verify checksum if provided
            if let Some(ref checksum) = artifact.sha256 {
                if !verify_checksum(&full_path, checksum).unwrap_or(false) {
                    return Err(FetchError::ChecksumError {
                        name: artifact_name.to_string(),
                    });
                }
            }
            return Ok(false); // Already exists locally
        }
    }

    // If artifact has a URL, download it
    if let Some(ref url) = artifact.url {
        // Create external directory
        std::fs::create_dir_all(external_dir).map_err(|e| FetchError::IoError(e.to_string()))?;

        // Determine destination path
        let dest_path = if let Some(ref local_path) = artifact.path {
            project_path.join(local_path)
        } else {
            // Extract filename from URL or use artifact name
            let filename = url.rsplit('/').next().unwrap_or(artifact_name);
            external_dir.join(filename)
        };

        // Check if already downloaded with valid checksum
        if !options.force && dest_path.exists() {
            if let Some(ref checksum) = artifact.sha256 {
                if verify_checksum(&dest_path, checksum).unwrap_or(false) {
                    return Ok(false); // Already downloaded and valid
                }
                // Checksum mismatch, delete and re-download
                let _ = std::fs::remove_file(&dest_path);
            } else {
                // No checksum to verify, assume valid
                return Ok(false);
            }
        }

        // Download the artifact
        let download_result = if let Some(ref checksum) = artifact.sha256 {
            download_manager
                .download_verified(url, &dest_path, checksum, None)
                .await
        } else {
            download_manager.download(url, &dest_path, None).await
        };

        match download_result {
            Ok(_) => Ok(true),
            Err(e) => Err(FetchError::DownloadError {
                name: artifact_name.to_string(),
                error: e.to_string(),
            }),
        }
    } else {
        // No URL and no local path, or local path doesn't exist
        if artifact.path.is_some() {
            Err(FetchError::IoError(format!(
                "Local artifact '{artifact_name}' not found"
            )))
        } else {
            Ok(false) // Nothing to download
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fetch_options_default() {
        let options = FetchOptions::default();
        assert_eq!(options.parallel, 4);
        assert!(!options.force);
    }

    #[test]
    fn test_fetch_result_default() {
        let result = FetchResult::default();
        assert!(result.downloaded.is_empty());
        assert!(result.skipped.is_empty());
        assert!(result.external_downloaded.is_empty());
        assert!(result.external_skipped.is_empty());
        assert!(result.failed.is_empty());
    }
}
