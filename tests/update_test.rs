//! Integration tests for `zigroot update` command
//!
//! Tests for Requirements 2.6, 2.7:
//! - Checks for newer versions of all packages
//! - Updates lock file with new versions
//! - Updates single package when name specified
//!
//! **Validates: Requirements 2.6, 2.7**

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

/// Helper to run zigroot update command
fn run_update(project: &TestProject, args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zigroot"));
    cmd.current_dir(project.path());
    cmd.arg("update");
    for arg in args {
        cmd.arg(arg);
    }
    cmd.output().expect("Failed to execute zigroot update")
}

/// Helper to check if manifest is valid TOML
fn is_valid_manifest(project: &TestProject) -> bool {
    let manifest_path = project.path().join("zigroot.toml");
    if !manifest_path.exists() {
        return false;
    }
    let content = std::fs::read_to_string(&manifest_path).unwrap_or_default();
    toml::from_str::<toml::Value>(&content).is_ok()
}

/// Helper to check if a package is in the manifest
fn manifest_has_package(project: &TestProject, package_name: &str) -> bool {
    let manifest_path = project.path().join("zigroot.toml");
    if !manifest_path.exists() {
        return false;
    }
    let content = std::fs::read_to_string(&manifest_path).unwrap_or_default();
    let manifest: toml::Value = match toml::from_str(&content) {
        Ok(v) => v,
        Err(_) => return false,
    };

    manifest
        .get("packages")
        .and_then(|p| p.as_table())
        .map(|t| t.contains_key(package_name))
        .unwrap_or(false)
}

/// Helper to get package version from manifest
fn get_package_version(project: &TestProject, package_name: &str) -> Option<String> {
    let manifest_path = project.path().join("zigroot.toml");
    let content = std::fs::read_to_string(&manifest_path).ok()?;
    let manifest: toml::Value = toml::from_str(&content).ok()?;

    manifest
        .get("packages")?
        .get(package_name)?
        .get("version")?
        .as_str()
        .map(String::from)
}

/// Helper to check if lock file exists
fn lock_file_exists(project: &TestProject) -> bool {
    project.path().join("zigroot.lock").exists()
}

/// Helper to get package version from lock file
fn get_lock_file_version(project: &TestProject, package_name: &str) -> Option<String> {
    let lock_path = project.path().join("zigroot.lock");
    if !lock_path.exists() {
        return None;
    }
    let content = std::fs::read_to_string(&lock_path).ok()?;
    let lock: toml::Value = toml::from_str(&content).ok()?;

    // Lock file has [[package]] array
    lock.get("package")?
        .as_array()?
        .iter()
        .find(|p| p.get("name").and_then(|n| n.as_str()) == Some(package_name))?
        .get("version")?
        .as_str()
        .map(String::from)
}

/// Helper to check if lock file is valid TOML
fn is_valid_lock_file(project: &TestProject) -> bool {
    let lock_path = project.path().join("zigroot.lock");
    if !lock_path.exists() {
        return false;
    }
    let content = std::fs::read_to_string(&lock_path).unwrap_or_default();
    toml::from_str::<toml::Value>(&content).is_ok()
}

/// Helper to initialize a project for update tests
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

/// Helper to setup a project with a package already added
fn setup_project_with_package(package_name: &str, version: Option<&str>) -> TestProject {
    let project = setup_project();
    
    let package_spec = if let Some(v) = version {
        format!("{package_name}@{v}")
    } else {
        package_name.to_string()
    };
    
    let output = run_add(&project, &[&package_spec]);
    assert!(
        output.status.success(),
        "Failed to add package: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        manifest_has_package(&project, package_name),
        "Package should be in manifest after add"
    );
    project
}

// ============================================
// Unit Tests for zigroot update
// ============================================

