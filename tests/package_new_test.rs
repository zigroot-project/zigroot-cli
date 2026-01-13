//! Integration tests for `zigroot package new` command
//!
//! Tests for Requirement 28.1:
//! - Creates package template in packages/<name>/
//! - Creates metadata.toml and version file
//!
//! **Validates: Requirements 28.1**

mod common;

use common::TestProject;
use std::process::Command;

/// Helper to run zigroot package new command
fn run_package_new(project: &TestProject, name: &str) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zigroot"));
    cmd.current_dir(project.path());
    cmd.args(["package", "new", name]);
    cmd.output().expect("Failed to execute zigroot package new")
}

/// Helper to check if package template was created correctly
fn has_valid_package_template(project: &TestProject, name: &str) -> bool {
    let pkg_dir = project.path().join("packages").join(name);
    let metadata_path = pkg_dir.join("metadata.toml");
    let version_path = pkg_dir.join("1.0.0.toml");

    pkg_dir.is_dir() && metadata_path.exists() && version_path.exists()
}

/// Helper to check if metadata.toml has required fields
fn has_valid_metadata(project: &TestProject, name: &str) -> bool {
    let metadata_path = project
        .path()
        .join("packages")
        .join(name)
        .join("metadata.toml");

    if !metadata_path.exists() {
        return false;
    }

    let content = std::fs::read_to_string(&metadata_path).unwrap_or_default();

    // Check for required fields per Requirement 28.3
    content.contains("[package]")
        && content.contains("name")
        && content.contains("description")
        && content.contains("license")
}

/// Helper to check if version file has required fields
fn has_valid_version_file(project: &TestProject, name: &str) -> bool {
    let version_path = project
        .path()
        .join("packages")
        .join(name)
        .join("1.0.0.toml");

    if !version_path.exists() {
        return false;
    }

    let content = std::fs::read_to_string(&version_path).unwrap_or_default();

    // Check for required fields per Requirement 28.4
    content.contains("version")
        && content.contains("[source]")
        && content.contains("url")
        && content.contains("sha256")
}

// ============================================
// Unit Tests for zigroot package new
// ============================================

