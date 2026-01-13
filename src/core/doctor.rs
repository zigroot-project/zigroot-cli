//! Doctor command logic
//!
//! Checks system dependencies and reports issues with suggestions.
//!
//! **Validates: Requirements 14.5, 14.6**

use std::path::Path;

/// Result of a single dependency check
#[derive(Debug, Clone)]
pub struct CheckResult {
    /// Name of the dependency being checked
    pub name: String,
    /// Whether the check passed
    pub passed: bool,
    /// Version if available
    pub version: Option<String>,
    /// Error message if check failed
    pub error: Option<String>,
    /// Suggestion for fixing the issue
    pub suggestion: Option<String>,
    /// Whether this is a required or optional dependency
    pub required: bool,
}

impl CheckResult {
    /// Create a passing check result
    pub fn pass(name: &str, version: Option<String>, required: bool) -> Self {
        Self {
            name: name.to_string(),
            passed: true,
            version,
            error: None,
            suggestion: None,
            required,
        }
    }

    /// Create a failing check result
    pub fn fail(name: &str, error: &str, suggestion: Option<&str>, required: bool) -> Self {
        Self {
            name: name.to_string(),
            passed: false,
            version: None,
            error: Some(error.to_string()),
            suggestion: suggestion.map(String::from),
            required,
        }
    }
}

/// Overall doctor report
#[derive(Debug, Default)]
pub struct DoctorReport {
    /// Individual check results
    pub checks: Vec<CheckResult>,
    /// Configuration issues found
    pub config_issues: Vec<String>,
}

impl DoctorReport {
    /// Create a new empty report
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a check result
    pub fn add_check(&mut self, result: CheckResult) {
        self.checks.push(result);
    }

    /// Add a configuration issue
    pub fn add_config_issue(&mut self, issue: String) {
        self.config_issues.push(issue);
    }

    /// Check if all required checks passed
    pub fn all_required_passed(&self) -> bool {
        self.checks
            .iter()
            .filter(|c| c.required)
            .all(|c| c.passed)
    }

    /// Check if all checks passed (including optional)
    pub fn all_passed(&self) -> bool {
        self.checks.iter().all(|c| c.passed) && self.config_issues.is_empty()
    }

    /// Count passed checks
    pub fn passed_count(&self) -> usize {
        self.checks.iter().filter(|c| c.passed).count()
    }

    /// Count failed checks
    pub fn failed_count(&self) -> usize {
        self.checks.iter().filter(|c| !c.passed).count()
    }

    /// Get all failed required checks
    pub fn failed_required(&self) -> Vec<&CheckResult> {
        self.checks
            .iter()
            .filter(|c| c.required && !c.passed)
            .collect()
    }
}

/// Check if a command is available in PATH
pub fn check_command_available(command: &str) -> Option<String> {
    std::process::Command::new(command)
        .arg("--version")
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                // Try to extract version from output
                let combined = format!("{stdout}{stderr}");
                extract_version(&combined)
            } else {
                None
            }
        })
}

/// Extract version string from command output
fn extract_version(output: &str) -> Option<String> {
    // Try to find version patterns like "1.2.3" or "v1.2.3"
    let version_regex = regex::Regex::new(r"v?(\d+\.\d+(?:\.\d+)?(?:-\w+)?)").ok()?;
    version_regex
        .captures(output)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_string())
}

/// Check Zig compiler availability
pub fn check_zig() -> CheckResult {
    match check_command_available("zig") {
        Some(version) => CheckResult::pass("Zig compiler", Some(version), true),
        None => CheckResult::fail(
            "Zig compiler",
            "Zig compiler not found in PATH",
            Some("Install Zig from https://ziglang.org/download/ or use your package manager"),
            true,
        ),
    }
}

/// Check Git availability
pub fn check_git() -> CheckResult {
    match check_command_available("git") {
        Some(version) => CheckResult::pass("Git", Some(version), true),
        None => CheckResult::fail(
            "Git",
            "Git not found in PATH",
            Some("Install Git from https://git-scm.com/ or use your package manager"),
            true,
        ),
    }
}