/// Test: Checks for newer versions of all packages
/// **Validates: Requirement 2.6**
#[test]
fn test_update_checks_for_newer_versions() {
    let project = setup_project_with_package("busybox", Some("1.36.0"));

    // Verify initial version
    let initial_version = get_package_version(&project, "busybox");
    assert_eq!(
        initial_version,
        Some("1.36.0".to_string()),
        "Initial version should be 1.36.0"
    );

    let output = run_update(&project, &[]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Command should succeed
    assert!(
        output.status.success(),
        "zigroot update should succeed: stdout={stdout}, stderr={stderr}"
    );

    // Output should indicate checking for updates
    let combined_output = format!("{stdout}{stderr}");
    assert!(
        combined_output.contains("update")
            || combined_output.contains("check")
            || combined_output.contains("busybox")
            || combined_output.contains("newer")
            || combined_output.contains("latest"),
        "Output should indicate checking for updates: {combined_output}"
    );

    // Manifest should remain valid
    assert!(is_valid_manifest(&project), "Manifest should remain valid");
}

/// Test: Updates lock file with new versions
/// **Validates: Requirement 2.6**
#[test]
fn test_update_updates_lock_file() {
    let project = setup_project_with_package("busybox", Some("1.36.0"));

    // Verify lock file exists
    assert!(
        lock_file_exists(&project),
        "Lock file should exist after adding package"
    );

    // Get initial lock file version
    let initial_lock_version = get_lock_file_version(&project, "busybox");
    assert_eq!(
        initial_lock_version,
        Some("1.36.0".to_string()),
        "Initial lock version should be 1.36.0"
    );

    let output = run_update(&project, &[]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Command should succeed
    assert!(
        output.status.success(),
        "zigroot update should succeed: stdout={stdout}, stderr={stderr}"
    );

    // Lock file should still exist and be valid
    assert!(
        lock_file_exists(&project),
        "Lock file should still exist after update"
    );
    assert!(
        is_valid_lock_file(&project),
        "Lock file should be valid after update"
    );

    // Lock file should be updated (version may change if newer available)
    let updated_lock_version = get_lock_file_version(&project, "busybox");
    assert!(
        updated_lock_version.is_some(),
        "Package should still be in lock file after update"
    );
}

/// Test: Updates single package when name specified
/// **Validates: Requirement 2.7**
#[test]
fn test_update_single_package() {
    let project = setup_project();

    // Add two packages
    let output1 = run_add(&project, &["busybox@1.36.0"]);
    assert!(
        output1.status.success(),
        "Failed to add first package: {}",
        String::from_utf8_lossy(&output1.stderr)
    );

    let output2 = run_add(&project, &["dropbear"]);
    assert!(
        output2.status.success(),
        "Failed to add second package: {}",
        String::from_utf8_lossy(&output2.stderr)
    );

    // Verify both packages are in manifest
    assert!(
        manifest_has_package(&project, "busybox"),
        "busybox should be in manifest"
    );
    assert!(
        manifest_has_package(&project, "dropbear"),
        "dropbear should be in manifest"
    );

    // Get initial versions
    let initial_busybox_version = get_lock_file_version(&project, "busybox");
    let initial_dropbear_version = get_lock_file_version(&project, "dropbear");

    // Update only busybox
    let output = run_update(&project, &["busybox"]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Command should succeed
    assert!(
        output.status.success(),
        "zigroot update busybox should succeed: stdout={stdout}, stderr={stderr}"
    );

    // Both packages should still be in manifest
    assert!(
        manifest_has_package(&project, "busybox"),
        "busybox should still be in manifest"
    );
    assert!(
        manifest_has_package(&project, "dropbear"),
        "dropbear should still be in manifest"
    );

    // Lock file should be valid
    assert!(
        is_valid_lock_file(&project),
        "Lock file should be valid after update"
    );

    // dropbear version should be unchanged (we only updated busybox)
    let updated_dropbear_version = get_lock_file_version(&project, "dropbear");
    assert_eq!(
        initial_dropbear_version, updated_dropbear_version,
        "dropbear version should be unchanged when only busybox is updated"
    );
}

/// Test: Update without packages shows appropriate message
/// **Validates: Requirement 2.6**
#[test]
fn test_update_no_packages() {
    let project = setup_project();

    // Don't add any packages

    let output = run_update(&project, &[]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Command should succeed (nothing to update is not an error)
    // Or it should indicate there are no packages to update
    let combined_output = format!("{stdout}{stderr}");
    assert!(
        output.status.success()
            || combined_output.contains("no packages")
            || combined_output.contains("nothing to update")
            || combined_output.contains("up to date"),
        "Update with no packages should succeed or indicate nothing to update: {combined_output}"
    );

    // Manifest should remain valid
    assert!(is_valid_manifest(&project), "Manifest should remain valid");
}

/// Test: Update non-existent package fails gracefully
/// **Validates: Requirement 2.7 (error handling)**
#[test]
fn test_update_nonexistent_package() {
    let project = setup_project_with_package("busybox", None);

    let output = run_update(&project, &["nonexistent-package"]);

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Command should fail or indicate package not found
    assert!(
        !output.status.success()
            || stderr.contains("not found")
            || stderr.contains("not installed"),
        "Updating non-existent package should fail or indicate not found"
    );

    // Manifest should remain valid
    assert!(is_valid_manifest(&project), "Manifest should remain valid");

    // Existing packages should be preserved
    assert!(
        manifest_has_package(&project, "busybox"),
        "Existing packages should be preserved"
    );
}

/// Test: Update preserves manifest validity
/// **Validates: Requirement 2.6**
#[test]
fn test_update_preserves_manifest_validity() {
    let project = setup_project_with_package("busybox", Some("1.36.0"));

    // Verify manifest is valid before update
    assert!(
        is_valid_manifest(&project),
        "Manifest should be valid before update"
    );

    let output = run_update(&project, &[]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Command should succeed
    assert!(
        output.status.success(),
        "zigroot update should succeed: stdout={stdout}, stderr={stderr}"
    );

    // Manifest should remain valid TOML
    assert!(
        is_valid_manifest(&project),
        "Manifest should remain valid after update"
    );

    // Manifest should still have required sections
    let content = project.read_file("zigroot.toml");
    assert!(
        content.contains("[project]"),
        "Manifest should still have [project] section"
    );
}

/// Test: Update multiple packages
/// **Validates: Requirement 2.6**
#[test]
fn test_update_multiple_packages() {
    let project = setup_project();

    // Add multiple packages
    let output1 = run_add(&project, &["busybox@1.36.0"]);
    assert!(
        output1.status.success(),
        "Failed to add busybox: {}",
        String::from_utf8_lossy(&output1.stderr)
    );

    let output2 = run_add(&project, &["dropbear"]);
    assert!(
        output2.status.success(),
        "Failed to add dropbear: {}",
        String::from_utf8_lossy(&output2.stderr)
    );

    let output3 = run_add(&project, &["zlib"]);
    assert!(
        output3.status.success(),
        "Failed to add zlib: {}",
        String::from_utf8_lossy(&output3.stderr)
    );

    // Verify all packages are in manifest
    assert!(manifest_has_package(&project, "busybox"));
    assert!(manifest_has_package(&project, "dropbear"));
    assert!(manifest_has_package(&project, "zlib"));

    // Update all packages
    let output = run_update(&project, &[]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Command should succeed
    assert!(
        output.status.success(),
        "zigroot update should succeed: stdout={stdout}, stderr={stderr}"
    );

    // All packages should still be in manifest
    assert!(
        manifest_has_package(&project, "busybox"),
        "busybox should still be in manifest"
    );
    assert!(
        manifest_has_package(&project, "dropbear"),
        "dropbear should still be in manifest"
    );
    assert!(
        manifest_has_package(&project, "zlib"),
        "zlib should still be in manifest"
    );

    // Lock file should be valid
    assert!(
        is_valid_lock_file(&project),
        "Lock file should be valid after update"
    );
}

/// Test: Update outside project directory fails
/// **Validates: CLI usability**
#[test]
fn test_update_outside_project() {
    let project = TestProject::new();
    // Don't initialize - no zigroot.toml

    let output = run_update(&project, &[]);

    // Command should fail (no manifest)
    assert!(
        !output.status.success(),
        "zigroot update outside project should fail"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("zigroot.toml")
            || stderr.contains("not found")
            || stderr.contains("init"),
        "Error should mention missing manifest or suggest init"
    );
}
