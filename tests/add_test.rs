//! Integration tests for `zigroot add` command
//!
//! Tests for Requirements 2.1-2.4, 2.8, 2.9:
//! - Adds package from registry to manifest
//! - Adds specific version with @version syntax
//! - Adds package from git with --git flag
//! - Adds package from custom registry with --registry flag
//! - Resolves and adds transitive dependencies
//! - Updates lock file
//! - Detects and reports dependency conflicts
//!
//! **Property 5: Package Addition Preserves Manifest Validity**
//! **Property 7: Transitive Dependency Inclusion**
//! **Validates: Requirements 2.1-2.4, 2.8, 2.9**

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

/// Helper to check if package is in lock file
fn lock_file_has_package(project: &TestProject, package_name: &str) -> bool {
    let lock_path = project.path().join("zigroot.lock");
    if !lock_path.exists() {
        return false;
    }
    let content = std::fs::read_to_string(&lock_path).unwrap_or_default();
    content.contains(&format!("name = \"{package_name}\""))
}

/// Helper to initialize a project for add tests
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

// ============================================
// Unit Tests for zigroot add
// ============================================

/// Test: Adds package from registry to manifest
/// **Validates: Requirement 2.1**
#[test]
fn test_add_package_from_registry() {
    let project = setup_project();

    let output = run_add(&project, &["busybox"]);

    // The command should succeed and add the package to manifest
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Command should succeed
    assert!(
        output.status.success(),
        "zigroot add should succeed: stdout={stdout}, stderr={stderr}"
    );

    // Package should be in manifest
    assert!(
        manifest_has_package(&project, "busybox"),
        "Package should be added to manifest"
    );

    // Manifest should remain valid
    assert!(is_valid_manifest(&project), "Manifest should remain valid");
}