/// Check UPX availability (optional, for compression)
pub fn check_upx() -> CheckResult {
    match check_command_available("upx") {
        Some(version) => CheckResult::pass("UPX (compression)", Some(version), false),
        None => CheckResult::fail(
            "UPX (compression)",
            "UPX not found in PATH",
            Some("Install UPX for binary compression: https://upx.github.io/ (optional)"),
            false,
        ),
    }
}

/// Check Docker/Podman availability (optional, for sandboxed builds)
pub fn check_container_runtime() -> CheckResult {
    // Try Docker first
    if let Some(version) = check_command_available("docker") {
        return CheckResult::pass("Container runtime (Docker)", Some(version), false);
    }
    // Try Podman as alternative
    if let Some(version) = check_command_available("podman") {
        return CheckResult::pass("Container runtime (Podman)", Some(version), false);
    }
    CheckResult::fail(
        "Container runtime",
        "Neither Docker nor Podman found in PATH",
        Some("Install Docker or Podman for sandboxed builds (optional)"),
        false,
    )
}

/// Check if project configuration is valid
pub fn check_project_config(project_dir: &Path) -> Vec<String> {
    let mut issues = Vec::new();
    let manifest_path = project_dir.join("zigroot.toml");

    if manifest_path.exists() {
        match std::fs::read_to_string(&manifest_path) {
            Ok(content) => {
                // Try to parse as TOML
                if let Err(e) = content.parse::<toml::Table>() {
                    issues.push(format!("Invalid manifest TOML: {e}"));
                } else {
                    // Check for common issues
                    if let Ok(table) = content.parse::<toml::Table>() {
                        // Check project section
                        if let Some(project) = table.get("project").and_then(|v| v.as_table()) {
                            if let Some(name) = project.get("name").and_then(|v| v.as_str()) {
                                if name.is_empty() {
                                    issues.push("Project name is empty".to_string());
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                issues.push(format!("Cannot read manifest: {e}"));
            }
        }
    }

    issues
}

/// Run all doctor checks
pub fn run_doctor(project_dir: Option<&Path>) -> DoctorReport {
    let mut report = DoctorReport::new();

    // Check required dependencies
    report.add_check(check_zig());
    report.add_check(check_git());

    // Check optional dependencies
    report.add_check(check_upx());
    report.add_check(check_container_runtime());

    // Check project configuration if in a project directory
    if let Some(dir) = project_dir {
        let config_issues = check_project_config(dir);
        for issue in config_issues {
            report.add_config_issue(issue);
        }
    }

    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_result_pass() {
        let result = CheckResult::pass("test", Some("1.0.0".to_string()), true);
        assert!(result.passed);
        assert_eq!(result.name, "test");
        assert_eq!(result.version, Some("1.0.0".to_string()));
        assert!(result.required);
    }

    #[test]
    fn test_check_result_fail() {
        let result = CheckResult::fail("test", "error", Some("suggestion"), false);
        assert!(!result.passed);
        assert_eq!(result.name, "test");
        assert_eq!(result.error, Some("error".to_string()));
        assert_eq!(result.suggestion, Some("suggestion".to_string()));
        assert!(!result.required);
    }

    #[test]
    fn test_doctor_report_counts() {
        let mut report = DoctorReport::new();
        report.add_check(CheckResult::pass("a", None, true));
        report.add_check(CheckResult::fail("b", "err", None, true));
        report.add_check(CheckResult::pass("c", None, false));

        assert_eq!(report.passed_count(), 2);
        assert_eq!(report.failed_count(), 1);
        assert!(!report.all_passed());
        assert!(!report.all_required_passed());
    }

    #[test]
    fn test_extract_version() {
        assert_eq!(extract_version("zig 0.11.0"), Some("0.11.0".to_string()));
        assert_eq!(extract_version("git version 2.39.0"), Some("2.39.0".to_string()));
        assert_eq!(extract_version("v1.2.3-beta"), Some("1.2.3-beta".to_string()));
    }
}
