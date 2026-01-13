//! Output formatting and progress indicators
//!
//! This module provides utilities for displaying progress bars,
//! colored output, and formatted messages to the user.
//!
//! ## Output Modes
//!
//! - **Normal**: Full output with colors and progress indicators
//! - **Quiet**: Only errors are displayed
//! - **JSON**: Machine-readable JSON output for scripting
//!
//! ## Color Coding
//!
//! - Green (✓): Success messages
//! - Red (✗): Error messages
//! - Yellow (⚠): Warning messages
//! - Blue (ℹ): Informational messages

use indicatif::{ProgressBar, ProgressStyle};
use serde::Serialize;
use std::io::{self, IsTerminal, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// Global output configuration
static QUIET_MODE: AtomicBool = AtomicBool::new(false);
static JSON_MODE: AtomicBool = AtomicBool::new(false);

/// Output configuration for CLI commands
#[derive(Debug, Clone, Default)]
pub struct OutputConfig {
    /// Suppress all output except errors
    pub quiet: bool,
    /// Output in JSON format
    pub json: bool,
    /// Verbose level (0 = normal, 1 = info, 2 = debug)
    pub verbose: u8,
}

impl OutputConfig {
    /// Create a new output configuration
    pub fn new(quiet: bool, json: bool, verbose: u8) -> Self {
        Self { quiet, json, verbose }
    }

    /// Apply this configuration globally
    pub fn apply_global(&self) {
        QUIET_MODE.store(self.quiet, Ordering::SeqCst);
        JSON_MODE.store(self.json, Ordering::SeqCst);
    }
}

/// Check if quiet mode is enabled
pub fn is_quiet() -> bool {
    QUIET_MODE.load(Ordering::SeqCst)
}

/// Check if JSON mode is enabled
pub fn is_json() -> bool {
    JSON_MODE.load(Ordering::SeqCst)
}

/// Check if output is interactive (terminal)
pub fn is_interactive() -> bool {
    io::stdout().is_terminal() && !is_quiet() && !is_json()
}

/// Create a spinner for operations with unknown duration
pub fn create_spinner(message: &str) -> ProgressBar {
    if !is_interactive() {
        return ProgressBar::hidden();
    }

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template("{spinner:.blue} {msg}")
            .expect("Invalid spinner template"),
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(std::time::Duration::from_millis(80));
    pb
}

/// Create a progress bar for downloads
pub fn create_download_bar(total: u64) -> ProgressBar {
    if !is_interactive() {
        return ProgressBar::hidden();
    }

    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .expect("Invalid progress bar template")
            .progress_chars("█▓▒░"),
    );
    pb
}

/// Create a progress bar for build steps
pub fn create_build_bar(total: u64) -> ProgressBar {
    if !is_interactive() {
        return ProgressBar::hidden();
    }

    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} packages ({msg})")
            .expect("Invalid progress bar template")
            .progress_chars("█▓▒░"),
    );
    pb
}

/// Status message prefixes
pub mod status {
    /// Success prefix (green checkmark)
    pub const SUCCESS: &str = "✓";

    /// Error prefix (red X)
    pub const ERROR: &str = "✗";

    /// Warning prefix (yellow triangle)
    pub const WARNING: &str = "⚠";

    /// Info prefix (blue circle)
    pub const INFO: &str = "ℹ";
}

/// Print a success message
pub fn print_success(message: &str) {
    if is_json() {
        let output = JsonOutput::success(message);
        println!("{}", serde_json::to_string(&output).unwrap_or_default());
    } else if !is_quiet() {
        println!("{} {message}", status::SUCCESS);
    }
}

/// Print an error message (always shown, even in quiet mode)
pub fn print_error(message: &str) {
    if is_json() {
        let output = JsonOutput::error(message);
        eprintln!("{}", serde_json::to_string(&output).unwrap_or_default());
    } else {
        eprintln!("{} {message}", status::ERROR);
    }
}

/// Print a warning message
pub fn print_warning(message: &str) {
    if is_json() {
        let output = JsonOutput::warning(message);
        println!("{}", serde_json::to_string(&output).unwrap_or_default());
    } else if !is_quiet() {
        println!("{} {message}", status::WARNING);
    }
}

/// Print an info message
pub fn print_info(message: &str) {
    if is_json() {
        let output = JsonOutput::info(message);
        println!("{}", serde_json::to_string(&output).unwrap_or_default());
    } else if !is_quiet() {
        println!("{} {message}", status::INFO);
    }
}

/// Print a plain message (no prefix)
pub fn print_plain(message: &str) {
    if !is_quiet() && !is_json() {
        println!("{message}");
    }
}

/// Print indented detail line
pub fn print_detail(message: &str) {
    if !is_quiet() && !is_json() {
        println!("  {message}");
    }
}

