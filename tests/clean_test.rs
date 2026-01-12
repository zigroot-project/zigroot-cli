//! Integration tests for `zigroot clean` command
//!
//! Tests for Requirement 4.5:
//! - Removes build/ directory
//! - Removes output/ directory
//!
//! **Validates: Requirements 4.5**

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

/// Helper to run zigroot clean command
fn run_clean(project: &TestProject, args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zigroot"));
    cmd.current_dir(project.path());
    cmd.arg("clean");
    for arg in args {
        cmd.arg(arg);
    }
    cmd.output().expect("Failed to execute zigroot clean")
}

/// Helper to check if build directory exists
fn build_dir_exists(project: &TestProject) -> bool {
    project.path().join("build").is_dir()
}

/// Helper to check if output directory exists
fn output_dir_exists(project: &TestProject) -> bool {
    project.path().join("output").is_dir()
}

/// Helper to initialize a project for clean tests
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

/// Helper to create build artifacts in a project
fn create_build_artifacts(project: &TestProject) {
    // Create build directory with some content
    project.create_dir("build");
    project.create_file("build/stamps/busybox.stamp", "1234567890");
    project.create_file("build/logs/busybox.log", "Build log content");
    project.create_file("build/packages/busybox/bin/busybox", "binary content");

    // Create output directory with some content
    project.create_dir("output");
    project.create_file("output/rootfs.img", "rootfs image content");
    project.create_file("output/rootfs.manifest", "manifest content");
}

// ============================================
// Unit Tests for zigroot clean
// ============================================

/// Test: Removes build/ directory
/// **Validates: Requirement 4.5**
#[test]
fn test_clean_removes_build_directory() {
    let project = setup_project();

    // Create build artifacts
    create_build_artifacts(&project);

    // Verify build directory exists before clean
    assert!(
        build_dir_exists(&project),
        "build/ directory should exist before clean"
    );

    let output = run_clean(&project, &[]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Clean command should succeed
    assert!(
        output.status.success(),
        "zigroot clean should succeed: stdout={stdout}, stderr={stderr}"
    );

    // Build directory should be removed
    assert!(
        !build_dir_exists(&project),
        "build/ directory should be removed after clean"
    );
}

/// Test: Removes output/ directory
/// **Validates: Requirement 4.5**
#[test]
fn test_clean_removes_output_directory() {
    let project = setup_project();

    // Create build artifacts
    create_build_artifacts(&project);

    // Verify output directory exists before clean
    assert!(
        output_dir_exists(&project),
        "output/ directory should exist before clean"
    );

    let output = run_clean(&project, &[]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Clean command should succeed
    assert!(
        output.status.success(),
        "zigroot clean should succeed: stdout={stdout}, stderr={stderr}"
    );

    // Output directory should be removed
    assert!(
        !output_dir_exists(&project),
        "output/ directory should be removed after clean"
    );
}

/// Test: Clean removes both build/ and output/ directories
/// **Validates: Requirement 4.5**
#[test]
fn test_clean_removes_both_directories() {
    let project = setup_project();

    // Create build artifacts
    create_build_artifacts(&project);

    // Verify both directories exist before clean
    assert!(
        build_dir_exists(&project),
        "build/ directory should exist before clean"
    );
    assert!(
        output_dir_exists(&project),
        "output/ directory should exist before clean"
    );

    let output = run_clean(&project, &[]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Clean command should succeed
    assert!(
        output.status.success(),
        "zigroot clean should succeed: stdout={stdout}, stderr={stderr}"
    );

    // Both directories should be removed
    assert!(
        !build_dir_exists(&project),
        "build/ directory should be removed after clean"
    );
    assert!(
        !output_dir_exists(&project),
        "output/ directory should be removed after clean"
    );
}

/// Test: Clean succeeds when directories don't exist
/// **Validates: Requirement 4.5**
#[test]
fn test_clean_succeeds_when_no_artifacts() {
    let project = setup_project();

    // Don't create any build artifacts
    // Verify directories don't exist
    assert!(
        !build_dir_exists(&project),
        "build/ directory should not exist initially"
    );
    assert!(
        !output_dir_exists(&project),
        "output/ directory should not exist initially"
    );

    let output = run_clean(&project, &[]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Clean command should succeed even when nothing to clean
    assert!(
        output.status.success(),
        "zigroot clean should succeed even with no artifacts: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: Clean preserves other project files
/// **Validates: Requirement 4.5**
#[test]
fn test_clean_preserves_project_files() {
    let project = setup_project();

    // Create build artifacts
    create_build_artifacts(&project);

    // Create some additional project files that should be preserved
    project.create_file("packages/mypackage/package.toml", "package content");
    project.create_file("user/files/custom.conf", "custom config");

    let output = run_clean(&project, &[]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Clean command should succeed
    assert!(
        output.status.success(),
        "zigroot clean should succeed: stdout={stdout}, stderr={stderr}"
    );

    // Build artifacts should be removed
    assert!(
        !build_dir_exists(&project),
        "build/ directory should be removed"
    );
    assert!(
        !output_dir_exists(&project),
        "output/ directory should be removed"
    );

    // Project files should be preserved
    assert!(
        project.file_exists("zigroot.toml"),
        "zigroot.toml should be preserved"
    );
    assert!(
        project.file_exists("packages/mypackage/package.toml"),
        "packages/ content should be preserved"
    );
    assert!(
        project.file_exists("user/files/custom.conf"),
        "user/ content should be preserved"
    );
}

/// Test: Clean fails gracefully without manifest
/// **Validates: Requirement 4.5**
#[test]
fn test_clean_fails_without_manifest() {
    let project = TestProject::new();

    // Don't initialize - no manifest exists
    let output = run_clean(&project, &[]);

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should fail because no manifest exists
    assert!(
        !output.status.success(),
        "zigroot clean should fail without manifest"
    );

    assert!(
        stderr.contains("manifest")
            || stderr.contains("zigroot.toml")
            || stderr.contains("not found")
            || stderr.contains("initialize")
            || stderr.contains("init"),
        "Error should mention missing manifest: {stderr}"
    );
}

/// Test: Clean only removes build/ directory when output/ doesn't exist
/// **Validates: Requirement 4.5**
#[test]
fn test_clean_removes_only_build_when_no_output() {
    let project = setup_project();

    // Create only build directory
    project.create_dir("build");
    project.create_file("build/test.txt", "test content");

    // Verify only build exists
    assert!(build_dir_exists(&project), "build/ should exist");
    assert!(!output_dir_exists(&project), "output/ should not exist");

    let output = run_clean(&project, &[]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Clean command should succeed
    assert!(
        output.status.success(),
        "zigroot clean should succeed: stdout={stdout}, stderr={stderr}"
    );

    // Build directory should be removed
    assert!(
        !build_dir_exists(&project),
        "build/ directory should be removed"
    );
}

/// Test: Clean only removes output/ directory when build/ doesn't exist
/// **Validates: Requirement 4.5**
#[test]
fn test_clean_removes_only_output_when_no_build() {
    let project = setup_project();

    // Create only output directory
    project.create_dir("output");
    project.create_file("output/rootfs.img", "image content");

    // Verify only output exists
    assert!(!build_dir_exists(&project), "build/ should not exist");
    assert!(output_dir_exists(&project), "output/ should exist");

    let output = run_clean(&project, &[]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Clean command should succeed
    assert!(
        output.status.success(),
        "zigroot clean should succeed: stdout={stdout}, stderr={stderr}"
    );

    // Output directory should be removed
    assert!(
        !output_dir_exists(&project),
        "output/ directory should be removed"
    );
}
