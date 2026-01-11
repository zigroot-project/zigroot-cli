//! Registry client implementation
//!
//! Fetches package and board definitions from GitHub raw URLs.

use crate::config::urls;

/// Registry client for fetching packages and boards
#[derive(Debug)]
pub struct RegistryClient {
    /// HTTP client
    client: reqwest::Client,
    /// Package registry base URL
    package_registry_url: String,
    /// Board registry base URL
    board_registry_url: String,
}

impl RegistryClient {
    /// Create a new registry client with default URLs
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            package_registry_url: urls::PACKAGE_REGISTRY.to_string(),
            board_registry_url: urls::BOARD_REGISTRY.to_string(),
        }
    }

    /// Create a registry client with custom URLs
    pub fn with_urls(package_url: String, board_url: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            package_registry_url: package_url,
            board_registry_url: board_url,
        }
    }

    /// Get the package registry URL
    pub fn package_registry_url(&self) -> &str {
        &self.package_registry_url
    }

    /// Get the board registry URL
    pub fn board_registry_url(&self) -> &str {
        &self.board_registry_url
    }

    /// Get the HTTP client
    pub fn client(&self) -> &reqwest::Client {
        &self.client
    }
}

impl Default for RegistryClient {
    fn default() -> Self {
        Self::new()
    }
}
