//! HTTP download functionality
//!
//! Handles downloading files with progress reporting and checksum verification.

use sha2::{Digest, Sha256};
use std::path::Path;

use crate::config::defaults;
use crate::error::DownloadError;

/// Download manager for fetching files
#[derive(Debug)]
pub struct DownloadManager {
    /// HTTP client
    client: reqwest::Client,
    /// Maximum retry attempts
    max_retries: u32,
}

impl DownloadManager {
    /// Create a new download manager
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            max_retries: defaults::MAX_DOWNLOAD_RETRIES,
        }
    }

    /// Get the HTTP client
    pub fn client(&self) -> &reqwest::Client {
        &self.client
    }

    /// Get max retries
    pub fn max_retries(&self) -> u32 {
        self.max_retries
    }
}

impl Default for DownloadManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Verify SHA256 checksum of a file
pub fn verify_checksum(path: &Path, expected: &str) -> Result<bool, DownloadError> {
    let content = std::fs::read(path).map_err(|e| DownloadError::IoError {
        path: path.to_path_buf(),
        error: e.to_string(),
    })?;

    let mut hasher = Sha256::new();
    hasher.update(&content);
    let result = hasher.finalize();
    let actual = hex::encode(result);

    Ok(actual == expected.to_lowercase())
}
