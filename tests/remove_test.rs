//! Integration tests for `zigroot remove` command
//!
//! Tests for Requirement 2.5:
//! - Removes package from manifest
//! - Updates lock file
//! - Manifest remains valid after removal
//!
//! **Property 6: Package Removal Preserves Manifest Validity**
//! **Validates: Requirements 2.5**

mod common;

use common::TestProject;
use proptest::prelude::*;
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

/// Helper to run zigroot remove command
fn run_remove(project: &TestProject, args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zigroot"));
    cmd.current_dir(project.path());
    cmd.arg("remove");
    for arg in args {
        cmd.arg(arg);
    }
    cmd.output().expect("Failed to execute zigroot remove")
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

/// Helper to check if lock file exists
fn lock_file_exists(project: &TestProject) -> bool {
    project.path().join("zigroot.lock").exists()
}

/// Helper to check if package is in lock file
fn lock_file_has_package(project: &TestProject, package_name: &str) -> bool {
    let lock_path = project.path().join("zigroot.lock");
    if !lock_path.exists() {
        return false;
    }
    let content = std::fs::read_to_string(&lock_path).unwrap_or_default();
    content.contains(&format!("name = \"{package_name}\""))
}

/// Helper to initialize a project for remove tests
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
fn setup_project_with_package(package_name: &str) -> TestProject {
    let project = setup_project();
    let output = run_add(&project, &[package_name]);
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
// Unit Tests for zigroot remove
// ============================================

/// Test: Removes package from manifest
/// **Validates: Requirement 2.5**
#[test]
fn test_remove_package_from_manifest() {
    let project = setup_project_with_package("busybox");

    // Verify package is in manifest before removal
    assert!(
        manifest_has_package(&project, "busybox"),
        "Package should be in manifest before removal"
    );

    let output = run_remove(&project, &["busybox"]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Command should succeed
    assert!(
        output.status.success(),
        "zigroot remove should succeed: stdout={stdout}, stderr={stderr}"
    );

    // Package should be removed from manifest
    assert!(
        !manifest_has_package(&project, "busybox"),
        "Package should be removed from manifest"
    );

    // Manifest should remain valid
    assert!(is_valid_manifest(&project), "Manifest should remain valid after removal");
}

/// Test: Updates lock file after removal
/// **Validates: Requirement 2.5**
#[test]
fn test_remove_updates_lock_file() {
    let project = setup_project_with_package("busybox");

    // Verify lock file has package before removal
    assert!(
        lock_file_exists(&project),
        "Lock file should exist after adding package"
    );
    assert!(
        lock_file_has_package(&project, "busybox"),
        "Lock file should contain package before removal"
    );

    let output = run_remove(&project, &["busybox"]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Command should succeed
    assert!(
        output.status.success(),
        "zigroot remove should succeed: stdout={stdout}, stderr={stderr}"
    );

    // Lock file should be updated (package removed)
    assert!(
        !lock_file_has_package(&project, "busybox"),
        "Package should be removed from lock file"
    );
}

/// Test: Manifest remains valid after removal
/// **Validates: Requirement 2.5**
#[test]
fn test_remove_manifest_remains_valid() {
    let project = setup_project_with_package("busybox");

    // Verify manifest is valid before removal
    assert!(
        is_valid_manifest(&project),
        "Manifest should be valid before removal"
    );

    let output = run_remove(&project, &["busybox"]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Command should succeed
    assert!(
        output.status.success(),
        "zigroot remove should succeed: stdout={stdout}, stderr={stderr}"
    );

    // Manifest should remain valid TOML
    assert!(
        is_valid_manifest(&project),
        "Manifest should remain valid after removal"
    );

    // Manifest should still have required sections
    let content = project.read_file("zigroot.toml");
    assert!(
        content.contains("[project]"),
        "Manifest should still have [project] section"
    );
}

/// Test: Removing non-existent package fails gracefully
/// **Validates: Requirement 2.5 (error handling)**
#[test]
fn test_remove_nonexistent_package() {
    let project = setup_project();

    let output = run_remove(&project, &["nonexistent-package"]);

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Command should fail or indicate package not found
    assert!(
        !output.status.success() || stderr.contains("not found") || stderr.contains("not installed"),
        "Removing non-existent package should fail or indicate not found"
    );

    // Manifest should remain valid
    assert!(is_valid_manifest(&project), "Manifest should remain valid");
}

/// Test: Removing preserves other packages
/// **Validates: Requirement 2.5**
#[test]
fn test_remove_preserves_other_packages() {
    let project = setup_project();

    // Add two packages
    let output1 = run_add(&project, &["busybox"]);
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

    // Remove only busybox
    let output = run_remove(&project, &["busybox"]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Command should succeed
    assert!(
        output.status.success(),
        "zigroot remove should succeed: stdout={stdout}, stderr={stderr}"
    );

    // busybox should be removed
    assert!(
        !manifest_has_package(&project, "busybox"),
        "busybox should be removed from manifest"
    );

    // dropbear should still be present
    assert!(
        manifest_has_package(&project, "dropbear"),
        "dropbear should still be in manifest"
    );

    // Manifest should remain valid
    assert!(is_valid_manifest(&project), "Manifest should remain valid");
}

/// Test: Remove without arguments shows error
/// **Validates: CLI usability**
#[test]
fn test_remove_without_arguments() {
    let project = setup_project();

    let output = run_remove(&project, &[]);

    // Command should fail (missing required argument)
    assert!(
        !output.status.success(),
        "zigroot remove without arguments should fail"
    );

    // Manifest should remain valid
    assert!(is_valid_manifest(&project), "Manifest should remain valid");
}

// ============================================
// Property-Based Tests
// ============================================

/// Strategy for generating valid package names
fn package_name_strategy() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9-]{0,20}".prop_filter("non-empty", |s| !s.is_empty())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 6: Package Removal Preserves Manifest Validity
    /// For any valid manifest with packages, removing a package SHALL result
    /// in a valid manifest that can still be parsed.
    /// **Validates: Requirements 2.5**
    #[test]
    fn prop_package_removal_preserves_manifest_validity(
        package_name in package_name_strategy()
    ) {
        let project = TestProject::new();

        // Initialize project
        let init_output = run_init(&project, &[]);
        prop_assume!(init_output.status.success());

        // Verify manifest is valid before operations
        prop_assert!(
            is_valid_manifest(&project),
            "Manifest should be valid after init"
        );

        // Add package
        let add_output = run_add(&project, &[&package_name]);
        prop_assume!(add_output.status.success());

        // Verify manifest is valid after add
        prop_assert!(
            is_valid_manifest(&project),
            "Manifest should be valid after add"
        );

        // Verify package is in manifest
        prop_assert!(
            manifest_has_package(&project, &package_name),
            "Package '{}' should be in manifest after add",
            package_name
        );

        // Remove package
        let remove_output = run_remove(&project, &[&package_name]);

        // Remove command should succeed
        prop_assert!(
            remove_output.status.success(),
            "Remove command should succeed for package '{}'",
            package_name
        );

        // Manifest should remain valid TOML after removal
        prop_assert!(
            is_valid_manifest(&project),
            "Manifest should remain valid after removing package '{}'",
            package_name
        );

        // Package should no longer be in manifest
        prop_assert!(
            !manifest_has_package(&project, &package_name),
            "Package '{}' should not be in manifest after removal",
            package_name
        );
    }

    /// Property: Lock file is updated after removal
    /// **Validates: Requirements 2.5**
    #[test]
    fn prop_lock_file_updated_after_removal(
        package_name in package_name_strategy()
    ) {
        let project = TestProject::new();

        // Initialize project
        let init_output = run_init(&project, &[]);
        prop_assume!(init_output.status.success());

        // Add package
        let add_output = run_add(&project, &[&package_name]);
        prop_assume!(add_output.status.success());

        // Verify lock file exists and has package
        prop_assert!(
            lock_file_exists(&project),
            "Lock file should exist after add"
        );
        prop_assert!(
            lock_file_has_package(&project, &package_name),
            "Lock file should contain package '{}' after add",
            package_name
        );

        // Remove package
        let remove_output = run_remove(&project, &[&package_name]);

        // Remove command should succeed
        prop_assert!(
            remove_output.status.success(),
            "Remove command should succeed for package '{}'",
            package_name
        );

        // Package should no longer be in lock file
        prop_assert!(
            !lock_file_has_package(&project, &package_name),
            "Package '{}' should not be in lock file after removal",
            package_name
        );
    }
}
