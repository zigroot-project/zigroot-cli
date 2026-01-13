//! Interactive configuration (TUI) logic
//!
//! Provides TUI-based configuration for zigroot projects.
//!
//! **Validates: Requirements 25.1-25.17**

use std::path::Path;

use crate::core::manifest::Manifest;
use crate::error::ZigrootError;

/// Configuration categories available in the TUI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigCategory {
    /// Board selection
    Board,
    /// Package selection
    Packages,
    /// Build options
    BuildOptions,
    /// External artifacts
    ExternalArtifacts,
}

impl std::fmt::Display for ConfigCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Board => write!(f, "Board"),
            Self::Packages => write!(f, "Packages"),
            Self::BuildOptions => write!(f, "Build Options"),
            Self::ExternalArtifacts => write!(f, "External Artifacts"),
        }
    }
}

/// TUI configuration state
#[derive(Debug)]
pub struct ConfigState {
    /// Current manifest (if loaded)
    pub manifest: Option<Manifest>,
    /// Whether the terminal is interactive
    pub is_interactive: bool,
    /// Selected category
    pub selected_category: ConfigCategory,
    /// Whether changes have been made
    pub has_changes: bool,
}

impl ConfigState {
    /// Create a new configuration state
    pub fn new(manifest: Option<Manifest>, is_interactive: bool) -> Self {
        Self {
            manifest,
            is_interactive,
            selected_category: ConfigCategory::Board,
            has_changes: false,
        }
    }
}

/// Check if the terminal is interactive
pub fn is_terminal_interactive() -> bool {
    use std::io::IsTerminal;

    // Check TERM environment variable
    if let Ok(term) = std::env::var("TERM") {
        if term == "dumb" || term.is_empty() {
            return false;
        }
    }

    // Check if stdin/stdout are TTYs
    std::io::stdin().is_terminal() && std::io::stdout().is_terminal()
}

/// Load manifest for configuration
pub fn load_manifest_for_config(project_dir: &Path) -> Result<Manifest, ZigrootError> {
    Manifest::load(project_dir)
}

/// Get available packages for selection
pub fn get_available_packages(project_dir: &Path) -> Vec<String> {
    let packages_dir = project_dir.join("packages");
    if !packages_dir.exists() {
        return Vec::new();
    }

    let mut packages = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&packages_dir) {
        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                if let Some(name) = entry.file_name().to_str() {
                    packages.push(name.to_string());
                }
            }
        }
    }
    packages.sort();
    packages
}

/// Get package dependencies
pub fn get_package_dependencies(project_dir: &Path, package_name: &str) -> Vec<String> {
    let package_toml = project_dir
        .join("packages")
        .join(package_name)
        .join("package.toml");

    if !package_toml.exists() {
        return Vec::new();
    }

    if let Ok(content) = std::fs::read_to_string(&package_toml) {
        if let Ok(value) = content.parse::<toml::Value>() {
            if let Some(package) = value.get("package") {
                if let Some(depends) = package.get("depends") {
                    if let Some(deps) = depends.as_array() {
                        return deps
                            .iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect();
                    }
                }
            }
        }
    }

    Vec::new()
}

/// Get packages that depend on a given package
pub fn get_package_dependents(project_dir: &Path, package_name: &str) -> Vec<String> {
    let packages = get_available_packages(project_dir);
    let mut dependents = Vec::new();

    for pkg in packages {
        if pkg == package_name {
            continue;
        }
        let deps = get_package_dependencies(project_dir, &pkg);
        if deps.contains(&package_name.to_string()) {
            dependents.push(pkg);
        }
    }

    dependents
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_category_display() {
        assert_eq!(ConfigCategory::Board.to_string(), "Board");
        assert_eq!(ConfigCategory::Packages.to_string(), "Packages");
        assert_eq!(ConfigCategory::BuildOptions.to_string(), "Build Options");
        assert_eq!(
            ConfigCategory::ExternalArtifacts.to_string(),
            "External Artifacts"
        );
    }

    #[test]
    fn test_config_state_new() {
        let state = ConfigState::new(None, false);
        assert!(state.manifest.is_none());
        assert!(!state.is_interactive);
        assert_eq!(state.selected_category, ConfigCategory::Board);
        assert!(!state.has_changes);
    }
}
