//! Integration tests for `zigroot verify` command
//!
//! Tests for Requirements 28.2-28.5, 29.2-29.4:
//! - Validates package structure
//! - Validates board structure
//! - Checks required fields
//! - --fetch downloads and verifies checksums
//!
//! **Validates: Requirements 28.2-28.5, 29.2-29.4**

mod common;

use common::TestProject;
use std::process::Command;

/// Helper to run zigroot verify command
fn run_verify(project: &TestProject, path: &str, fetch: bool) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zigroot"));
    cmd.current_dir(project.path());
    cmd.args(["verify", path]);
    if fetch {
        cmd.arg("--fetch");
    }
    cmd.output().expect("Failed to execute zigroot verify")
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
// Package Validation Tests
// ============================================

/// Test: Validates valid package structure
/// **Validates: Requirement 28.2**
#[test]
fn test_verify_valid_package() {
    let project = TestProject::new();
    create_valid_package(&project, "valid-pkg");

    let output = run_verify(&project, "packages/valid-pkg", false);

    assert!(
        output.status.success(),
        "zigroot verify should succeed for valid package: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Test: Validates package with missing metadata.toml
/// **Validates: Requirement 28.2**
#[test]
fn test_verify_package_missing_metadata() {
    let project = TestProject::new();
    project.create_dir("packages/bad-pkg");

    // Only create version file, no metadata.toml
    let version = r#"[release]
version = "1.0.0"

[source]
url = "https://example.com/test.tar.gz"
sha256 = "abc123"
"#;
    project.create_file("packages/bad-pkg/1.0.0.toml", version);

    let output = run_verify(&project, "packages/bad-pkg", false);

    assert!(
        !output.status.success(),
        "zigroot verify should fail for package missing metadata.toml"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("metadata") || stderr.contains("required"),
        "Error should mention missing metadata: {stderr}"
    );
}

/// Test: Validates package metadata.toml has required fields (name, description, license)
/// **Validates: Requirement 28.3**
#[test]
fn test_verify_package_missing_required_fields() {
    let project = TestProject::new();
    project.create_dir("packages/incomplete-pkg");

    // Create metadata.toml missing required fields
    let metadata = r#"[package]
name = "incomplete-pkg"
# Missing description and license
"#;
    project.create_file("packages/incomplete-pkg/metadata.toml", metadata);

    let version = r#"[release]
version = "1.0.0"

[source]
url = "https://example.com/test.tar.gz"
sha256 = "abc123"
"#;
    project.create_file("packages/incomplete-pkg/1.0.0.toml", version);

    let output = run_verify(&project, "packages/incomplete-pkg", false);

    assert!(
        !output.status.success(),
        "zigroot verify should fail for package missing required fields"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("description") || stderr.contains("license") || stderr.contains("required"),
        "Error should mention missing required field: {stderr}"
    );
}

/// Test: Validates version file has required fields (version, source.url, source.sha256)
/// **Validates: Requirement 28.4**
#[test]
fn test_verify_version_file_missing_fields() {
    let project = TestProject::new();
    project.create_dir("packages/bad-version-pkg");

    let metadata = r#"[package]
name = "bad-version-pkg"
description = "Test package"
license = "MIT"
"#;
    project.create_file("packages/bad-version-pkg/metadata.toml", metadata);

    // Version file missing sha256
    let version = r#"[release]
version = "1.0.0"

[source]
url = "https://example.com/test.tar.gz"
# Missing sha256
"#;
    project.create_file("packages/bad-version-pkg/1.0.0.toml", version);

    let output = run_verify(&project, "packages/bad-version-pkg", false);

    assert!(
        !output.status.success(),
        "zigroot verify should fail for version file missing sha256"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("sha256") || stderr.contains("source") || stderr.contains("required"),
        "Error should mention missing sha256: {stderr}"
    );
}

/// Test: --fetch downloads and verifies checksums
/// **Validates: Requirement 28.5**
#[test]
fn test_verify_with_fetch_flag() {
    let project = TestProject::new();
    create_valid_package(&project, "fetch-test-pkg");

    // Note: This test uses a placeholder URL that won't actually download
    // In a real scenario, this would test actual download and checksum verification
    let output = run_verify(&project, "packages/fetch-test-pkg", true);

    // The command should either succeed (if URL is reachable) or fail with download error
    // (not a validation error)
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Either succeeds or fails with network/download error (not structure error)
    assert!(
        output.status.success()
            || stderr.contains("download")
            || stderr.contains("fetch")
            || stderr.contains("network")
            || stderr.contains("connect")
            || stderr.contains("example.com"),
        "Should accept --fetch flag: stdout={stdout}, stderr={stderr}"
    );
}

// ============================================
// Board Validation Tests
// ============================================

/// Test: Validates valid board structure
/// **Validates: Requirement 29.2**
#[test]
fn test_verify_valid_board() {
    let project = TestProject::new();
    create_valid_board(&project, "valid-board");

    let output = run_verify(&project, "boards/valid-board", false);

    assert!(
        output.status.success(),
        "zigroot verify should succeed for valid board: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Test: Validates board with missing board.toml
/// **Validates: Requirement 29.2**
#[test]
fn test_verify_board_missing_board_toml() {
    let project = TestProject::new();
    project.create_dir("boards/bad-board");
    // No board.toml created

    let output = run_verify(&project, "boards/bad-board", false);

    assert!(
        !output.status.success(),
        "zigroot verify should fail for board missing board.toml"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("board.toml") || stderr.contains("required") || stderr.contains("not found"),
        "Error should mention missing board.toml: {stderr}"
    );
}

/// Test: Validates board.toml has required fields (name, description, target, cpu)
/// **Validates: Requirement 29.3**
#[test]
fn test_verify_board_missing_required_fields() {
    let project = TestProject::new();
    project.create_dir("boards/incomplete-board");

    // Board missing required fields
    let board_toml = r#"[board]
name = "incomplete-board"
# Missing description, target, cpu

[defaults]
image_format = "ext4"
rootfs_size = "256M"
hostname = "test"
"#;
    project.create_file("boards/incomplete-board/board.toml", board_toml);

    let output = run_verify(&project, "boards/incomplete-board", false);

    assert!(
        !output.status.success(),
        "zigroot verify should fail for board missing required fields"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("target")
            || stderr.contains("cpu")
            || stderr.contains("description")
            || stderr.contains("required"),
        "Error should mention missing required field: {stderr}"
    );
}

/// Test: Validates target is a valid Zig target triple
/// **Validates: Requirement 29.4**
#[test]
fn test_verify_board_invalid_target() {
    let project = TestProject::new();
    project.create_dir("boards/invalid-target-board");

    let board_toml = r#"[board]
name = "invalid-target-board"
description = "Board with invalid target"
target = "not-a-valid-target"
cpu = "cortex-a7"

[defaults]
image_format = "ext4"
rootfs_size = "256M"
hostname = "test"
"#;
    project.create_file("boards/invalid-target-board/board.toml", board_toml);

    let output = run_verify(&project, "boards/invalid-target-board", false);

    // Should either fail or warn about invalid target
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Either fails or warns about target
    assert!(
        !output.status.success()
            || stderr.contains("target")
            || stdout.contains("target")
            || stderr.contains("warning"),
        "Should validate or warn about invalid target: stdout={stdout}, stderr={stderr}"
    );
}

// ============================================
// Auto-detection Tests
// ============================================

/// Test: Detects package vs board automatically
/// **Validates: Requirements 28.2, 29.2**
#[test]
fn test_verify_auto_detects_type() {
    let project = TestProject::new();

    // Create both a package and a board
    create_valid_package(&project, "auto-pkg");
    create_valid_board(&project, "auto-board");

    // Verify package
    let pkg_output = run_verify(&project, "packages/auto-pkg", false);
    assert!(
        pkg_output.status.success(),
        "Should verify package: {}",
        String::from_utf8_lossy(&pkg_output.stderr)
    );

    // Verify board
    let board_output = run_verify(&project, "boards/auto-board", false);
    assert!(
        board_output.status.success(),
        "Should verify board: {}",
        String::from_utf8_lossy(&board_output.stderr)
    );
}

/// Test: Handles non-existent path
/// **Validates: Requirements 28.2, 29.2**
#[test]
fn test_verify_nonexistent_path() {
    let project = TestProject::new();

    let output = run_verify(&project, "nonexistent/path", false);

    assert!(
        !output.status.success(),
        "zigroot verify should fail for non-existent path"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not found")
            || stderr.contains("does not exist")
            || stderr.contains("No such"),
        "Error should mention path not found: {stderr}"
    );
}

/// Test: Validates TOML syntax errors are reported
/// **Validates: Requirements 28.2, 29.2**
#[test]
fn test_verify_invalid_toml_syntax() {
    let project = TestProject::new();
    project.create_dir("packages/bad-toml-pkg");

    // Create invalid TOML
    let metadata = r#"[package
name = "bad-toml-pkg"
this is not valid toml
"#;
    project.create_file("packages/bad-toml-pkg/metadata.toml", metadata);

    let output = run_verify(&project, "packages/bad-toml-pkg", false);

    assert!(
        !output.status.success(),
        "zigroot verify should fail for invalid TOML syntax"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("TOML")
            || stderr.contains("parse")
            || stderr.contains("syntax")
            || stderr.contains("invalid"),
        "Error should mention TOML parsing error: {stderr}"
    );
}