/// Test: Creates package template in packages/<name>/
/// **Validates: Requirement 28.1**
#[test]
fn test_package_new_creates_template_directory() {
    let project = TestProject::new();

    // Initialize project first
    let init_output = Command::new(env!("CARGO_BIN_EXE_zigroot"))
        .current_dir(project.path())
        .arg("init")
        .output()
        .expect("Failed to run init");
    assert!(init_output.status.success(), "Init should succeed");

    let output = run_package_new(&project, "my-package");

    assert!(
        output.status.success(),
        "zigroot package new should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify package directory was created
    let pkg_dir = project.path().join("packages").join("my-package");
    assert!(
        pkg_dir.is_dir(),
        "packages/my-package/ directory should be created"
    );
}

/// Test: Creates metadata.toml file
/// **Validates: Requirement 28.1**
#[test]
fn test_package_new_creates_metadata_toml() {
    let project = TestProject::new();

    // Initialize project first
    let init_output = Command::new(env!("CARGO_BIN_EXE_zigroot"))
        .current_dir(project.path())
        .arg("init")
        .output()
        .expect("Failed to run init");
    assert!(init_output.status.success(), "Init should succeed");

    let output = run_package_new(&project, "test-pkg");

    assert!(
        output.status.success(),
        "zigroot package new should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify metadata.toml was created
    let metadata_path = project
        .path()
        .join("packages")
        .join("test-pkg")
        .join("metadata.toml");
    assert!(
        metadata_path.exists(),
        "metadata.toml should be created in packages/test-pkg/"
    );

    // Verify metadata.toml has required fields
    assert!(
        has_valid_metadata(&project, "test-pkg"),
        "metadata.toml should contain required fields (name, description, license)"
    );
}

/// Test: Creates version file (1.0.0.toml)
/// **Validates: Requirement 28.1**
#[test]
fn test_package_new_creates_version_file() {
    let project = TestProject::new();

    // Initialize project first
    let init_output = Command::new(env!("CARGO_BIN_EXE_zigroot"))
        .current_dir(project.path())
        .arg("init")
        .output()
        .expect("Failed to run init");
    assert!(init_output.status.success(), "Init should succeed");

    let output = run_package_new(&project, "new-pkg");

    assert!(
        output.status.success(),
        "zigroot package new should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify version file was created
    let version_path = project
        .path()
        .join("packages")
        .join("new-pkg")
        .join("1.0.0.toml");
    assert!(
        version_path.exists(),
        "1.0.0.toml should be created in packages/new-pkg/"
    );

    // Verify version file has required fields
    assert!(
        has_valid_version_file(&project, "new-pkg"),
        "1.0.0.toml should contain required fields (version, source.url, source.sha256)"
    );
}

/// Test: Package name is used in generated files
/// **Validates: Requirement 28.1**
#[test]
fn test_package_new_uses_package_name() {
    let project = TestProject::new();

    // Initialize project first
    let init_output = Command::new(env!("CARGO_BIN_EXE_zigroot"))
        .current_dir(project.path())
        .arg("init")
        .output()
        .expect("Failed to run init");
    assert!(init_output.status.success(), "Init should succeed");

    let output = run_package_new(&project, "custom-name");

    assert!(
        output.status.success(),
        "zigroot package new should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify package name is in metadata.toml
    let metadata_content = project.read_file("packages/custom-name/metadata.toml");
    assert!(
        metadata_content.contains("custom-name"),
        "metadata.toml should contain the package name"
    );
}

/// Test: Complete package template is valid
/// **Validates: Requirement 28.1**
#[test]
fn test_package_new_creates_complete_template() {
    let project = TestProject::new();

    // Initialize project first
    let init_output = Command::new(env!("CARGO_BIN_EXE_zigroot"))
        .current_dir(project.path())
        .arg("init")
        .output()
        .expect("Failed to run init");
    assert!(init_output.status.success(), "Init should succeed");

    let output = run_package_new(&project, "complete-pkg");

    assert!(
        output.status.success(),
        "zigroot package new should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify complete template
    assert!(
        has_valid_package_template(&project, "complete-pkg"),
        "Package template should have all required files"
    );
}

/// Test: Fails gracefully when package already exists
/// **Validates: Requirement 28.1**
#[test]
fn test_package_new_fails_if_exists() {
    let project = TestProject::new();

    // Initialize project first
    let init_output = Command::new(env!("CARGO_BIN_EXE_zigroot"))
        .current_dir(project.path())
        .arg("init")
        .output()
        .expect("Failed to run init");
    assert!(init_output.status.success(), "Init should succeed");

    // Create package first time
    let output1 = run_package_new(&project, "existing-pkg");
    assert!(output1.status.success(), "First creation should succeed");

    // Try to create same package again
    let output2 = run_package_new(&project, "existing-pkg");

    // Should fail or warn about existing package
    let stderr = String::from_utf8_lossy(&output2.stderr);
    let stdout = String::from_utf8_lossy(&output2.stdout);

    assert!(
        !output2.status.success()
            || stderr.contains("exists")
            || stderr.contains("already")
            || stdout.contains("exists")
            || stdout.contains("already"),
        "Should fail or warn when package already exists: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: Works without initialized project (creates packages/ if needed)
/// **Validates: Requirement 28.1**
#[test]
fn test_package_new_without_init() {
    let project = TestProject::new();

    // Don't initialize - just run package new directly
    let output = run_package_new(&project, "standalone-pkg");

    // Should either succeed (creating packages/ dir) or fail with helpful message
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Either succeeds or gives helpful error
    assert!(
        output.status.success()
            || stderr.contains("init")
            || stderr.contains("zigroot.toml")
            || stderr.contains("project"),
        "Should succeed or give helpful error: stdout={stdout}, stderr={stderr}"
    );
}