/// JSON output structure for machine-readable output
#[derive(Debug, Serialize)]
pub struct JsonOutput {
    /// Status: "success", "error", "warning", "info"
    pub status: String,
    /// Message content
    pub message: String,
    /// Optional additional data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl JsonOutput {
    /// Create a success output
    pub fn success(message: &str) -> Self {
        Self {
            status: "success".to_string(),
            message: message.to_string(),
            data: None,
        }
    }

    /// Create an error output
    pub fn error(message: &str) -> Self {
        Self {
            status: "error".to_string(),
            message: message.to_string(),
            data: None,
        }
    }

    /// Create a warning output
    pub fn warning(message: &str) -> Self {
        Self {
            status: "warning".to_string(),
            message: message.to_string(),
            data: None,
        }
    }

    /// Create an info output
    pub fn info(message: &str) -> Self {
        Self {
            status: "info".to_string(),
            message: message.to_string(),
            data: None,
        }
    }

    /// Create output with additional data
    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }
}

/// Build summary statistics
#[derive(Debug, Clone, Serialize)]
pub struct BuildSummary {
    /// Total build time
    pub total_time: Duration,
    /// Number of packages built
    pub packages_built: usize,
    /// Total packages
    pub total_packages: usize,
    /// Image size in bytes (if applicable)
    pub image_size: Option<u64>,
    /// Whether build was successful
    pub success: bool,
}

impl BuildSummary {
    /// Create a new build summary
    pub fn new(start_time: Instant, packages_built: usize, total_packages: usize) -> Self {
        Self {
            total_time: start_time.elapsed(),
            packages_built,
            total_packages,
            image_size: None,
            success: true,
        }
    }

    /// Set the image size
    pub fn with_image_size(mut self, size: u64) -> Self {
        self.image_size = Some(size);
        self
    }

    /// Mark as failed
    pub fn failed(mut self) -> Self {
        self.success = false;
        self
    }

    /// Display the summary banner
    pub fn display(&self) {
        if is_json() {
            let output = JsonOutput {
                status: if self.success { "success" } else { "error" }.to_string(),
                message: "Build complete".to_string(),
                data: Some(serde_json::to_value(self).unwrap_or_default()),
            };
            println!("{}", serde_json::to_string_pretty(&output).unwrap_or_default());
            return;
        }

        if is_quiet() {
            return;
        }

        println!();
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

        if self.success {
            println!("{} Build completed successfully", status::SUCCESS);
        } else {
            println!("{} Build failed", status::ERROR);
        }

        println!();
        println!(
            "  Packages: {}/{} built",
            self.packages_built, self.total_packages
        );
        println!("  Time:     {:.2}s", self.total_time.as_secs_f64());

        if let Some(size) = self.image_size {
            println!("  Image:    {}", format_size(size));
        }

        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    }
}

/// Format a byte size as human-readable string
pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

/// Error suggestion helper
pub mod suggestions {
    use crate::error::{
        BoardError, BuildError, DownloadError, InitError, PackageError, ResolverError,
        ZigrootError,
    };

    /// Get a suggestion for a given error
    pub fn get_suggestion(error: &anyhow::Error) -> Option<String> {
        // Check for ZigrootError variants
        if let Some(e) = error.downcast_ref::<ZigrootError>() {
            return get_zigroot_suggestion(e);
        }

        // Check for specific error types
        if let Some(e) = error.downcast_ref::<PackageError>() {
            return get_package_suggestion(e);
        }

        if let Some(e) = error.downcast_ref::<BoardError>() {
            return get_board_suggestion(e);
        }

        if let Some(e) = error.downcast_ref::<InitError>() {
            return get_init_suggestion(e);
        }

        if let Some(e) = error.downcast_ref::<DownloadError>() {
            return get_download_suggestion(e);
        }

        if let Some(e) = error.downcast_ref::<BuildError>() {
            return get_build_suggestion(e);
        }

        if let Some(e) = error.downcast_ref::<ResolverError>() {
            return get_resolver_suggestion(e);
        }

        // Check for IO errors
        if let Some(e) = error.downcast_ref::<std::io::Error>() {
            return get_io_suggestion(e);
        }

        None
    }

    fn get_zigroot_suggestion(error: &ZigrootError) -> Option<String> {
        match error {
            ZigrootError::ManifestNotFound { .. } => {
                Some("Run 'zigroot init' to create a new project".to_string())
            }
            ZigrootError::ManifestParse { .. } => {
                Some("Check your zigroot.toml for syntax errors".to_string())
            }
            ZigrootError::Package(e) => get_package_suggestion(e),
            ZigrootError::Board(e) => get_board_suggestion(e),
            ZigrootError::Init(e) => get_init_suggestion(e),
            ZigrootError::Download(e) => get_download_suggestion(e),
            ZigrootError::Build(e) => get_build_suggestion(e),
            ZigrootError::Resolver(e) => get_resolver_suggestion(e),
            _ => None,
        }
    }

