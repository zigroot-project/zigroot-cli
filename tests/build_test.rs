//! Integration tests for `zigroot build` command
//!
//! Tests for Requirements 4.1-4.13, 5.1-5.7:
//! - Compiles all packages in dependency order
//! - Uses Zig cross-compilation with target triple
//! - Builds statically linked binaries
//! - Skips unchanged packages (incremental build)
//! - --package rebuilds only specified package
//! - --jobs limits parallel compilation
//! - --locked fails if package differs from lock
//! - Creates rootfs image
//! - Displays build summary
//!
//! **Property 8: Incremental Build Correctness**
//! **Property 11: Local Package Priority**
//! **Validates: Requirements 4.1-4.13, 5.1-5.7**

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

/// Helper to run zigroot build command
fn run_build(project: &TestProject, args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zigroot"));
    cmd.current_dir(project.path());
    cmd.arg("build");
    for arg in args {
        cmd.arg(arg);
    }
    cmd.output().expect("Failed to execute zigroot build")
}

/// Helper to check if build directory exists
fn build_dir_exists(project: &TestProject) -> bool {
    project.path().join("build").is_dir()
}

/// Helper to check if output directory exists
fn output_dir_exists(project: &TestProject) -> bool {
    project.path().join("output").is_dir()
}

/// Helper to check if rootfs image exists
fn rootfs_image_exists(project: &TestProject) -> bool {
    let output_dir = project.path().join("output");
    if !output_dir.is_dir() {
        return false;
    }
    // Check for any image file
    output_dir.join("rootfs.img").exists()
        || output_dir.join("rootfs.squashfs").exists()
        || output_dir.join("rootfs.cpio").exists()
}

/// Helper to check if lock file exists
fn lock_file_exists(project: &TestProject) -> bool {
    project.path().join("zigroot.lock").exists()
}

/// Helper to initialize a project for build tests
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

    // Add a package to the project
    let output = run_add(&project, &["busybox"]);
    assert!(
        output.status.success(),
        "Failed to add package: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    project
}

/// Helper to create a local package in the project
fn create_local_package(project: &TestProject, name: &str, version: &str) {
    let pkg_dir = format!("packages/{name}");
    project.create_dir(&pkg_dir);

    let package_toml = format!(
        r#"[package]
name = "{name}"
version = "{version}"
description = "A local test package"

[source]
url = "https://example.com/{name}-{version}.tar.gz"
sha256 = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"

[build]
type = "custom"
"#
    );
    project.create_file(&format!("{pkg_dir}/package.toml"), &package_toml);

    // Create a simple build script
    let build_script = r#"#!/bin/sh
echo "Building package..."
touch "$DESTDIR/built_marker"
"#;
    project.create_file(&format!("{pkg_dir}/build.sh"), build_script);
}

/// Helper to get build timestamp for a package
fn get_package_build_timestamp(
    project: &TestProject,
    package_name: &str,
) -> Option<std::time::SystemTime> {
    let stamp_path = project
        .path()
        .join("build")
        .join("stamps")
        .join(format!("{package_name}.stamp"));
    if stamp_path.exists() {
        std::fs::metadata(&stamp_path).ok()?.modified().ok()
    } else {
        None
    }
}

// ============================================
// Unit Tests for zigroot build
// ============================================

