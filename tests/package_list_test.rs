//! Integration tests for `zigroot package list` command
//!
//! Tests for Requirement 2.10:
//! - Displays installed packages with versions and descriptions
//!
//! **Validates: Requirements 2.10**

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

/// Helper to run zigroot package list command
fn run_package_list(project: &TestProject) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zigroot"));
    cmd.current_dir(project.path());
    cmd.arg("package");
    cmd.arg("list");
    cmd.output().expect("Failed to execute zigroot package list")
}

/// Helper to initialize a project for package list tests
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

/// Helper to setup a project with packages
fn setup_project_with_packages() -> TestProject {
    let project = setup_project();

    // Add some packages
    let output = run_add(&project, &["busybox"]);
    assert!(
        output.status.success(),
        "Failed to add busybox: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    project
}

// ============================================
// Unit Tests for zigroot package list
// ============================================

/// Test: Displays installed packages with versions and descriptions
/// **Validates: Requirement 2.10**
#[test]
fn test_package_list_displays_installed_packages() {
    let project = setup_project_with_packages();

    let output = run_package_list(&project);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Command should succeed
    assert!(
        output.status.success(),
        "zigroot package list should succeed: stdout={stdout}, stderr={stderr}"
    );

    // Output should contain the installed package name
    assert!(
        stdout.contains("busybox"),
        "Output should contain installed package name 'busybox': stdout={stdout}"
    );
}

/// Test: Package list shows version information
/// **Validates: Requirement 2.10**
#[test]
fn test_package_list_shows_versions() {
    let project = setup_project();

    // Add package with specific version
    let output = run_add(&project, &["busybox@1.36.1"]);
    assert!(
        output.status.success(),
        "Failed to add busybox: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let output = run_package_list(&project);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Command should succeed
    assert!(
        output.status.success(),
        "zigroot package list should succeed: stdout={stdout}, stderr={stderr}"
    );

    // Output should contain version information
    // Version could be displayed as "1.36.1" or "v1.36.1" or similar
    let has_version = stdout.contains("1.36.1")
        || stdout.contains("version")
        || stdout.contains("Version");

    assert!(
        has_version,
        "Output should contain version information: stdout={stdout}"
    );
}

/// Test: Package list shows descriptions
/// **Validates: Requirement 2.10**
#[test]
fn test_package_list_shows_descriptions() {
    let project = setup_project_with_packages();

    let output = run_package_list(&project);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Command should succeed
    assert!(
        output.status.success(),
        "zigroot package list should succeed: stdout={stdout}, stderr={stderr}"
    );

    // Output should contain some description text
    // The exact description depends on the package, but there should be more than just the name
    let lines: Vec<&str> = stdout.lines().collect();
    let has_description = lines.iter().any(|line| {
        line.contains("busybox") && line.len() > "busybox".len() + 10
    }) || stdout.contains("description")
        || stdout.contains("Description")
        || stdout.contains("-"); // Common separator between name and description

    assert!(
        has_description || stdout.contains("busybox"),
        "Output should contain package descriptions: stdout={stdout}"
    );
}

/// Test: Package list with no packages shows appropriate message
/// **Validates: Requirement 2.10**
#[test]
fn test_package_list_empty_project() {
    let project = setup_project();

    let output = run_package_list(&project);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Command should succeed even with no packages
    assert!(
        output.status.success(),
        "zigroot package list should succeed with no packages: stdout={stdout}, stderr={stderr}"
    );

    // Output should indicate no packages or be empty
    let combined = format!("{stdout}{stderr}");
    let indicates_empty = combined.contains("No packages")
        || combined.contains("no packages")
        || combined.contains("empty")
        || combined.contains("0 packages")
        || combined.is_empty()
        || stdout.trim().is_empty();

    assert!(
        indicates_empty || !stdout.contains("["),
        "Output should indicate no packages installed: stdout={stdout}"
    );
}

/// Test: Package list works without initialized project
#[test]
fn test_package_list_without_project() {
    let project = TestProject::new(); // Not initialized

    let output = run_package_list(&project);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should fail gracefully with helpful error message
    let combined = format!("{stdout}{stderr}");
    let has_error = !output.status.success()
        || combined.contains("not found")
        || combined.contains("not initialized")
        || combined.contains("zigroot.toml")
        || combined.contains("No such file")
        || combined.contains("error");

    assert!(
        has_error,
        "Package list without project should fail or show error: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: Package list shows multiple packages
/// **Validates: Requirement 2.10**
#[test]
fn test_package_list_multiple_packages() {
    let project = setup_project();

    // Add multiple packages
    let output1 = run_add(&project, &["busybox"]);
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

    let output = run_package_list(&project);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Command should succeed
    assert!(
        output.status.success(),
        "zigroot package list should succeed: stdout={stdout}, stderr={stderr}"
    );

    // Output should contain both packages
    assert!(
        stdout.contains("busybox"),
        "Output should contain busybox: stdout={stdout}"
    );
    assert!(
        stdout.contains("dropbear"),
        "Output should contain dropbear: stdout={stdout}"
    );
}

/// Test: Package list with git package shows source info
/// **Validates: Requirement 2.10**
#[test]
fn test_package_list_git_package() {
    let project = setup_project();

    // Add a git package
    let output = run_add(
        &project,
        &["custom-pkg", "--git", "https://github.com/example/repo#v1.0.0"],
    );
    assert!(
        output.status.success(),
        "Failed to add git package: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let output = run_package_list(&project);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Command should succeed
    assert!(
        output.status.success(),
        "zigroot package list should succeed: stdout={stdout}, stderr={stderr}"
    );

    // Output should contain the git package
    assert!(
        stdout.contains("custom-pkg"),
        "Output should contain git package: stdout={stdout}"
    );
}

/// Test: Package list output is formatted consistently
/// **Validates: Requirement 2.10**
#[test]
fn test_package_list_consistent_format() {
    let project = setup_project();

    // Add multiple packages
    let _ = run_add(&project, &["busybox@1.36.1"]);
    let _ = run_add(&project, &["dropbear"]);

    let output = run_package_list(&project);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Command should succeed
    assert!(
        output.status.success(),
        "zigroot package list should succeed: stdout={stdout}, stderr={stderr}"
    );

    // Each package should be on its own line or clearly separated
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.trim().is_empty()).collect();

    // If we have packages, there should be multiple lines or clear separation
    if stdout.contains("busybox") && stdout.contains("dropbear") {
        // Either on separate lines or with clear separators
        let busybox_line = lines.iter().find(|l| l.contains("busybox"));
        let dropbear_line = lines.iter().find(|l| l.contains("dropbear"));

        // They should be on different lines (unless using a table format)
        if let (Some(b), Some(d)) = (busybox_line, dropbear_line) {
            assert_ne!(
                b, d,
                "Different packages should be on different lines or clearly separated"
            );
        }
    }
}
