//! Version management for zigroot
//!
//! This module handles:
//! - Minimum zigroot version checking for packages and boards
//! - Semver comparison and constraint parsing
//! - Self-update checking
//!
//! **Validates: Requirements 30.1-30.8, 31.1-31.10**

use semver::{Version, VersionReq};
use thiserror::Error;

/// Current zigroot version from Cargo.toml
pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Errors related to version checking
#[derive(Error, Debug, PartialEq)]
pub enum VersionError {
    /// Current zigroot version doesn't satisfy the required constraint
    #[error("Zigroot version {current} does not satisfy requirement '{constraint}' from {origin}. Please update zigroot to continue. Run 'zigroot update --self' to check for updates.")]
    VersionMismatch {
        current: String,
        constraint: String,
        origin: String,
    },

    /// Invalid version constraint format
    #[error("Invalid version constraint '{constraint}': {reason}")]
    InvalidConstraint { constraint: String, reason: String },

    /// Invalid version format
    #[error("Invalid version '{version}': {reason}")]
    InvalidVersion { version: String, reason: String },
}

/// Check if the current zigroot version satisfies a version constraint
///
/// # Arguments
/// * `constraint` - A semver constraint string (e.g., ">=0.2.0", "^1.0", "~1.2")
/// * `origin` - Description of where the constraint came from (for error messages)
///
/// # Returns
/// * `Ok(())` if the current version satisfies the constraint
/// * `Err(VersionError)` if the constraint is not satisfied or invalid
///
/// # Examples
/// ```
/// use zigroot::core::version::check_zigroot_version;
///
/// // This will check against the current zigroot version
/// let result = check_zigroot_version(">=0.1.0", "package 'busybox'");
/// ```
pub fn check_zigroot_version(constraint: &str, origin: &str) -> Result<(), VersionError> {
    check_version_constraint(CURRENT_VERSION, constraint, origin)
}

/// Check if a version satisfies a constraint (internal function for testing)
///
/// # Arguments
/// * `version` - The version to check
/// * `constraint` - A semver constraint string
/// * `origin` - Description of where the constraint came from
pub fn check_version_constraint(
    version: &str,
    constraint: &str,
    origin: &str,
) -> Result<(), VersionError> {
    let parsed_version = Version::parse(version).map_err(|e| VersionError::InvalidVersion {
        version: version.to_string(),
        reason: e.to_string(),
    })?;

    let version_req =
        VersionReq::parse(constraint).map_err(|e| VersionError::InvalidConstraint {
            constraint: constraint.to_string(),
            reason: e.to_string(),
        })?;

    if version_req.matches(&parsed_version) {
        Ok(())
    } else {
        Err(VersionError::VersionMismatch {
            current: version.to_string(),
            constraint: constraint.to_string(),
            origin: origin.to_string(),
        })
    }
}

/// Parse and validate a semver version string
///
/// # Arguments
/// * `version` - A semver version string (e.g., "1.2.3", "0.1.0-alpha")
///
/// # Returns
/// * `Ok(Version)` if the version is valid
/// * `Err(VersionError)` if the version is invalid
pub fn parse_version(version: &str) -> Result<Version, VersionError> {
    Version::parse(version).map_err(|e| VersionError::InvalidVersion {
        version: version.to_string(),
        reason: e.to_string(),
    })
}

/// Parse and validate a semver version constraint
///
/// # Arguments
/// * `constraint` - A semver constraint string (e.g., ">=1.0.0", "^2.0", "~1.2")
///
/// # Returns
/// * `Ok(VersionReq)` if the constraint is valid
/// * `Err(VersionError)` if the constraint is invalid
pub fn parse_constraint(constraint: &str) -> Result<VersionReq, VersionError> {
    VersionReq::parse(constraint).map_err(|e| VersionError::InvalidConstraint {
        constraint: constraint.to_string(),
        reason: e.to_string(),
    })
}

/// Compare two versions
///
/// # Returns
/// * `Ordering::Less` if v1 < v2
/// * `Ordering::Equal` if v1 == v2
/// * `Ordering::Greater` if v1 > v2
pub fn compare_versions(v1: &str, v2: &str) -> Result<std::cmp::Ordering, VersionError> {
    let parsed_v1 = parse_version(v1)?;
    let parsed_v2 = parse_version(v2)?;
    Ok(parsed_v1.cmp(&parsed_v2))
}

/// Check if version v1 is newer than v2
pub fn is_newer(v1: &str, v2: &str) -> Result<bool, VersionError> {
    Ok(compare_versions(v1, v2)? == std::cmp::Ordering::Greater)
}

/// Information about a zigroot release
#[derive(Debug, Clone, PartialEq)]
pub struct ReleaseInfo {
    /// Version string
    pub version: String,
    /// Release URL
    pub url: String,
    /// Release notes (if available)
    pub notes: Option<String>,
    /// Published date
    pub published_at: Option<String>,
}

