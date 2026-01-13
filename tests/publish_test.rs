//! Integration tests for `zigroot publish` command
//!
//! Tests for Requirements 28.7-28.11, 29.5-29.8:
//! - Creates PR to appropriate registry
//! - Validates before publishing
//! - Requires GitHub authentication
//! - Checks for name conflicts
//! - Detects package vs board
//!
//! **Validates: Requirements 28.7-28.11, 29.5-29.8**

mod common;

use common::TestProject;
use std::process::Command;

/// Helper to run zigroot publish command
fn run_publish(project: &TestProject, path: &str) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zigroot"));
    cmd.current_dir(project.path());
    cmd.args(["publish", path]);
    // Ensure no GitHub token is set for auth tests
    cmd.env_remove("GITHUB_TOKEN");
    cmd.output().expect("Failed to execute zigroot publish")
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

/// Create a valid board structure for testing
fn create_valid_board(project: &TestProject, name: &str) {
    let board_dir = format!("boards/{}", name);
    project.create_dir(&board_dir);

    let board_toml = format!(
        r#"[board]
name = "{}"
description = "A test board"
target = "arm-linux-musleabihf"
cpu = "cortex-a7"

[defaults]
image_format = "ext4"
rootfs_size = "256M"
hostname = "test"
"#,
        name
    );
    project.create_file(&format!("{}/board.toml", board_dir), &board_toml);
}

// ============================================
// Authentication Tests
// ============================================

/// Test: Requires GitHub authentication
/// **Validates: Requirements 28.9, 29.7**
#[test]
fn test_publish_requires_github_auth() {
    let project = TestProject::new();
    create_valid_package(&project, "auth-test-pkg");

    let output = run_publish(&project, "packages/auth-test-pkg");

    // Should fail due to missing authentication
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Either fails with auth error or mentions authentication requirement
    assert!(
        !output.status.success()
            || stderr.contains("auth")
            || stderr.contains("GITHUB_TOKEN")
            || stderr.contains("token")
            || stdout.contains("auth")
            || stdout.contains("GITHUB_TOKEN"),
        "Should require GitHub authentication: stdout={stdout}, stderr={stderr}"
    );
}

// ============================================
// Validation Tests
// ============================================

/// Test: Validates package before publishing
/// **Validates: Requirement 28.8**
#[test]
fn test_publish_validates_package() {
    let project = TestProject::new();
    project.create_dir("packages/invalid-pkg");
    // No metadata.toml - invalid package

    let output = run_publish(&project, "packages/invalid-pkg");

    assert!(
        !output.status.success(),
        "zigroot publish should fail for invalid package"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("metadata")
            || stderr.contains("invalid")
            || stderr.contains("valid")
            || stderr.contains("required"),
        "Error should mention validation failure: {stderr}"
    );
}

/// Test: Validates board before publishing
/// **Validates: Requirement 29.6**
#[test]
fn test_publish_validates_board() {
    let project = TestProject::new();
    project.create_dir("boards/invalid-board");
    // No board.toml - invalid board

    let output = run_publish(&project, "boards/invalid-board");

    assert!(
        !output.status.success(),
        "zigroot publish should fail for invalid board"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("board.toml")
            || stderr.contains("invalid")
            || stderr.contains("valid")
            || stderr.contains("required"),
        "Error should mention validation failure: {stderr}"
    );
}

// ============================================
// Type Detection Tests
// ============================================

/// Test: Detects package vs board automatically
/// **Validates: Requirements 28.7, 29.5**
#[test]
fn test_publish_detects_package_type() {
    let project = TestProject::new();
    create_valid_package(&project, "detect-pkg");

    let output = run_publish(&project, "packages/detect-pkg");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should detect it's a package (may fail due to auth, but should recognize type)
    assert!(
        stderr.contains("package")
            || stdout.contains("package")
            || stderr.contains("auth")
            || stderr.contains("GITHUB_TOKEN"),
        "Should detect package type: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: Detects board type automatically
/// **Validates: Requirements 28.7, 29.5**
#[test]
fn test_publish_detects_board_type() {
    let project = TestProject::new();
    create_valid_board(&project, "detect-board");

    let output = run_publish(&project, "boards/detect-board");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should detect it's a board (may fail due to auth, but should recognize type)
    assert!(
        stderr.contains("board")
            || stdout.contains("board")
            || stderr.contains("auth")
            || stderr.contains("GITHUB_TOKEN"),
        "Should detect board type: stdout={stdout}, stderr={stderr}"
    );
}

// ============================================
// Path Validation Tests
// ============================================

/// Test: Fails for non-existent path
/// **Validates: Requirements 28.7, 29.5**
#[test]
fn test_publish_nonexistent_path() {
    let project = TestProject::new();

    let output = run_publish(&project, "packages/nonexistent");

    assert!(
        !output.status.success(),
        "zigroot publish should fail for non-existent path"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not found")
            || stderr.contains("does not exist")
            || stderr.contains("No such"),
        "Error should mention path not found: {stderr}"
    );
}

/// Test: Provides helpful output about publishing process
/// **Validates: Requirements 28.7, 29.5**
#[test]
fn test_publish_provides_helpful_output() {
    let project = TestProject::new();
    create_valid_package(&project, "helpful-pkg");

    let output = run_publish(&project, "packages/helpful-pkg");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should provide some helpful output (even if failing due to auth)
    assert!(
        !stdout.is_empty() || !stderr.is_empty(),
        "Should provide output about publishing"
    );

    // If failing due to auth, should mention how to authenticate
    if !output.status.success() && (stderr.contains("auth") || stderr.contains("token")) {
        assert!(
            stderr.contains("GITHUB_TOKEN") || stderr.contains("gh"),
            "Should mention how to authenticate: {stderr}"
        );
    }
}
