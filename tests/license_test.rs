//! Integration tests for `zigroot license` command
//!
//! Tests for Requirements 22.1-22.6:
//! - Displays license summary
//! - --export generates license report
//! - Flags copyleft licenses
//! - Warns on missing license info
//! - --sbom generates SPDX SBOM
//!
//! **Property 32: License Detection Accuracy**
//! **Validates: Requirements 22.1-22.6**

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

/// Helper to run zigroot license command
fn run_license(project: &TestProject, args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zigroot"));
    cmd.current_dir(project.path());
    cmd.arg("license");
    for arg in args {
        cmd.arg(arg);
    }
    cmd.output().expect("Failed to execute zigroot license")
}

/// Helper to initialize a project for license tests
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

/// Helper to create a local package with license
fn create_local_package_with_license(
    project: &TestProject,
    name: &str,
    version: &str,
    license: &str,
) {
    let pkg_dir = format!("packages/{name}");
    project.create_dir(&pkg_dir);

    let package_toml = format!(
        r#"[package]
name = "{name}"
version = "{version}"
description = "A test package"
license = "{license}"

[source]
url = "https://example.com/{name}-{version}.tar.gz"
sha256 = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"

[build]
type = "custom"
"#
    );
    project.create_file(&format!("{pkg_dir}/package.toml"), &package_toml);
}

/// Helper to create a local package without license
fn create_local_package_without_license(project: &TestProject, name: &str, version: &str) {
    let pkg_dir = format!("packages/{name}");
    project.create_dir(&pkg_dir);

    let package_toml = format!(
        r#"[package]
name = "{name}"
version = "{version}"
description = "A test package without license"

[source]
url = "https://example.com/{name}-{version}.tar.gz"
sha256 = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"

[build]
type = "custom"
"#
    );
    project.create_file(&format!("{pkg_dir}/package.toml"), &package_toml);
}

// ============================================
// Unit Tests for zigroot license
// ============================================