/// Result of checking for updates
#[derive(Debug, Clone, PartialEq)]
pub enum UpdateCheckResult {
    /// A newer version is available
    UpdateAvailable {
        current: String,
        latest: String,
        release_url: String,
    },
    /// Already on the latest version
    UpToDate { current: String },
    /// Could not check for updates
    CheckFailed { reason: String },
}

/// Detected installation method for zigroot
#[derive(Debug, Clone, PartialEq)]
pub enum InstallMethod {
    /// Installed via cargo
    Cargo,
    /// Installed via Homebrew
    Homebrew,
    /// Installed via AUR
    Aur,
    /// Installed as a standalone binary
    Binary,
    /// Unknown installation method
    Unknown,
}

impl InstallMethod {
    /// Get update instructions for this installation method
    pub fn update_instructions(&self) -> &'static str {
        match self {
            Self::Cargo => "Run: cargo install zigroot --force",
            Self::Homebrew => "Run: brew upgrade zigroot",
            Self::Aur => "Run: yay -Syu zigroot",
            Self::Binary => "Download the latest release from https://github.com/zigroot-project/zigroot-cli/releases",
            Self::Unknown => "Visit https://github.com/zigroot-project/zigroot-cli/releases for installation instructions",
        }
    }
}

/// Detect how zigroot was installed
pub fn detect_install_method() -> InstallMethod {
    // Check if running from cargo install location
    if let Ok(exe_path) = std::env::current_exe() {
        let path_str = exe_path.to_string_lossy();

        // Check for cargo install path
        if path_str.contains(".cargo/bin") {
            return InstallMethod::Cargo;
        }

        // Check for Homebrew path
        if path_str.contains("/opt/homebrew/") || path_str.contains("/usr/local/Cellar/") {
            return InstallMethod::Homebrew;
        }

        // Check for AUR/pacman path
        if path_str.contains("/usr/bin/") && cfg!(target_os = "linux") {
            // Could be AUR or system package
            if std::process::Command::new("pacman")
                .args(["-Qi", "zigroot"])
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
            {
                return InstallMethod::Aur;
            }
        }
    }

    InstallMethod::Unknown
}

/// GitHub releases API URL
pub const GITHUB_RELEASES_API: &str =
    "https://api.github.com/repos/zigroot-project/zigroot-cli/releases/latest";

/// Check for updates by querying GitHub releases
///
/// This is an async function that queries the GitHub API.
/// For synchronous usage, use `check_for_updates_sync`.
pub async fn check_for_updates() -> UpdateCheckResult {
    check_for_updates_with_client(&reqwest::Client::new()).await
}

/// Check for updates using a provided HTTP client (for testing)
pub async fn check_for_updates_with_client(client: &reqwest::Client) -> UpdateCheckResult {
    let response = match client
        .get(GITHUB_RELEASES_API)
        .header("User-Agent", format!("zigroot/{}", CURRENT_VERSION))
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            return UpdateCheckResult::CheckFailed {
                reason: format!("Network error: {e}"),
            }
        }
    };

    if !response.status().is_success() {
        return UpdateCheckResult::CheckFailed {
            reason: format!("GitHub API returned status {}", response.status()),
        };
    }

    let json: serde_json::Value = match response.json().await {
        Ok(j) => j,
        Err(e) => {
            return UpdateCheckResult::CheckFailed {
                reason: format!("Failed to parse response: {e}"),
            }
        }
    };

    let tag_name = match json.get("tag_name").and_then(|v| v.as_str()) {
        Some(t) => t,
        None => {
            return UpdateCheckResult::CheckFailed {
                reason: "No tag_name in response".to_string(),
            }
        }
    };

    // Remove 'v' prefix if present
    let latest_version = tag_name.strip_prefix('v').unwrap_or(tag_name);

    let release_url = json
        .get("html_url")
        .and_then(|v| v.as_str())
        .unwrap_or("https://github.com/zigroot-project/zigroot-cli/releases")
        .to_string();

    // Compare versions
    match is_newer(latest_version, CURRENT_VERSION) {
        Ok(true) => UpdateCheckResult::UpdateAvailable {
            current: CURRENT_VERSION.to_string(),
            latest: latest_version.to_string(),
            release_url,
        },
        Ok(false) => UpdateCheckResult::UpToDate {
            current: CURRENT_VERSION.to_string(),
        },
        Err(_) => UpdateCheckResult::CheckFailed {
            reason: format!("Failed to compare versions: {latest_version} vs {CURRENT_VERSION}"),
        },
    }
}

/// Format update check result for display
pub fn format_update_result(result: &UpdateCheckResult, install_method: &InstallMethod) -> String {
    match result {
        UpdateCheckResult::UpdateAvailable {
            current,
            latest,
            release_url,
        } => {
            format!(
                "A new version of zigroot is available!\n\n\
                 Current version: {current}\n\
                 Latest version:  {latest}\n\n\
                 {}\n\n\
                 Release notes: {release_url}",
                install_method.update_instructions()
            )
        }
        UpdateCheckResult::UpToDate { current } => {
            format!("zigroot {current} is the latest version.")
        }
        UpdateCheckResult::CheckFailed { reason } => {
            format!("Could not check for updates: {reason}")
        }
    }
}

