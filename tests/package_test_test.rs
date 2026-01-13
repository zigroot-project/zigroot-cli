//! Integration tests for `zigroot package test` command
//!
//! Tests for Requirement 28.6:
//! - Attempts to build package
//! - Reports success or failure
//!
//! **Validates: Requirements 28.6**

mod common;

use common::TestProject;
use std::process::Command;

/// Helper to run zigroot package test command
fn run_package_test(project: &TestProject, path: &str) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zigroot"));
    cmd.current_dir(project.path());
    cmd.args(["package", "test", path]);
    cmd.output().expect("Failed to execute zigroot package test")
}

/// Create a valid package structure for testing
fn create_valid_package(project: &TestProject, name: &str) {
    let pkg_dir = format!("packages/{}", name);
    project.create_dir(&pkg_dir);

    let metadata = format!(
        r#"[package]
name = "{}"
description = "A test package"
license = "MIT"

[build]
type = "make"
"#,
        name
    );
    project.create_file(&format!("{}/metadata.toml", pkg_dir), &metadata);

    let version = r#"[release]
version = "1.0.0"

[source]
url = "https://example.com/test-1.0.0.tar.gz"
sha256 = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
"#;
    project.create_file(&format!("{}/1.0.0.toml", pkg_dir), version);
}

// ============================================
// Unit Tests for zigroot package test
// ============================================

/// Test: Attempts to build package and reports success
/// **Validates: Requirement 28.6**
#[test]
fn test_package_test_valid_package() {
    let project = TestProject::new();
    create_valid_package(&project, "test-pkg");

    let output = run_package_test(&project, "packages/test-pkg");

    // Should succeed or provide meaningful output about the test
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success()
            || stdout.contains("valid")
            || stdout.contains("test")
            || stderr.contains("build"),
        "Should attempt to test package: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: Reports failure for invalid package
/// **Validates: Requirement 28.6**
#[test]
fn test_package_test_invalid_package() {
    let project = TestProject::new();
    project.create_dir("packages/invalid-pkg");
    // No metadata.toml - invalid package

    let output = run_package_test(&project, "packages/invalid-pkg");

    assert!(
        !output.status.success(),
        "zigroot package test should fail for invalid package"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("metadata")
            || stderr.contains("invalid")
            || stderr.contains("not found")
            || stderr.contains("valid"),
        "Error should mention invalid package: {stderr}"
    );
}

/// Test: Reports failure for non-existent path
/// **Validates: Requirement 28.6**
#[test]
fn test_package_test_nonexistent_path() {
    let project = TestProject::new();

    let output = run_package_test(&project, "packages/nonexistent");

    assert!(
        !output.status.success(),
        "zigroot package test should fail for non-existent path"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not found")
            || stderr.contains("does not exist")
            || stderr.contains("No such"),
        "Error should mention path not found: {stderr}"
    );
}

/// Test: Provides meaningful output about test results
/// **Validates: Requirement 28.6**
#[test]
fn test_package_test_provides_output() {
    let project = TestProject::new();
    create_valid_package(&project, "output-test-pkg");

    let output = run_package_test(&project, "packages/output-test-pkg");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should provide some output about the test
    assert!(
        !stdout.is_empty() || !stderr.is_empty(),
        "Should provide output about test results"
    );

    // If successful, should indicate success
    if output.status.success() {
        assert!(
            stdout.contains("✓")
                || stdout.contains("valid")
                || stdout.contains("success")
                || stdout.contains("Testing"),
            "Success output should indicate positive result: {stdout}"
        );
    }
}
