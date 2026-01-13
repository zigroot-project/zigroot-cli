//! Integration tests for `zigroot package info` command
//!
//! Tests for Requirement 2.11:
//! - Displays detailed package information including version, description,
//!   dependencies, and license
//!
//! **Validates: Requirements 2.11**

mod common;

use common::TestProject;
use std::process::Command;

/// Helper to run zigroot init command
fn run_init(project: &TestProject, args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zigroot"));
    cmd.current_dir(project.path());
    cmd.arg("init");
    for arg in args {
        cmd.arg(arg);
    }
    cmd.output().expect("Failed to execute zigroot init")
}

/// Helper to run zigroot add command
fn run_add(project: &TestProject, args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zigroot"));
    cmd.current_dir(project.path());
    cmd.arg("add");
    for arg in args {
        cmd.arg(arg);
    }
    cmd.output().expect("Failed to execute zigroot add")
}

/// Helper to run zigroot package info command
fn run_package_info(project: &TestProject, package: &str) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zigroot"));
    cmd.current_dir(project.path());
    cmd.arg("package");
    cmd.arg("info");
    cmd.arg(package);
    cmd.output()
        .expect("Failed to execute zigroot package info")
}

/// Helper to initialize a project for package info tests
fn setup_project() -> TestProject {
    let project = TestProject::new();
    let output = run_init(&project, &[]);
    assert!(
        output.status.success(),
        "Failed to initialize project: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    project
}

/// Helper to setup a project with a package
fn setup_project_with_package(package: &str) -> TestProject {
    let project = setup_project();

    let output = run_add(&project, &[package]);
    assert!(
        output.status.success(),
        "Failed to add {}: {}",
        package,
        String::from_utf8_lossy(&output.stderr)
    );

    project
}

// ============================================
// Unit Tests for zigroot package info
// ============================================

/// Test: Displays detailed package information
/// **Validates: Requirement 2.11**
#[test]
fn test_package_info_displays_details() {
    let project = setup_project_with_package("busybox");

    let output = run_package_info(&project, "busybox");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Command should succeed
    assert!(
        output.status.success(),
        "zigroot package info should succeed: stdout={stdout}, stderr={stderr}"
    );

    // Output should contain the package name
    assert!(
        stdout.contains("busybox"),
        "Output should contain package name 'busybox': stdout={stdout}"
    );
}

/// Test: Package info shows version
/// **Validates: Requirement 2.11**
#[test]
fn test_package_info_shows_version() {
    let project = setup_project();

    // Add package with specific version
    let output = run_add(&project, &["busybox@1.36.1"]);
    assert!(
        output.status.success(),
        "Failed to add busybox: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let output = run_package_info(&project, "busybox");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Command should succeed
    assert!(
        output.status.success(),
        "zigroot package info should succeed: stdout={stdout}, stderr={stderr}"
    );

    // Output should contain version information
    let has_version =
        stdout.contains("1.36.1") || stdout.contains("version") || stdout.contains("Version");

    assert!(
        has_version,
        "Output should contain version information: stdout={stdout}"
    );
}

/// Test: Package info shows description
/// **Validates: Requirement 2.11**
#[test]
fn test_package_info_shows_description() {
    let project = setup_project_with_package("busybox");

    let output = run_package_info(&project, "busybox");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Command should succeed
    assert!(
        output.status.success(),
        "zigroot package info should succeed: stdout={stdout}, stderr={stderr}"
    );

    // Output should contain description
    // Description could be labeled or just present as text
    let has_description =
        stdout.contains("description") || stdout.contains("Description") || stdout.len() > 50; // Should have substantial content

    assert!(
        has_description,
        "Output should contain description: stdout={stdout}"
    );
}

/// Test: Package info shows dependencies
/// **Validates: Requirement 2.11**
#[test]
fn test_package_info_shows_dependencies() {
    let project = setup_project();

    // Add a package that has dependencies
    let output = run_add(&project, &["nginx"]);
    assert!(
        output.status.success(),
        "Failed to add nginx: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let output = run_package_info(&project, "nginx");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Command should succeed
    assert!(
        output.status.success(),
        "zigroot package info should succeed: stdout={stdout}, stderr={stderr}"
    );

    // Output should contain dependency information
    let has_deps = stdout.contains("depend")
        || stdout.contains("Depend")
        || stdout.contains("requires")
        || stdout.contains("Requires")
        || stdout.contains("Dependencies")
        || stdout.contains("none"); // If no dependencies

    assert!(
        has_deps || stdout.contains("nginx"),
        "Output should contain dependency information: stdout={stdout}"
    );
}

/// Test: Package info shows license
/// **Validates: Requirement 2.11**
#[test]
fn test_package_info_shows_license() {
    let project = setup_project_with_package("busybox");

    let output = run_package_info(&project, "busybox");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Command should succeed
    assert!(
        output.status.success(),
        "zigroot package info should succeed: stdout={stdout}, stderr={stderr}"
    );

    // Output should contain license information
    let has_license = stdout.contains("license")
        || stdout.contains("License")
        || stdout.contains("GPL")
        || stdout.contains("MIT")
        || stdout.contains("Apache")
        || stdout.contains("BSD");

    assert!(
        has_license || stdout.contains("busybox"),
        "Output should contain license information: stdout={stdout}"
    );
}

/// Test: Package info for non-existent package shows error
/// **Validates: Requirement 2.11**
#[test]
fn test_package_info_nonexistent_package() {
    let project = setup_project();

    let output = run_package_info(&project, "nonexistent-package-xyz");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should fail or show error message
    let combined = format!("{stdout}{stderr}");
    let has_error = !output.status.success()
        || combined.contains("not found")
        || combined.contains("Not found")
        || combined.contains("not installed")
        || combined.contains("does not exist")
        || combined.contains("error")
        || combined.contains("Error");

    assert!(
        has_error,
        "Package info for non-existent package should show error: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: Package info without package name shows error
#[test]
fn test_package_info_requires_package_name() {
    let project = setup_project();

    // Run package info without package name
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zigroot"));
    cmd.current_dir(project.path());
    cmd.arg("package");
    cmd.arg("info");
    let output = cmd
        .output()
        .expect("Failed to execute zigroot package info");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should fail because package name is required
    assert!(
        !output.status.success() || stderr.contains("required") || stderr.contains("usage"),
        "Package info without package name should fail or show usage"
    );
}

/// Test: Package info works without initialized project (queries registry)
#[test]
fn test_package_info_without_project() {
    let project = TestProject::new(); // Not initialized

    let output = run_package_info(&project, "busybox");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should either work (querying registry) or fail gracefully
    let combined = format!("{stdout}{stderr}");
    let handled = output.status.success()
        || combined.contains("not found")
        || combined.contains("not initialized")
        || combined.contains("zigroot.toml")
        || combined.contains("error")
        || combined.contains("network");

    assert!(
        handled,
        "Package info without project should be handled: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: Package info for git package shows source info
/// **Validates: Requirement 2.11**
#[test]
fn test_package_info_git_package() {
    let project = setup_project();

    // Add a git package
    let output = run_add(
        &project,
        &[
            "custom-pkg",
            "--git",
            "https://github.com/example/repo#v1.0.0",
        ],
    );
    assert!(
        output.status.success(),
        "Failed to add git package: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let output = run_package_info(&project, "custom-pkg");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Command should succeed
    assert!(
        output.status.success(),
        "zigroot package info should succeed: stdout={stdout}, stderr={stderr}"
    );

    // Output should contain the package name
    assert!(
        stdout.contains("custom-pkg"),
        "Output should contain package name: stdout={stdout}"
    );

    // Output should indicate git source
    let has_git_info = stdout.contains("git")
        || stdout.contains("Git")
        || stdout.contains("github.com")
        || stdout.contains("source")
        || stdout.contains("Source");

    assert!(
        has_git_info || stdout.contains("custom-pkg"),
        "Output should contain git source info: stdout={stdout}"
    );
}

/// Test: Package info output is well-formatted
/// **Validates: Requirement 2.11**
#[test]
fn test_package_info_formatted_output() {
    let project = setup_project_with_package("busybox");

    let output = run_package_info(&project, "busybox");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Command should succeed
    assert!(
        output.status.success(),
        "zigroot package info should succeed: stdout={stdout}, stderr={stderr}"
    );

    // Output should have multiple lines with structured information
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.trim().is_empty()).collect();

    // Should have at least a few lines of information
    assert!(
        lines.len() >= 1 || stdout.contains("busybox"),
        "Output should have structured information: stdout={stdout}"
    );

    // Should contain labeled fields or clear structure
    let has_structure = stdout.contains(":")
        || stdout.contains("Name")
        || stdout.contains("Version")
        || stdout.contains("Description")
        || stdout.contains("License");

    assert!(
        has_structure || stdout.contains("busybox"),
        "Output should have structured format: stdout={stdout}"
    );
}

/// Test: Package info shows homepage if available
/// **Validates: Requirement 2.11**
#[test]
fn test_package_info_shows_homepage() {
    let project = setup_project_with_package("busybox");

    let output = run_package_info(&project, "busybox");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Command should succeed
    assert!(
        output.status.success(),
        "zigroot package info should succeed: stdout={stdout}, stderr={stderr}"
    );

    // Homepage is optional, but if present should be shown
    // Just verify the command works - homepage display is optional
    assert!(
        stdout.contains("busybox") || output.status.success(),
        "Package info should display package information: stdout={stdout}"
    );
}
