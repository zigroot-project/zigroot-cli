//! Error types for zigroot
//!
//! Domain-specific error types using thiserror.

use std::path::PathBuf;
use thiserror::Error;

/// Project initialization errors
#[derive(Error, Debug)]
pub enum InitError {
    /// Directory not found
    #[error("Directory not found: {path}")]
    DirectoryNotFound { path: PathBuf },

    /// Directory is not empty
    #[error("Directory is not empty: {path}. Use --force to initialize anyway")]
    DirectoryNotEmpty { path: PathBuf },

    /// IO error during initialization
    #[error("IO error for '{path}': {error}")]
    IoError { path: PathBuf, error: String },

    /// Manifest generation/parsing error
    #[error("Failed to create manifest: {error}")]
    ManifestError { error: String },

    /// Board not found in registry
    #[error("Board '{name}' not found in registry")]
    BoardNotFound { name: String },

    /// Registry error
    #[error("Registry error: {error}")]
    RegistryError { error: String },
}

/// Package-related errors
#[derive(Error, Debug)]
pub enum PackageError {
    /// Package not found in registry
    #[error("Package '{name}' not found in registry")]
    NotFound { name: String },

    /// Version constraint cannot be satisfied
    #[error("Version constraint '{constraint}' cannot be satisfied for '{package}'")]
    VersionConflict { package: String, constraint: String },

    /// Checksum mismatch
    #[error("Checksum mismatch for '{file}': expected {expected}, got {actual}")]
    ChecksumMismatch {
        file: String,
        expected: String,
        actual: String,
    },

    /// Missing required field
    #[error("Package '{package}' is missing required field '{field}'")]
    MissingField { package: String, field: String },

    /// Multiple source types specified
    #[error("Package '{package}' specifies multiple source types (only one allowed)")]
    MultipleSourceTypes { package: String },

    /// No source type specified
    #[error("Package '{package}' has no source type (url, git, or sources required)")]
    NoSourceType { package: String },

    /// Git source without ref
    #[error("Package '{package}' specifies git source without tag, branch, or rev")]
    GitWithoutRef { package: String },

    /// URL source without checksum
    #[error("Package '{package}' specifies url source without sha256 checksum")]
    UrlWithoutChecksum { package: String },

    /// Parse error
    #[error("Failed to parse package definition: {0}")]
    ParseError(String),
}

/// Board-related errors
#[derive(Error, Debug)]
pub enum BoardError {
    /// Board not found
    #[error("Board '{name}' not found in registry")]
    NotFound { name: String },

    /// Missing required field
    #[error("Board '{board}' is missing required field '{field}'")]
    MissingField { board: String, field: String },

    /// Incompatible with packages
    #[error("Board '{board}' is incompatible with package '{package}'")]
    IncompatiblePackage { board: String, package: String },

    /// Parse error
    #[error("Failed to parse board definition: {0}")]
    ParseError(String),
}

/// Dependency resolution errors
#[derive(Error, Debug)]
pub enum ResolverError {
    /// Circular dependency detected
    #[error("Circular dependency detected: {}", cycle.join(" -> "))]
    CircularDependency { cycle: Vec<String> },

    /// Dependency conflict
    #[error("Dependency conflict: {message}")]
    Conflict { message: String },

    /// Missing dependency
    #[error("Missing dependency: '{dependency}' required by '{package}'")]
    MissingDependency { package: String, dependency: String },
}

/// Download errors
#[derive(Error, Debug)]
pub enum DownloadError {
    /// Network error
    #[error("Network error downloading '{url}': {error}")]
    NetworkError { url: String, error: String },

    /// Checksum verification failed
    #[error("Checksum verification failed for '{file}'")]
    ChecksumFailed { file: String },

    /// IO error
    #[error("IO error for '{path}': {error}")]
    IoError { path: PathBuf, error: String },

    /// Max retries exceeded
    #[error("Download failed after {retries} retries: {url}")]
    MaxRetriesExceeded { url: String, retries: u32 },
}

/// Filesystem errors
#[derive(Error, Debug)]
pub enum FilesystemError {
    /// Failed to create directory
    #[error("Failed to create directory '{path}': {error}")]
    CreateDir { path: PathBuf, error: String },

    /// Failed to remove directory
    #[error("Failed to remove directory '{path}': {error}")]
    RemoveDir { path: PathBuf, error: String },

    /// Failed to write file
    #[error("Failed to write file '{path}': {error}")]
    WriteFile { path: PathBuf, error: String },

    /// Failed to read file
    #[error("Failed to read file '{path}': {error}")]
    ReadFile { path: PathBuf, error: String },
}

/// Build errors
#[derive(Error, Debug)]
pub enum BuildError {
    /// Build failed
    #[error("Build failed for package '{package}': {error}")]
    BuildFailed { package: String, error: String },

    /// Toolchain not found
    #[error("Toolchain not found: {toolchain}")]
    ToolchainNotFound { toolchain: String },

    /// Configuration error
    #[error("Configuration error: {message}")]
    ConfigError { message: String },
}

/// Option validation errors
#[derive(Error, Debug)]
pub enum OptionError {
    /// Invalid option type
    #[error("Option '{name}' has invalid type: expected {expected}, got {got}")]
    InvalidType {
        name: String,
        expected: String,
        got: String,
    },

    /// Invalid choice value
    #[error("Option '{name}' has invalid value '{value}': must be one of {choices:?}")]
    InvalidChoice {
        name: String,
        value: String,
        choices: Vec<String>,
    },

    /// Pattern mismatch
    #[error("Option '{name}' value '{value}' does not match pattern '{pattern}'")]
    PatternMismatch {
        name: String,
        value: String,
        pattern: String,
    },

    /// Empty not allowed
    #[error("Option '{name}' cannot be empty")]
    EmptyNotAllowed { name: String },

    /// Out of range
    #[error("Option '{name}' value {value} is out of range (min: {min:?}, max: {max:?})")]
    OutOfRange {
        name: String,
        value: f64,
        min: Option<f64>,
        max: Option<f64>,
    },

    /// Invalid pattern
    #[error("Option '{name}' has invalid pattern '{pattern}': {error}")]
    InvalidPattern {
        name: String,
        pattern: String,
        error: String,
    },
}


/// Top-level zigroot error type
#[derive(Error, Debug)]
pub enum ZigrootError {
    /// Manifest error
    #[error("Manifest error: {0}")]
    Manifest(String),

    /// Manifest not found
    #[error("Manifest not found at '{path}'. Run 'zigroot init' to create a project.")]
    ManifestNotFound { path: String },

    /// Manifest parse error
    #[error("Failed to parse manifest: {source}")]
    ManifestParse { source: toml::de::Error },

    /// Package error
    #[error("Package error: {0}")]
    Package(#[from] PackageError),

    /// Board error
    #[error("Board error: {0}")]
    Board(#[from] BoardError),

    /// Resolver error
    #[error("Resolver error: {0}")]
    Resolver(#[from] ResolverError),

    /// Build error
    #[error("Build error: {0}")]
    Build(#[from] BuildError),

    /// Download error
    #[error("Download error: {0}")]
    Download(#[from] DownloadError),

    /// Filesystem error
    #[error("Filesystem error: {0}")]
    Filesystem(#[from] FilesystemError),

    /// Init error
    #[error("Init error: {0}")]
    Init(#[from] InitError),

    /// Option error
    #[error("Option error: {0}")]
    Option(#[from] OptionError),

    /// IO error
    #[error("IO error: {source}")]
    Io { source: std::io::Error },

    /// Generic error
    #[error("{0}")]
    Generic(String),
}