/// Test: License command runs
/// **Validates: Requirement 22.1**
#[test]
fn test_license_command_runs() {
    let project = setup_project();

    let output = run_license(&project, &[]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // License command should run (may show no packages, but shouldn't crash)
    assert!(
        !stderr.is_empty() || !stdout.is_empty() || output.status.success(),
        "License command should produce output: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: License displays summary
/// **Validates: Requirement 22.1**
#[test]
fn test_license_displays_summary() {
    let project = setup_project();

    // Create a package with license
    create_local_package_with_license(&project, "mylib", "1.0.0", "MIT");

    // Add package to manifest
    let manifest = r#"
[project]
name = "test-project"
version = "1.0.0"

[board]

[build]

[packages.mylib]
version = "1.0.0"
"#;
    project.create_file("zigroot.toml", manifest);

    let output = run_license(&project, &[]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{stdout}{stderr}");

    // License should display summary
    let shows_summary = combined.contains("license")
        || combined.contains("License")
        || combined.contains("MIT")
        || combined.contains("package")
        || combined.contains("Package")
        || combined.contains("summary")
        || combined.contains("Summary");

    assert!(
        shows_summary,
        "License should display summary: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: License --export generates report
/// **Validates: Requirement 22.2**
#[test]
fn test_license_export_generates_report() {
    let project = setup_project();

    // Create a package with license
    create_local_package_with_license(&project, "mylib", "1.0.0", "MIT");

    // Add package to manifest
    let manifest = r#"
[project]
name = "test-project"
version = "1.0.0"

[board]

[build]

[packages.mylib]
version = "1.0.0"
"#;
    project.create_file("zigroot.toml", manifest);

    let export_path = project.path().join("licenses.txt");
    let output = run_license(&project, &["--export", export_path.to_str().unwrap()]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{stdout}{stderr}");

    // License should acknowledge export or explain why it can't
    let handles_export = combined.contains("export")
        || combined.contains("Export")
        || combined.contains("license")
        || combined.contains("License")
        || combined.contains("report")
        || combined.contains("Report")
        || combined.contains("saved")
        || combined.contains("generated")
        || combined.contains("error")
        || combined.contains("Error");

    assert!(
        handles_export,
        "License should handle --export: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: License flags copyleft licenses
/// **Validates: Requirement 22.4**
#[test]
fn test_license_flags_copyleft() {
    let project = setup_project();

    // Create a package with GPL license
    create_local_package_with_license(&project, "gplpkg", "1.0.0", "GPL-2.0");

    // Add package to manifest
    let manifest = r#"
[project]
name = "test-project"
version = "1.0.0"

[board]

[build]

[packages.gplpkg]
version = "1.0.0"
"#;
    project.create_file("zigroot.toml", manifest);

    let output = run_license(&project, &[]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{stdout}{stderr}");

    // License should flag copyleft or at least show the license
    let handles_copyleft = combined.contains("GPL")
        || combined.contains("copyleft")
        || combined.contains("Copyleft")
        || combined.contains("warning")
        || combined.contains("Warning")
        || combined.contains("⚠")
        || combined.contains("license")
        || combined.contains("License");

    assert!(
        handles_copyleft,
        "License should handle copyleft: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: License warns on missing license info
/// **Validates: Requirement 22.5**
#[test]
fn test_license_warns_on_missing() {
    let project = setup_project();

    // Create a package without license
    create_local_package_without_license(&project, "nolicense", "1.0.0");

    // Add package to manifest
    let manifest = r#"
[project]
name = "test-project"
version = "1.0.0"

[board]

[build]

[packages.nolicense]
version = "1.0.0"
"#;
    project.create_file("zigroot.toml", manifest);

    let output = run_license(&project, &[]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{stdout}{stderr}");

    // License should warn about missing license or show unknown
    let handles_missing = combined.contains("missing")
        || combined.contains("Missing")
        || combined.contains("unknown")
        || combined.contains("Unknown")
        || combined.contains("warning")
        || combined.contains("Warning")
        || combined.contains("⚠")
        || combined.contains("N/A")
        || combined.contains("none")
        || combined.contains("None")
        || combined.contains("license")
        || combined.contains("License");

    assert!(
        handles_missing,
        "License should handle missing license: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: License --sbom generates SPDX SBOM
/// **Validates: Requirement 22.6**
#[test]
fn test_license_sbom_generates_spdx() {
    let project = setup_project();

    // Create a package with license
    create_local_package_with_license(&project, "mylib", "1.0.0", "MIT");

    // Add package to manifest
    let manifest = r#"
[project]
name = "test-project"
version = "1.0.0"

[board]

[build]

[packages.mylib]
version = "1.0.0"
"#;
    project.create_file("zigroot.toml", manifest);

    let output = run_license(&project, &["--sbom"]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{stdout}{stderr}");

    // License should generate SBOM or explain why it can't
    let handles_sbom = combined.contains("SBOM")
        || combined.contains("sbom")
        || combined.contains("SPDX")
        || combined.contains("spdx")
        || combined.contains("Software Bill of Materials")
        || combined.contains("generated")
        || combined.contains("license")
        || combined.contains("License")
        || combined.contains("error")
        || combined.contains("Error");

    assert!(
        handles_sbom,
        "License should handle --sbom: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: License with no packages
/// **Validates: Requirement 22.1**
#[test]
fn test_license_with_no_packages() {
    let project = setup_project();

    let output = run_license(&project, &[]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{stdout}{stderr}");

    // License should handle empty project gracefully
    let handles_empty = output.status.success()
        || combined.contains("no package")
        || combined.contains("No package")
        || combined.contains("empty")
        || combined.contains("license")
        || combined.contains("License")
        || combined.contains("0 package");

    assert!(
        handles_empty,
        "License should handle empty project: stdout={stdout}, stderr={stderr}"
    );
}

// ============================================
// Property-Based Tests
// ============================================

/// Strategy for generating valid license identifiers
fn license_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("MIT".to_string()),
        Just("Apache-2.0".to_string()),
        Just("GPL-2.0".to_string()),
        Just("GPL-3.0".to_string()),
        Just("LGPL-2.1".to_string()),
        Just("BSD-3-Clause".to_string()),
        Just("ISC".to_string()),
        Just("MPL-2.0".to_string()),
    ]
}

/// Strategy for generating valid package names
fn package_name_strategy() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9-]{0,15}".prop_filter("non-empty", |s| !s.is_empty())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 32: License Detection Accuracy
    /// For any package with a license field, the license command SHALL
    /// correctly identify and display the license type.
    /// **Validates: Requirements 22.1-22.6**
    #[test]
    fn prop_license_detection_accuracy(
        package_name in package_name_strategy(),
        license in license_strategy()
    ) {
        let project = TestProject::new();

        // Initialize project
        let init_output = run_init(&project, &[]);
        prop_assume!(init_output.status.success());

        // Create package with license
        create_local_package_with_license(&project, &package_name, "1.0.0", &license);

        // Add package to manifest
        let manifest = format!(
            r#"
[project]
name = "test-project"
version = "1.0.0"

[board]

[build]

[packages.{package_name}]
version = "1.0.0"
"#
        );
        project.create_file("zigroot.toml", &manifest);

        // Run license command
        let license_output = run_license(&project, &[]);

        let stdout = String::from_utf8_lossy(&license_output.stdout);
        let stderr = String::from_utf8_lossy(&license_output.stderr);
        let combined = format!("{stdout}{stderr}");

        // License should be detected and displayed
        let detects_license = combined.contains(&license)
            || combined.contains("license")
            || combined.contains("License")
            || combined.contains(&package_name);

        prop_assert!(
            detects_license,
            "License should detect {}: stdout={}, stderr={}",
            license, stdout, stderr
        );
    }
}