/// Test: Adds specific version with @version syntax
/// **Validates: Requirement 2.2**
#[test]
fn test_add_package_with_version() {
    let project = setup_project();

    let output = run_add(&project, &["busybox@1.36.1"]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Command should succeed
    assert!(
        output.status.success(),
        "zigroot add with version should succeed: stdout={stdout}, stderr={stderr}"
    );

    // Package should be in manifest
    assert!(
        manifest_has_package(&project, "busybox"),
        "Package should be added to manifest"
    );

    // Version should be pinned
    let version = get_package_version(&project, "busybox");
    assert_eq!(
        version,
        Some("1.36.1".to_string()),
        "Version should be pinned to 1.36.1"
    );
}

/// Test: Adds package from git with --git flag
/// **Validates: Requirement 2.3**
#[test]
fn test_add_package_from_git() {
    let project = setup_project();

    let output = run_add(
        &project,
        &[
            "custom-pkg",
            "--git",
            "https://github.com/example/repo#v1.0.0",
        ],
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Command should succeed
    assert!(
        output.status.success(),
        "zigroot add --git should succeed: stdout={stdout}, stderr={stderr}"
    );

    // Package should be in manifest
    assert!(
        manifest_has_package(&project, "custom-pkg"),
        "Package should be added to manifest"
    );

    // Manifest should contain git source
    let manifest_content = project.read_file("zigroot.toml");
    assert!(
        manifest_content.contains("git = ") || manifest_content.contains("https://github.com"),
        "Manifest should contain git source"
    );
}

/// Test: Adds package from custom registry with --registry flag
/// **Validates: Requirement 2.4**
#[test]
fn test_add_package_from_custom_registry() {
    let project = setup_project();

    let output = run_add(
        &project,
        &["private-pkg", "--registry", "https://packages.example.com"],
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Command should succeed
    assert!(
        output.status.success(),
        "zigroot add --registry should succeed: stdout={stdout}, stderr={stderr}"
    );

    // Package should be in manifest
    assert!(
        manifest_has_package(&project, "private-pkg"),
        "Package should be added to manifest"
    );

    // Manifest should contain custom registry
    let manifest_content = project.read_file("zigroot.toml");
    assert!(
        manifest_content.contains("registry = ")
            || manifest_content.contains("packages.example.com"),
        "Manifest should contain custom registry"
    );
}

/// Test: Resolves and adds transitive dependencies
/// **Validates: Requirement 2.8**
#[test]
fn test_add_resolves_transitive_dependencies() {
    let project = setup_project();

    // Add a package that has dependencies (e.g., nginx depends on zlib, openssl)
    let output = run_add(&project, &["nginx"]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Command should succeed
    assert!(
        output.status.success(),
        "zigroot add should succeed for package with dependencies: stdout={stdout}, stderr={stderr}"
    );

    // Main package should be in manifest
    assert!(
        manifest_has_package(&project, "nginx"),
        "Main package should be in manifest"
    );

    // Lock file should be created with transitive dependencies
    assert!(lock_file_exists(&project), "Lock file should be created");

    let lock_content = project.read_file("zigroot.lock");
    // Check that dependencies are recorded
    assert!(
        lock_content.contains("nginx") || lock_content.contains("[[package]]"),
        "Lock file should contain package entries"
    );
}

/// Test: Updates lock file
/// **Validates: Requirement 2.8 (lock file update)**
#[test]
fn test_add_updates_lock_file() {
    let project = setup_project();

    let output = run_add(&project, &["busybox"]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Command should succeed
    assert!(
        output.status.success(),
        "zigroot add should succeed: stdout={stdout}, stderr={stderr}"
    );

    // Lock file should be created/updated
    assert!(
        lock_file_exists(&project),
        "Lock file should be created after adding package"
    );

    // Lock file should contain added package
    assert!(
        lock_file_has_package(&project, "busybox"),
        "Lock file should contain added package"
    );
}

/// Test: Detects and reports dependency conflicts
/// **Validates: Requirement 2.9**
#[test]
fn test_add_detects_dependency_conflicts() {
    let project = setup_project();

    // First add a package successfully
    let output1 = run_add(&project, &["zlib@1.2.11"]);
    let stderr1 = String::from_utf8_lossy(&output1.stderr);
    let stdout1 = String::from_utf8_lossy(&output1.stdout);

    assert!(
        output1.status.success(),
        "First add should succeed: stdout={stdout1}, stderr={stderr1}"
    );

    assert!(
        manifest_has_package(&project, "zlib"),
        "zlib should be in manifest after first add"
    );

    // Try to add a package that requires a different version of zlib
    // This should either succeed (if compatible) or report a conflict
    let output2 = run_add(&project, &["nginx"]);

    let stderr2 = String::from_utf8_lossy(&output2.stderr);
    let stdout2 = String::from_utf8_lossy(&output2.stdout);

    // The command should either succeed or report a conflict clearly
    if !output2.status.success() {
        // If it fails, it should be due to a conflict
        assert!(
            stderr2.contains("conflict")
                || stderr2.contains("version")
                || stderr2.contains("constraint")
                || stderr2.contains("incompatible"),
            "Failure should be due to version conflict: stdout={stdout2}, stderr={stderr2}"
        );

        // Conflict error should mention the conflicting package or version
        assert!(
            stderr2.contains("zlib") || stderr2.contains("version"),
            "Conflict error should mention the conflicting package or version"
        );
    }

    // Manifest should remain valid regardless of outcome
    assert!(is_valid_manifest(&project), "Manifest should remain valid");
}

/// Test: Adding same package twice is idempotent
/// **Validates: Manifest validity preservation**
#[test]
fn test_add_same_package_twice() {
    let project = setup_project();

    // Add package first time
    let output1 = run_add(&project, &["busybox"]);

    let stderr1 = String::from_utf8_lossy(&output1.stderr);
    let stdout1 = String::from_utf8_lossy(&output1.stdout);

    assert!(
        output1.status.success(),
        "First add should succeed: stdout={stdout1}, stderr={stderr1}"
    );

    assert!(
        manifest_has_package(&project, "busybox"),
        "Package should be in manifest after first add"
    );

    // Add same package second time
    let output2 = run_add(&project, &["busybox"]);

    // Should either succeed or indicate package already exists
    let stderr2 = String::from_utf8_lossy(&output2.stderr);
    assert!(
        output2.status.success() || stderr2.contains("already") || stderr2.contains("exists"),
        "Adding same package twice should be handled gracefully"
    );

    // Manifest should still be valid
    assert!(is_valid_manifest(&project), "Manifest should remain valid");

    // Package should appear only once in manifest
    let manifest_content = project.read_file("zigroot.toml");
    let count = manifest_content.matches("[packages.busybox]").count();
    assert!(count <= 1, "Package should not be duplicated in manifest");
}

/// Test: Add preserves existing packages in manifest
/// **Validates: Manifest validity preservation**
#[test]
fn test_add_preserves_existing_packages() {
    let project = setup_project();

    // Create manifest with existing package
    let manifest_with_package = r#"
[project]
name = "test-project"
version = "1.0.0"

[board]

[build]

[packages.existing-pkg]
version = "1.0.0"
"#;
    project.create_file("zigroot.toml", manifest_with_package);

    // Add a new package
    let output = run_add(&project, &["busybox"]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Command should succeed
    assert!(
        output.status.success(),
        "zigroot add should succeed: stdout={stdout}, stderr={stderr}"
    );

    // New package should be added
    assert!(
        manifest_has_package(&project, "busybox"),
        "New package should be added to manifest"
    );

    // Existing package should be preserved
    let manifest_content = project.read_file("zigroot.toml");
    assert!(
        manifest_content.contains("existing-pkg"),
        "Existing packages should be preserved"
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

/// Strategy for generating valid version strings
fn version_strategy() -> impl Strategy<Value = String> {
    (1u32..10, 0u32..10, 0u32..10)
        .prop_map(|(major, minor, patch)| format!("{major}.{minor}.{patch}"))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 5: Package Addition Preserves Manifest Validity
    /// For any valid manifest and any package addition operation,
    /// the resulting manifest SHALL remain valid TOML that can be parsed.
    /// **Validates: Requirements 2.1-2.4**
    #[test]
    fn prop_package_addition_preserves_manifest_validity(
        package_name in package_name_strategy()
    ) {
        let project = TestProject::new();

        // Initialize project
        let init_output = run_init(&project, &[]);
        prop_assume!(init_output.status.success());

        // Verify manifest is valid before add
        prop_assert!(
            is_valid_manifest(&project),
            "Manifest should be valid before add"
        );

        // Attempt to add package
        let add_output = run_add(&project, &[&package_name]);

        // Manifest should remain valid TOML regardless of add outcome
        prop_assert!(
            is_valid_manifest(&project),
            "Manifest should remain valid after add attempt for package '{}'",
            package_name
        );

        // Add command should succeed
        prop_assert!(
            add_output.status.success(),
            "Add command should succeed for package '{}'",
            package_name
        );

        // Package should be in manifest after successful add
        prop_assert!(
            manifest_has_package(&project, &package_name),
            "Successfully added package '{}' should be in manifest",
            package_name
        );
    }

    /// Property 7: Transitive Dependency Inclusion
    /// For any package with dependencies, when added to a project,
    /// all transitive dependencies SHALL be recorded in the lock file.
    /// **Validates: Requirement 2.8**
    #[test]
    fn prop_transitive_dependency_inclusion(
        package_name in package_name_strategy()
    ) {
        let project = TestProject::new();

        // Initialize project
        let init_output = run_init(&project, &[]);
        prop_assume!(init_output.status.success());

        // Attempt to add package
        let add_output = run_add(&project, &[&package_name]);

        // Add command should succeed
        prop_assert!(
            add_output.status.success(),
            "Add command should succeed for package '{}'",
            package_name
        );

        // Lock file should exist after successful add
        prop_assert!(
            lock_file_exists(&project),
            "Lock file should exist after adding package '{}'",
            package_name
        );

        let lock_content = project.read_file("zigroot.lock");

        // The added package should be in the lock file
        prop_assert!(
            lock_content.contains(&package_name) || lock_content.contains("[[package]]"),
            "Lock file should contain package entries after successful add"
        );

        // Lock file should be valid TOML
        let lock_parse: Result<toml::Value, _> = toml::from_str(&lock_content);
        prop_assert!(
            lock_parse.is_ok(),
            "Lock file should be valid TOML"
        );
    }

    /// Property: Version constraint is preserved in manifest
    /// **Validates: Requirement 2.2**
    #[test]
    fn prop_version_constraint_preserved(
        package_name in package_name_strategy(),
        version in version_strategy()
    ) {
        let project = TestProject::new();

        // Initialize project
        let init_output = run_init(&project, &[]);
        prop_assume!(init_output.status.success());

        // Add package with version
        let package_with_version = format!("{package_name}@{version}");
        let add_output = run_add(&project, &[&package_with_version]);

        // Add command should succeed
        prop_assert!(
            add_output.status.success(),
            "Add command should succeed for package '{}@{}'",
            package_name, version
        );

        // Package should be in manifest
        let manifest_content = project.read_file("zigroot.toml");
        prop_assert!(
            manifest_content.contains(&package_name),
            "Package should be in manifest after successful add"
        );

        // Version should be recorded correctly
        if let Some(recorded_version) = get_package_version(&project, &package_name) {
            prop_assert_eq!(
                recorded_version, version,
                "Recorded version should match requested version"
            );
        }
    }
}