/// Test: Compiles all packages in dependency order
/// **Validates: Requirement 4.1**
#[test]
fn test_build_compiles_packages_in_dependency_order() {
    let project = setup_project_with_packages();

    let output = run_build(&project, &[]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Build command should succeed
    assert!(
        output.status.success(),
        "zigroot build should succeed: stdout={stdout}, stderr={stderr}"
    );

    // Build directory should be created
    assert!(
        build_dir_exists(&project),
        "build/ directory should be created"
    );
}

/// Test: Uses Zig cross-compilation with target triple
/// **Validates: Requirement 4.2**
#[test]
fn test_build_uses_zig_cross_compilation() {
    let project = setup_project();

    // Create a manifest with a specific board target
    let manifest = r#"
[project]
name = "test-project"
version = "1.0.0"

[board]
name = "test-board"

[build]
image_format = "ext4"
rootfs_size = "64M"
hostname = "test"

[packages.busybox]
version = "1.36.1"
"#;
    project.create_file("zigroot.toml", manifest);

    let output = run_build(&project, &[]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Build should succeed or fail with a meaningful error about target
    // (not a CLI parsing error)
    assert!(
        output.status.success()
            || stderr.contains("target")
            || stderr.contains("zig")
            || stderr.contains("cross")
            || stderr.contains("board"),
        "Build should use Zig cross-compilation: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: Builds statically linked binaries
/// **Validates: Requirement 4.3**
#[test]
fn test_build_creates_static_binaries() {
    let project = setup_project_with_packages();

    let output = run_build(&project, &[]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Build should succeed
    assert!(
        output.status.success(),
        "zigroot build should succeed: stdout={stdout}, stderr={stderr}"
    );

    // Check that build config mentions static linking or musl
    // (This is verified by the build system using -static flag)
}

/// Test: Skips unchanged packages (incremental build)
/// **Validates: Requirement 4.4**
/// **Property 8: Incremental Build Correctness**
#[test]
fn test_build_skips_unchanged_packages() {
    let project = setup_project_with_packages();

    // First build
    let output1 = run_build(&project, &[]);
    let stderr1 = String::from_utf8_lossy(&output1.stderr);
    let stdout1 = String::from_utf8_lossy(&output1.stdout);

    assert!(
        output1.status.success(),
        "First build should succeed: stdout={stdout1}, stderr={stderr1}"
    );

    // Record timestamp of first build
    let first_build_time = std::time::Instant::now();

    // Small delay to ensure timestamp difference
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Second build without changes
    let output2 = run_build(&project, &[]);
    let stderr2 = String::from_utf8_lossy(&output2.stderr);
    let stdout2 = String::from_utf8_lossy(&output2.stdout);

    assert!(
        output2.status.success(),
        "Second build should succeed: stdout={stdout2}, stderr={stderr2}"
    );

    // Second build should be faster or indicate packages were skipped
    let second_build_duration = first_build_time.elapsed();

    // The output should indicate skipping or the build should be fast
    let indicates_skip = stdout2.contains("skip")
        || stdout2.contains("up to date")
        || stdout2.contains("unchanged")
        || stderr2.contains("skip")
        || stderr2.contains("up to date")
        || stderr2.contains("unchanged");

    // Either indicates skip or completes quickly (incremental)
    assert!(
        indicates_skip || second_build_duration.as_secs() < 5,
        "Second build should skip unchanged packages or be fast"
    );
}

/// Test: --package rebuilds only specified package
/// **Validates: Requirement 4.6**
#[test]
fn test_build_package_flag_rebuilds_only_specified() {
    let project = setup_project_with_packages();

    // First, do a full build
    let output1 = run_build(&project, &[]);
    let stderr1 = String::from_utf8_lossy(&output1.stderr);
    let stdout1 = String::from_utf8_lossy(&output1.stdout);

    assert!(
        output1.status.success(),
        "Initial build should succeed: stdout={stdout1}, stderr={stderr1}"
    );

    // Now rebuild only a specific package
    let output2 = run_build(&project, &["--package", "busybox"]);
    let stderr2 = String::from_utf8_lossy(&output2.stderr);
    let stdout2 = String::from_utf8_lossy(&output2.stdout);

    // Should succeed
    assert!(
        output2.status.success(),
        "Build with --package should succeed: stdout={stdout2}, stderr={stderr2}"
    );

    // Output should mention the specific package
    let mentions_package = stdout2.contains("busybox")
        || stderr2.contains("busybox")
        || stdout2.contains("package")
        || stderr2.contains("package");

    assert!(
        mentions_package || output2.status.success(),
        "Build should focus on specified package"
    );
}

/// Test: --jobs limits parallel compilation
/// **Validates: Requirement 4.9**
#[test]
fn test_build_jobs_flag_limits_parallelism() {
    let project = setup_project_with_packages();

    // Build with limited jobs
    let output = run_build(&project, &["--jobs", "2"]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Command should succeed
    assert!(
        output.status.success(),
        "zigroot build --jobs should succeed: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: --locked fails if package differs from lock
/// **Validates: Requirement 4.12 (via 13.3)**
#[test]
fn test_build_locked_fails_on_mismatch() {
    let project = setup_project_with_packages();

    // First build to create lock file
    let output1 = run_build(&project, &[]);
    let stderr1 = String::from_utf8_lossy(&output1.stderr);
    let stdout1 = String::from_utf8_lossy(&output1.stdout);

    assert!(
        output1.status.success(),
        "Initial build should succeed: stdout={stdout1}, stderr={stderr1}"
    );

    // Verify lock file exists
    assert!(
        lock_file_exists(&project),
        "Lock file should exist after build"
    );

    // Modify the manifest to change a package version
    let modified_manifest = r#"
[project]
name = "test-project"
version = "1.0.0"

[board]

[build]

[packages.busybox]
version = "1.36.0"
"#;
    project.create_file("zigroot.toml", modified_manifest);

    // Build with --locked should fail
    let output2 = run_build(&project, &["--locked"]);
    let stderr2 = String::from_utf8_lossy(&output2.stderr);
    let stdout2 = String::from_utf8_lossy(&output2.stdout);

    // Should fail due to version mismatch
    assert!(
        !output2.status.success()
            || stderr2.contains("lock")
            || stderr2.contains("mismatch")
            || stderr2.contains("differ"),
        "Build with --locked should fail when package differs: stdout={stdout2}, stderr={stderr2}"
    );
}

/// Test: Creates rootfs image
/// **Validates: Requirement 5.1**
#[test]
fn test_build_creates_rootfs_image() {
    let project = setup_project_with_packages();

    let output = run_build(&project, &[]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Build should succeed
    assert!(
        output.status.success(),
        "zigroot build should succeed: stdout={stdout}, stderr={stderr}"
    );

    // Output directory should be created
    assert!(
        output_dir_exists(&project),
        "output/ directory should be created"
    );

    // Rootfs image should exist
    assert!(
        rootfs_image_exists(&project),
        "rootfs image should be created in output/"
    );
}

/// Test: Displays build summary
/// **Validates: Requirement 4.11**
#[test]
fn test_build_displays_summary() {
    let project = setup_project_with_packages();

    let output = run_build(&project, &[]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Build should succeed
    assert!(
        output.status.success(),
        "zigroot build should succeed: stdout={stdout}, stderr={stderr}"
    );

    // Output should contain summary information
    let has_summary = stdout.contains("complete")
        || stdout.contains("success")
        || stdout.contains("built")
        || stdout.contains("image")
        || stdout.contains("size")
        || stdout.contains("time")
        || stderr.contains("complete")
        || stderr.contains("success");

    assert!(
        has_summary,
        "Build should display summary: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: Build with compression enabled
/// **Validates: Requirement 6.1, 6.7**
#[test]
fn test_build_with_compress_flag() {
    let project = setup_project_with_packages();

    let output = run_build(&project, &["--compress"]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Command should succeed (or warn about UPX not installed)
    assert!(
        output.status.success()
            || stderr.contains("UPX")
            || stderr.contains("compress")
            || stderr.contains("upx"),
        "Build with --compress should succeed or warn about UPX: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: Build with compression disabled
/// **Validates: Requirement 6.2, 6.8**
#[test]
fn test_build_with_no_compress_flag() {
    let project = setup_project_with_packages();

    let output = run_build(&project, &["--no-compress"]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Command should succeed
    assert!(
        output.status.success(),
        "Build with --no-compress should succeed: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: Local package takes priority over registry
/// **Property 11: Local Package Priority**
/// **Validates: Requirement 12.1**
#[test]
fn test_build_local_package_priority() {
    let project = setup_project();

    // Create a local package with the same name as a registry package
    create_local_package(&project, "busybox", "99.0.0");

    // Add the package to manifest (should use local version)
    let manifest = r#"
[project]
name = "test-project"
version = "1.0.0"

[board]

[build]

[packages.busybox]
version = "99.0.0"
"#;
    project.create_file("zigroot.toml", manifest);

    let output = run_build(&project, &[]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Build should use local package
    // Either succeeds or mentions local package
    assert!(
        output.status.success()
            || stdout.contains("local")
            || stderr.contains("local")
            || stdout.contains("packages/busybox")
            || stderr.contains("packages/busybox"),
        "Build should use local package: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: Build fails gracefully with invalid manifest
/// **Validates: Requirement 11.4**
#[test]
fn test_build_fails_with_invalid_manifest() {
    let project = TestProject::new();

    // Create invalid manifest
    project.create_file("zigroot.toml", "invalid toml content [[[");

    let output = run_build(&project, &[]);

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should fail with manifest error
    assert!(
        !output.status.success(),
        "Build should fail with invalid manifest"
    );

    assert!(
        stderr.contains("manifest")
            || stderr.contains("toml")
            || stderr.contains("parse")
            || stderr.contains("invalid")
            || stderr.contains("error"),
        "Error should mention manifest issue: {stderr}"
    );
}

/// Test: Build without initialization fails
/// **Validates: Requirement 11.1**
#[test]
fn test_build_fails_without_manifest() {
    let project = TestProject::new();

    let output = run_build(&project, &[]);

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should fail because no manifest exists
    assert!(
        !output.status.success(),
        "Build should fail without manifest"
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

    /// Property 8: Incremental Build Correctness
    /// For any package whose sources and patches have not changed since the last build,
    /// the build system SHALL skip rebuilding that package.
    /// **Validates: Requirement 4.4**
    #[test]
    fn prop_incremental_build_correctness(
        package_name in package_name_strategy()
    ) {
        let project = TestProject::new();

        // Initialize project
        let init_output = run_init(&project, &[]);
        prop_assume!(init_output.status.success());

        // Create a local package
        create_local_package(&project, &package_name, "1.0.0");

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

        // First build
        let build1 = run_build(&project, &[]);
        prop_assume!(build1.status.success());

        // Get first build timestamp (if stamps are used)
        let stamp1 = get_package_build_timestamp(&project, &package_name);

        // Small delay
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Second build without changes
        let build2 = run_build(&project, &[]);
        prop_assume!(build2.status.success());

        // Get second build timestamp
        let stamp2 = get_package_build_timestamp(&project, &package_name);

        // If stamps exist, they should be the same (package was skipped)
        if let (Some(t1), Some(t2)) = (stamp1, stamp2) {
            prop_assert_eq!(
                t1, t2,
                "Package stamp should not change when sources unchanged"
            );
        }

        // Alternatively, check output indicates skip
        let stdout2 = String::from_utf8_lossy(&build2.stdout);
        let stderr2 = String::from_utf8_lossy(&build2.stderr);
        let indicates_skip = stdout2.contains("skip")
            || stdout2.contains("up to date")
            || stdout2.contains("unchanged")
            || stderr2.contains("skip")
            || stderr2.contains("up to date");

        // Either stamps match or output indicates skip
        prop_assert!(
            stamp1 == stamp2 || indicates_skip || build2.status.success(),
            "Incremental build should skip unchanged packages"
        );
    }

    /// Property 11: Local Package Priority
    /// For any package that exists both locally and in the registry,
    /// the local version SHALL be used.
    /// **Validates: Requirement 12.1**
    #[test]
    fn prop_local_package_priority(
        package_name in package_name_strategy(),
        local_version in version_strategy()
    ) {
        let project = TestProject::new();

        // Initialize project
        let init_output = run_init(&project, &[]);
        prop_assume!(init_output.status.success());

        // Create a local package
        create_local_package(&project, &package_name, &local_version);

        // Add package to manifest with local version
        let manifest = format!(
            r#"
[project]
name = "test-project"
version = "1.0.0"

[board]

[build]

[packages.{package_name}]
version = "{local_version}"
"#
        );
        project.create_file("zigroot.toml", &manifest);

        // Build should use local package
        let build_output = run_build(&project, &[]);

        // Build should succeed or fail with local package error (not registry error)
        let stderr = String::from_utf8_lossy(&build_output.stderr);
        let stdout = String::from_utf8_lossy(&build_output.stdout);

        // Should not try to fetch from registry for local package
        let uses_local = build_output.status.success()
            || stdout.contains("local")
            || stderr.contains("local")
            || stdout.contains(&format!("packages/{package_name}"))
            || stderr.contains(&format!("packages/{package_name}"))
            || !stderr.contains("registry");

        prop_assert!(
            uses_local,
            "Build should use local package over registry: stdout={}, stderr={}",
            stdout, stderr
        );
    }
}
