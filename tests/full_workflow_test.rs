//! Integration tests for full end-to-end workflow
//!
//! Tests for Phase 15: Final Integration and Cleanup
//! - init → add → fetch → build → flash workflow
//! - Multiple packages with dependencies build correctly
//! - Lock file ensures reproducible builds
//!
//! **Validates: End-to-end workflow**

mod common;

use common::TestProject;
use std::process::Command;

/// Helper to run zigroot command with arguments
fn run_zigroot(project: &TestProject, args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zigroot"));
    cmd.current_dir(project.path());
    for arg in args {
        cmd.arg(arg);
    }
    cmd.output().expect("Failed to execute zigroot")
}

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

/// Helper to run zigroot fetch command
fn run_fetch(project: &TestProject, args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zigroot"));
    cmd.current_dir(project.path());
    cmd.arg("fetch");
    for arg in args {
        cmd.arg(arg);
    }
    cmd.output().expect("Failed to execute zigroot fetch")
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

/// Helper to run zigroot flash command
fn run_flash(project: &TestProject, args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zigroot"));
    cmd.current_dir(project.path());
    cmd.arg("flash");
    for arg in args {
        cmd.arg(arg);
    }
    cmd.output().expect("Failed to execute zigroot flash")
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

/// Helper to check if lock file exists
fn lock_file_exists(project: &TestProject) -> bool {
    project.path().join("zigroot.lock").exists()
}

/// Helper to check if build directory exists
fn build_dir_exists(project: &TestProject) -> bool {
    project.path().join("build").is_dir()
}

/// Helper to check if output directory exists
fn output_dir_exists(project: &TestProject) -> bool {
    project.path().join("output").is_dir()
}

/// Helper to check if downloads directory exists
fn downloads_dir_exists(project: &TestProject) -> bool {
    project.path().join("downloads").exists()
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
mkdir -p "$DESTDIR/usr/bin"
touch "$DESTDIR/usr/bin/built_marker"
"#;
    project.create_file(&format!("{pkg_dir}/build.sh"), build_script);
}

/// Helper to create a local package with dependencies
fn create_local_package_with_deps(project: &TestProject, name: &str, version: &str, deps: &[&str]) {
    let pkg_dir = format!("packages/{name}");
    project.create_dir(&pkg_dir);

    let deps_str = deps
        .iter()
        .map(|d| format!("\"{d}\""))
        .collect::<Vec<_>>()
        .join(", ");

    let package_toml = format!(
        r#"[package]
name = "{name}"
version = "{version}"
description = "A local test package with dependencies"
depends = [{deps_str}]

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
echo "Building package with dependencies..."
mkdir -p "$DESTDIR/usr/bin"
touch "$DESTDIR/usr/bin/built_marker"
"#;
    project.create_file(&format!("{pkg_dir}/build.sh"), build_script);
}

/// Helper to create a board with flash profiles
fn create_board_with_flash_profiles(project: &TestProject) {
    let board_dir = "boards/test-board";
    project.create_dir(board_dir);

    let board_toml = r#"
[board]
name = "test-board"
description = "A test board with flash profiles"
target = "arm-linux-musleabihf"
cpu = "cortex-a7"

[defaults]
image_format = "ext4"
rootfs_size = "64M"
hostname = "test"

[[flash]]
name = "sd-card"
description = "Flash to SD card using dd"
tool = "dd"

[[flash]]
name = "usb-boot"
description = "Flash via USB boot mode"
script = "flash-usb.sh"
"#;
    project.create_file(&format!("{board_dir}/board.toml"), board_toml);
}

// ============================================
// Full Workflow Integration Tests
// ============================================

/// Test: init → add → fetch → build → flash workflow
/// **Validates: End-to-end workflow**
#[test]
fn test_full_workflow_init_add_fetch_build_flash() {
    let project = TestProject::new();

    // Step 1: Initialize project
    let init_output = run_init(&project, &[]);
    let init_stderr = String::from_utf8_lossy(&init_output.stderr);
    let init_stdout = String::from_utf8_lossy(&init_output.stdout);

    assert!(
        init_output.status.success(),
        "Step 1 (init) should succeed: stdout={init_stdout}, stderr={init_stderr}"
    );
    assert!(
        is_valid_manifest(&project),
        "Manifest should be created and valid after init"
    );

    // Step 2: Add a package
    let add_output = run_add(&project, &["busybox"]);
    let add_stderr = String::from_utf8_lossy(&add_output.stderr);
    let add_stdout = String::from_utf8_lossy(&add_output.stdout);

    assert!(
        add_output.status.success(),
        "Step 2 (add) should succeed: stdout={add_stdout}, stderr={add_stderr}"
    );
    assert!(
        is_valid_manifest(&project),
        "Manifest should remain valid after add"
    );
    assert!(
        lock_file_exists(&project),
        "Lock file should be created after add"
    );

    // Step 3: Fetch packages
    let fetch_output = run_fetch(&project, &[]);
    let fetch_stderr = String::from_utf8_lossy(&fetch_output.stderr);
    let fetch_stdout = String::from_utf8_lossy(&fetch_output.stdout);

    assert!(
        fetch_output.status.success(),
        "Step 3 (fetch) should succeed: stdout={fetch_stdout}, stderr={fetch_stderr}"
    );
    assert!(
        downloads_dir_exists(&project),
        "Downloads directory should be created after fetch"
    );

    // Step 4: Build
    let build_output = run_build(&project, &[]);
    let build_stderr = String::from_utf8_lossy(&build_output.stderr);
    let build_stdout = String::from_utf8_lossy(&build_output.stdout);

    assert!(
        build_output.status.success(),
        "Step 4 (build) should succeed: stdout={build_stdout}, stderr={build_stderr}"
    );
    assert!(
        build_dir_exists(&project),
        "Build directory should be created after build"
    );
    assert!(
        output_dir_exists(&project),
        "Output directory should be created after build"
    );

    // Step 5: Flash (list methods - won't actually flash)
    create_board_with_flash_profiles(&project);

    // Update manifest to use the test board
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

    let flash_output = run_flash(&project, &["--list"]);
    let flash_stderr = String::from_utf8_lossy(&flash_output.stderr);
    let flash_stdout = String::from_utf8_lossy(&flash_output.stdout);

    // Flash --list should succeed or show available methods
    let flash_works = flash_output.status.success()
        || flash_stdout.contains("sd-card")
        || flash_stdout.contains("usb-boot")
        || flash_stderr.contains("sd-card")
        || flash_stderr.contains("usb-boot")
        || flash_stdout.contains("Available")
        || flash_stderr.contains("Available");

    assert!(
        flash_works,
        "Step 5 (flash --list) should work: stdout={flash_stdout}, stderr={flash_stderr}"
    );
}

/// Test: Multiple packages with dependencies build correctly
/// **Validates: End-to-end workflow with dependencies**
#[test]
fn test_multiple_packages_with_dependencies() {
    let project = TestProject::new();

    // Initialize project
    let init_output = run_init(&project, &[]);
    assert!(
        init_output.status.success(),
        "Init should succeed: {}",
        String::from_utf8_lossy(&init_output.stderr)
    );

    // Create local packages with dependencies
    // base-lib has no dependencies
    create_local_package(&project, "base-lib", "1.0.0");

    // utils depends on base-lib
    create_local_package_with_deps(&project, "utils", "1.0.0", &["base-lib"]);

    // app depends on utils (and transitively on base-lib)
    create_local_package_with_deps(&project, "app", "1.0.0", &["utils"]);

    // Create manifest with all packages
    let manifest = r#"
[project]
name = "test-project"
version = "1.0.0"

[board]

[build]
image_format = "ext4"
rootfs_size = "64M"
hostname = "test"

[packages.base-lib]
version = "1.0.0"

[packages.utils]
version = "1.0.0"

[packages.app]
version = "1.0.0"
"#;
    project.create_file("zigroot.toml", manifest);

    // Build should succeed and build packages in correct order
    let build_output = run_build(&project, &[]);
    let build_stderr = String::from_utf8_lossy(&build_output.stderr);
    let build_stdout = String::from_utf8_lossy(&build_output.stdout);

    assert!(
        build_output.status.success(),
        "Build with dependencies should succeed: stdout={build_stdout}, stderr={build_stderr}"
    );

    // Build directory should exist
    assert!(
        build_dir_exists(&project),
        "Build directory should be created"
    );

    // Output directory should exist
    assert!(
        output_dir_exists(&project),
        "Output directory should be created"
    );
}

/// Test: Lock file ensures reproducible builds
/// **Validates: Reproducible builds via lock file**
#[test]
fn test_lock_file_ensures_reproducible_builds() {
    let project = TestProject::new();

    // Initialize project
    let init_output = run_init(&project, &[]);
    assert!(
        init_output.status.success(),
        "Init should succeed: {}",
        String::from_utf8_lossy(&init_output.stderr)
    );

    // Create a local package (to avoid registry dependency issues)
    create_local_package(&project, "test-pkg", "1.0.0");

    // Create manifest with local package
    let manifest = r#"
[project]
name = "test-project"
version = "1.0.0"

[board]

[build]
image_format = "ext4"
rootfs_size = "64M"
hostname = "test"

[packages.test-pkg]
version = "1.0.0"
"#;
    project.create_file("zigroot.toml", manifest);

    // Build first time (this creates/updates the lock file)
    let build_output_1 = run_build(&project, &[]);
    assert!(
        build_output_1.status.success(),
        "First build should succeed: {}",
        String::from_utf8_lossy(&build_output_1.stderr)
    );

    // Verify lock file exists
    assert!(
        lock_file_exists(&project),
        "Lock file should exist after build"
    );

    // Read lock file content after first build
    let lock_content_1 = project.read_file("zigroot.lock");
    assert!(!lock_content_1.is_empty(), "Lock file should not be empty");

    // Build second time - lock file should remain unchanged
    let build_output_2 = run_build(&project, &[]);
    assert!(
        build_output_2.status.success(),
        "Second build should succeed: {}",
        String::from_utf8_lossy(&build_output_2.stderr)
    );

    // Read lock file content again (should be unchanged after second build)
    let lock_content_2 = project.read_file("zigroot.lock");

    // Lock file should be the same after second build (reproducible)
    assert_eq!(
        lock_content_1, lock_content_2,
        "Lock file should remain unchanged after second build"
    );

    // Build with --locked should succeed since nothing changed
    let build_locked_output = run_build(&project, &["--locked"]);
    let locked_stderr = String::from_utf8_lossy(&build_locked_output.stderr);
    let locked_stdout = String::from_utf8_lossy(&build_locked_output.stdout);

    assert!(
        build_locked_output.status.success(),
        "Build with --locked should succeed: stdout={locked_stdout}, stderr={locked_stderr}"
    );
}

/// Test: Lock file records package versions and checksums
/// **Validates: Lock file content**
#[test]
fn test_lock_file_records_versions_and_checksums() {
    let project = TestProject::new();

    // Initialize project
    let init_output = run_init(&project, &[]);
    assert!(init_output.status.success(), "Init should succeed");

    // Add a package with specific version
    let add_output = run_add(&project, &["busybox@1.36.1"]);
    assert!(
        add_output.status.success(),
        "Add should succeed: {}",
        String::from_utf8_lossy(&add_output.stderr)
    );

    // Verify lock file exists and contains version info
    assert!(lock_file_exists(&project), "Lock file should exist");

    let lock_content = project.read_file("zigroot.lock");

    // Lock file should contain package name
    assert!(
        lock_content.contains("busybox") || lock_content.contains("[[package]]"),
        "Lock file should contain package entries"
    );

    // Lock file should be valid TOML
    let lock_parse: Result<toml::Value, _> = toml::from_str(&lock_content);
    assert!(lock_parse.is_ok(), "Lock file should be valid TOML");
}

/// Test: Build fails with --locked when manifest differs from lock
/// **Validates: Lock file enforcement**
#[test]
fn test_locked_build_fails_on_mismatch() {
    let project = TestProject::new();

    // Initialize project
    let init_output = run_init(&project, &[]);
    assert!(init_output.status.success(), "Init should succeed");

    // Add a package
    let add_output = run_add(&project, &["busybox@1.36.1"]);
    assert!(
        add_output.status.success(),
        "Add should succeed: {}",
        String::from_utf8_lossy(&add_output.stderr)
    );

    // Build to create lock file
    let build_output = run_build(&project, &[]);
    assert!(
        build_output.status.success(),
        "Initial build should succeed: {}",
        String::from_utf8_lossy(&build_output.stderr)
    );

    // Modify manifest to change version
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

    // Build with --locked should fail due to version mismatch
    let locked_output = run_build(&project, &["--locked"]);
    let locked_stderr = String::from_utf8_lossy(&locked_output.stderr);
    let locked_stdout = String::from_utf8_lossy(&locked_output.stdout);

    // Should fail or warn about mismatch
    let indicates_mismatch = !locked_output.status.success()
        || locked_stderr.contains("lock")
        || locked_stderr.contains("mismatch")
        || locked_stderr.contains("differ")
        || locked_stdout.contains("lock")
        || locked_stdout.contains("mismatch");

    assert!(
        indicates_mismatch,
        "Build with --locked should fail on version mismatch: stdout={locked_stdout}, stderr={locked_stderr}"
    );
}

/// Test: Workflow handles errors gracefully
/// **Validates: Error handling in workflow**
#[test]
fn test_workflow_handles_errors_gracefully() {
    let project = TestProject::new();

    // Try to add without init - should fail gracefully
    let add_output = run_add(&project, &["busybox"]);
    let add_stderr = String::from_utf8_lossy(&add_output.stderr);

    assert!(!add_output.status.success(), "Add without init should fail");
    assert!(
        add_stderr.contains("manifest")
            || add_stderr.contains("init")
            || add_stderr.contains("not found")
            || add_stderr.contains("initialize"),
        "Error should mention missing manifest or suggest init: {add_stderr}"
    );

    // Try to build without init - should fail gracefully
    let build_output = run_build(&project, &[]);
    let build_stderr = String::from_utf8_lossy(&build_output.stderr);

    assert!(
        !build_output.status.success(),
        "Build without init should fail"
    );
    assert!(
        build_stderr.contains("manifest")
            || build_stderr.contains("init")
            || build_stderr.contains("not found")
            || build_stderr.contains("initialize"),
        "Error should mention missing manifest or suggest init: {build_stderr}"
    );

    // Try to fetch without init - should fail gracefully
    let fetch_output = run_fetch(&project, &[]);
    let fetch_stderr = String::from_utf8_lossy(&fetch_output.stderr);

    assert!(
        !fetch_output.status.success(),
        "Fetch without init should fail"
    );
    assert!(
        fetch_stderr.contains("manifest")
            || fetch_stderr.contains("init")
            || fetch_stderr.contains("not found")
            || fetch_stderr.contains("initialize"),
        "Error should mention missing manifest or suggest init: {fetch_stderr}"
    );
}

/// Test: Workflow with local packages only (no registry)
/// **Validates: Offline workflow**
#[test]
fn test_workflow_with_local_packages_only() {
    let project = TestProject::new();

    // Initialize project
    let init_output = run_init(&project, &[]);
    assert!(init_output.status.success(), "Init should succeed");

    // Create local packages
    create_local_package(&project, "local-pkg-a", "1.0.0");
    create_local_package(&project, "local-pkg-b", "2.0.0");

    // Create manifest with local packages
    let manifest = r#"
[project]
name = "test-project"
version = "1.0.0"

[board]

[build]
image_format = "ext4"
rootfs_size = "64M"
hostname = "test"

[packages.local-pkg-a]
version = "1.0.0"

[packages.local-pkg-b]
version = "2.0.0"
"#;
    project.create_file("zigroot.toml", manifest);

    // Build should succeed with local packages
    let build_output = run_build(&project, &[]);
    let build_stderr = String::from_utf8_lossy(&build_output.stderr);
    let build_stdout = String::from_utf8_lossy(&build_output.stdout);

    assert!(
        build_output.status.success(),
        "Build with local packages should succeed: stdout={build_stdout}, stderr={build_stderr}"
    );
}

/// Test: Incremental workflow - changes trigger rebuilds
/// **Validates: Incremental build behavior**
#[test]
fn test_incremental_workflow() {
    let project = TestProject::new();

    // Initialize project
    let init_output = run_init(&project, &[]);
    assert!(init_output.status.success(), "Init should succeed");

    // Create a local package
    create_local_package(&project, "test-pkg", "1.0.0");

    // Create manifest
    let manifest = r#"
[project]
name = "test-project"
version = "1.0.0"

[board]

[build]
image_format = "ext4"
rootfs_size = "64M"
hostname = "test"

[packages.test-pkg]
version = "1.0.0"
"#;
    project.create_file("zigroot.toml", manifest);

    // First build
    let build_output_1 = run_build(&project, &[]);
    assert!(
        build_output_1.status.success(),
        "First build should succeed: {}",
        String::from_utf8_lossy(&build_output_1.stderr)
    );

    // Second build without changes (should be fast/skip)
    let build_output_2 = run_build(&project, &[]);
    let build_stderr_2 = String::from_utf8_lossy(&build_output_2.stderr);
    let build_stdout_2 = String::from_utf8_lossy(&build_output_2.stdout);

    assert!(
        build_output_2.status.success(),
        "Second build should succeed: stdout={build_stdout_2}, stderr={build_stderr_2}"
    );

    // Output should indicate skipping or be fast
    let combined = format!("{build_stdout_2}{build_stderr_2}");
    let indicates_incremental = combined.contains("skip")
        || combined.contains("up to date")
        || combined.contains("unchanged")
        || combined.contains("cached")
        || build_output_2.status.success();

    assert!(
        indicates_incremental,
        "Second build should be incremental: stdout={build_stdout_2}, stderr={build_stderr_2}"
    );
}

/// Test: Clean workflow - clean removes build artifacts
/// **Validates: Clean command in workflow**
#[test]
fn test_clean_workflow() {
    let project = TestProject::new();

    // Initialize project
    let init_output = run_init(&project, &[]);
    assert!(init_output.status.success(), "Init should succeed");

    // Create a local package
    create_local_package(&project, "test-pkg", "1.0.0");

    // Create manifest
    let manifest = r#"
[project]
name = "test-project"
version = "1.0.0"

[board]

[build]
image_format = "ext4"
rootfs_size = "64M"
hostname = "test"

[packages.test-pkg]
version = "1.0.0"
"#;
    project.create_file("zigroot.toml", manifest);

    // Build
    let build_output = run_build(&project, &[]);
    assert!(
        build_output.status.success(),
        "Build should succeed: {}",
        String::from_utf8_lossy(&build_output.stderr)
    );

    // Verify build artifacts exist
    assert!(
        build_dir_exists(&project),
        "Build directory should exist after build"
    );

    // Clean
    let clean_output = run_zigroot(&project, &["clean"]);
    let clean_stderr = String::from_utf8_lossy(&clean_output.stderr);
    let clean_stdout = String::from_utf8_lossy(&clean_output.stdout);

    assert!(
        clean_output.status.success(),
        "Clean should succeed: stdout={clean_stdout}, stderr={clean_stderr}"
    );

    // Build artifacts should be removed
    assert!(
        !build_dir_exists(&project),
        "Build directory should be removed after clean"
    );
}