// ============================================
// Background Update Check
// ============================================

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

/// Cache file name for update check results
const UPDATE_CACHE_FILE: &str = "update_check.json";

/// How often to check for updates (24 hours)
const UPDATE_CHECK_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

/// Cached update check result
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CachedUpdateCheck {
    /// When the check was performed
    pub checked_at: u64,
    /// The result of the check
    pub result: CachedResult,
}

/// Serializable version of UpdateCheckResult
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum CachedResult {
    /// A newer version is available
    UpdateAvailable {
        current: String,
        latest: String,
        release_url: String,
    },
    /// Already on the latest version
    UpToDate { current: String },
    /// Could not check for updates
    CheckFailed { reason: String },
}

impl From<UpdateCheckResult> for CachedResult {
    fn from(result: UpdateCheckResult) -> Self {
        match result {
            UpdateCheckResult::UpdateAvailable {
                current,
                latest,
                release_url,
            } => Self::UpdateAvailable {
                current,
                latest,
                release_url,
            },
            UpdateCheckResult::UpToDate { current } => Self::UpToDate { current },
            UpdateCheckResult::CheckFailed { reason } => Self::CheckFailed { reason },
        }
    }
}

impl From<CachedResult> for UpdateCheckResult {
    fn from(cached: CachedResult) -> Self {
        match cached {
            CachedResult::UpdateAvailable {
                current,
                latest,
                release_url,
            } => Self::UpdateAvailable {
                current,
                latest,
                release_url,
            },
            CachedResult::UpToDate { current } => Self::UpToDate { current },
            CachedResult::CheckFailed { reason } => Self::CheckFailed { reason },
        }
    }
}

/// Get the path to the update cache file
pub fn get_update_cache_path() -> Option<PathBuf> {
    dirs::cache_dir().map(|d| d.join("zigroot").join(UPDATE_CACHE_FILE))
}

/// Check if we should perform an update check (at most once per day)
pub fn should_check_for_updates() -> bool {
    let cache_path = match get_update_cache_path() {
        Some(p) => p,
        None => return true, // No cache dir, always check
    };

    if !cache_path.exists() {
        return true;
    }

    // Read the cache file
    let content = match std::fs::read_to_string(&cache_path) {
        Ok(c) => c,
        Err(_) => return true, // Can't read cache, check again
    };

    let cached: CachedUpdateCheck = match serde_json::from_str(&content) {
        Ok(c) => c,
        Err(_) => return true, // Invalid cache, check again
    };

    // Check if enough time has passed
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let elapsed = now.saturating_sub(cached.checked_at);
    elapsed >= UPDATE_CHECK_INTERVAL.as_secs()
}

/// Save update check result to cache
pub fn save_update_cache(result: &UpdateCheckResult) -> Result<(), std::io::Error> {
    let cache_path = match get_update_cache_path() {
        Some(p) => p,
        None => return Ok(()), // No cache dir, skip caching
    };

    // Create cache directory if needed
    if let Some(parent) = cache_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let cached = CachedUpdateCheck {
        checked_at: now,
        result: result.clone().into(),
    };

    let content = serde_json::to_string_pretty(&cached)?;
    std::fs::write(&cache_path, content)?;

    Ok(())
}

/// Load cached update check result
pub fn load_update_cache() -> Option<CachedUpdateCheck> {
    let cache_path = get_update_cache_path()?;

    if !cache_path.exists() {
        return None;
    }

    let content = std::fs::read_to_string(&cache_path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Format a non-intrusive update notification
pub fn format_update_notification(result: &UpdateCheckResult) -> Option<String> {
    match result {
        UpdateCheckResult::UpdateAvailable {
            current,
            latest,
            release_url: _,
        } => Some(format!(
            "\n💡 A new version of zigroot is available: {current} → {latest}\n   Run 'zigroot update --self' for details.\n"
        )),
        _ => None,
    }
}

/// Perform a background update check if needed
///
/// This function:
/// 1. Checks if enough time has passed since the last check
/// 2. If so, performs an update check in the background
/// 3. Caches the result
/// 4. Returns a notification message if an update is available
///
/// This is designed to be called at the start of any command.
pub async fn background_update_check() -> Option<String> {
    // Check if we should perform an update check
    if !should_check_for_updates() {
        // Check if we have a cached result with an available update
        if let Some(cached) = load_update_cache() {
            let result: UpdateCheckResult = cached.result.into();
            return format_update_notification(&result);
        }
        return None;
    }

    // Perform the check
    let result = check_for_updates().await;

    // Cache the result (ignore errors)
    let _ = save_update_cache(&result);

    // Return notification if update available
    format_update_notification(&result)
}