    fn get_package_suggestion(error: &PackageError) -> Option<String> {
        match error {
            PackageError::NotFound { name } => {
                Some(format!("Run 'zigroot search {name}' to find similar packages"))
            }
            PackageError::ChecksumMismatch { .. } => {
                Some("Try 'zigroot fetch --force' to re-download".to_string())
            }
            PackageError::MissingField { field, .. } => {
                Some(format!("Add the '{field}' field to your package definition"))
            }
            PackageError::MultipleSourceTypes { .. } => {
                Some("Specify only one source type: url, git, or sources".to_string())
            }
            PackageError::NoSourceType { .. } => {
                Some("Add a source type: url+sha256, git+ref, or sources".to_string())
            }
            PackageError::GitWithoutRef { .. } => {
                Some("Add a tag, branch, or rev to your git source".to_string())
            }
            PackageError::UrlWithoutChecksum { .. } => {
                Some("Add a sha256 checksum for the URL source".to_string())
            }
            _ => None,
        }
    }

    fn get_board_suggestion(error: &BoardError) -> Option<String> {
        match error {
            BoardError::NotFound { name } => {
                Some(format!("Run 'zigroot search --boards {name}' to find similar boards"))
            }
            BoardError::MissingField { field, .. } => {
                Some(format!("Add the '{field}' field to your board definition"))
            }
            BoardError::IncompatiblePackage { package, .. } => {
                Some(format!("Remove '{package}' or choose a compatible board"))
            }
            _ => None,
        }
    }

    fn get_init_suggestion(error: &InitError) -> Option<String> {
        match error {
            InitError::DirectoryNotEmpty { .. } => {
                Some("Use --force to initialize in a non-empty directory".to_string())
            }
            InitError::BoardNotFound { name } => {
                Some(format!("Run 'zigroot board list' to see available boards, or check if '{name}' is spelled correctly"))
            }
            _ => None,
        }
    }

    fn get_download_suggestion(error: &DownloadError) -> Option<String> {
        match error {
            DownloadError::NetworkError { .. } => {
                Some("Check your internet connection and try again".to_string())
            }
            DownloadError::ChecksumFailed { .. } => {
                Some("Try 'zigroot fetch --force' to re-download".to_string())
            }
            DownloadError::MaxRetriesExceeded { .. } => {
                Some("Check your internet connection or try again later".to_string())
            }
            _ => None,
        }
    }

    fn get_build_suggestion(error: &BuildError) -> Option<String> {
        match error {
            BuildError::ToolchainNotFound { toolchain } => {
                Some(format!("Install {toolchain} or run 'zigroot doctor' to check dependencies"))
            }
            BuildError::ConfigError { .. } => {
                Some("Check your zigroot.toml configuration".to_string())
            }
            _ => None,
        }
    }

    fn get_resolver_suggestion(error: &ResolverError) -> Option<String> {
        match error {
            ResolverError::CircularDependency { .. } => {
                Some("Run 'zigroot tree' to visualize dependencies".to_string())
            }
            ResolverError::Conflict { .. } => {
                Some("Try updating packages or adjusting version constraints".to_string())
            }
            ResolverError::MissingDependency { dependency, .. } => {
                Some(format!("Run 'zigroot add {dependency}' to add the missing package"))
            }
        }
    }

    fn get_io_suggestion(error: &std::io::Error) -> Option<String> {
        match error.kind() {
            std::io::ErrorKind::PermissionDenied => {
                Some("Check file permissions or run with appropriate privileges".to_string())
            }
            std::io::ErrorKind::NotFound => {
                Some("Ensure the file or directory exists".to_string())
            }
            std::io::ErrorKind::AlreadyExists => {
                Some("The file or directory already exists".to_string())
            }
            _ => None,
        }
    }
}

/// Display an error with optional suggestion
pub fn display_error(error: &anyhow::Error) {
    if is_json() {
        let suggestion = suggestions::get_suggestion(error);
        let mut output = JsonOutput::error(&error.to_string());
        if let Some(suggestion) = suggestion {
            output = output.with_data(serde_json::json!({
                "suggestion": suggestion,
                "causes": error.chain().skip(1).map(|e| e.to_string()).collect::<Vec<_>>()
            }));
        }
        eprintln!("{}", serde_json::to_string_pretty(&output).unwrap_or_default());
        return;
    }

    // Print main error
    print_error(&error.to_string());

    // Print cause chain
    for cause in error.chain().skip(1) {
        eprintln!("  caused by: {cause}");
    }

    // Print suggestion if available
    if let Some(suggestion) = suggestions::get_suggestion(error) {
        eprintln!();
        eprintln!("{} Suggestion: {suggestion}", status::INFO);
    }
}

/// Flush stdout to ensure output is displayed
pub fn flush() {
    let _ = io::stdout().flush();
}
