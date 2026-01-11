//! Output formatting and progress indicators
//!
//! This module provides utilities for displaying progress bars,
//! colored output, and formatted messages to the user.

use indicatif::{ProgressBar, ProgressStyle};

/// Create a spinner for operations with unknown duration
pub fn create_spinner(message: &str) -> ProgressBar {
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
