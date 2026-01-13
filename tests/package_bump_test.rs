//! Integration tests for `zigroot package bump` command
//!
//! Tests for Requirement 28.12:
//! - Creates new version file from latest
//!
//! **Validates: Requirements 28.12**

mod common;

use common::TestProject;
use std::process::Command;

/// Helper to run zigroot package bump command
fn run_package_bump(project: &TestProject, path: &str, version: &str) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zigroot"));
    cmd.current_dir(project.path());
    cmd.args(["package", "bump", path, version]);
    cmd.output()
        .expect("Failed to execute zigroot package bump")
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
// Unit Tests for zigroot package bump
// ============================================

/// Test: Creates new version file from latest
/// **Validates: Requirement 28.12**
#[test]
fn test_package_bump_creates_new_version() {
    let project = TestProject::new();
    create_valid_package(&project, "bump-pkg");

    let output = run_package_bump(&project, "packages/bump-pkg", "1.1.0");

    assert!(
        output.status.success(),
        "zigroot package bump should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify new version file was created
    let new_version_path = project.path().join("packages/bump-pkg/1.1.0.toml");
    assert!(
        new_version_path.exists(),
        "New version file 1.1.0.toml should be created"
    );
}

/// Test: New version file contains updated version
/// **Validates: Requirement 28.12**
#[test]
fn test_package_bump_updates_version_field() {
    let project = TestProject::new();
    create_valid_package(&project, "version-update-pkg");

    let output = run_package_bump(&project, "packages/version-update-pkg", "2.0.0");

    assert!(
        output.status.success(),
        "zigroot package bump should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Read new version file and check version field
    let new_version_content = project.read_file("packages/version-update-pkg/2.0.0.toml");
    assert!(
        new_version_content.contains("2.0.0"),
        "New version file should contain version 2.0.0"
    );
}

/// Test: Fails for non-existent package
/// **Validates: Requirement 28.12**
#[test]
fn test_package_bump_nonexistent_package() {
    let project = TestProject::new();

    let output = run_package_bump(&project, "packages/nonexistent", "1.1.0");

    assert!(
        !output.status.success(),
        "zigroot package bump should fail for non-existent package"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not found")
            || stderr.contains("does not exist")
            || stderr.contains("No such"),
        "Error should mention path not found: {stderr}"
    );
}

/// Test: Fails if version already exists
/// **Validates: Requirement 28.12**
#[test]
fn test_package_bump_version_exists() {
    let project = TestProject::new();
    create_valid_package(&project, "existing-version-pkg");

    // Try to bump to existing version
    let output = run_package_bump(&project, "packages/existing-version-pkg", "1.0.0");

    assert!(
        !output.status.success(),
        "zigroot package bump should fail if version already exists"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("exists") || stderr.contains("already"),
        "Error should mention version already exists: {stderr}"
    );
}

/// Test: Preserves source structure from latest version
/// **Validates: Requirement 28.12**
#[test]
fn test_package_bump_preserves_structure() {
    let project = TestProject::new();
    create_valid_package(&project, "structure-pkg");

    let output = run_package_bump(&project, "packages/structure-pkg", "1.2.0");

    assert!(
        output.status.success(),
        "zigroot package bump should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Read new version file and check it has source section
    let new_version_content = project.read_file("packages/structure-pkg/1.2.0.toml");
    assert!(
        new_version_content.contains("[source]"),
        "New version file should preserve [source] section"
    );
    assert!(
        new_version_content.contains("url") && new_version_content.contains("sha256"),
        "New version file should preserve url and sha256 fields"
    );
}

/// Test: Provides helpful output
/// **Validates: Requirement 28.12**
#[test]
fn test_package_bump_provides_output() {
    let project = TestProject::new();
    create_valid_package(&project, "output-pkg");

    let output = run_package_bump(&project, "packages/output-pkg", "1.3.0");

    assert!(
        output.status.success(),
        "zigroot package bump should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("1.3.0") || stdout.contains("Created") || stdout.contains("✓"),
        "Should provide helpful output about created version: {stdout}"
    );
}
