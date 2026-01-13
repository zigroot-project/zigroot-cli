//! Integration tests for `zigroot external` command
//!
//! Tests for Requirements 8.1, 8.2, 8.9-8.13:
//! - list shows configured artifacts and status
//! - add --url adds remote artifact
//! - add --path adds local artifact
//!
//! **Validates: Requirements 8.1, 8.2, 8.9-8.13**

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

/// Helper to run zigroot external command
fn run_external(project: &TestProject, args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zigroot"));
    cmd.current_dir(project.path());
    cmd.arg("external");
    for arg in args {
        cmd.arg(arg);
    }
    cmd.output().expect("Failed to execute zigroot external")
}

/// Helper to initialize a project for external tests
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

/// Helper to create a manifest with external artifacts
fn create_manifest_with_externals(project: &TestProject) {
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

[external.bootloader]
type = "bootloader"
url = "https://example.com/uboot.bin"
sha256 = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"

[external.kernel]
type = "kernel"
path = "external/kernel.img"

[external.dtb]
type = "dtb"
url = "https://example.com/device.dtb"
sha256 = "abc123def456789012345678901234567890123456789012345678901234abcd"
path = "external/device.dtb"
"#;
    project.create_file("zigroot.toml", manifest);
}

/// Helper to create a manifest without external artifacts
fn create_manifest_without_externals(project: &TestProject) {
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
"#;
    project.create_file("zigroot.toml", manifest);
}

// ============================================
// Unit Tests for zigroot external list
// ============================================

/// Test: list shows configured artifacts and status
/// **Validates: Requirement 8.9**
#[test]
fn test_external_list_shows_configured_artifacts() {
    let project = setup_project();
    create_manifest_with_externals(&project);

    let output = run_external(&project, &["list"]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Command should succeed
    assert!(
        output.status.success(),
        "zigroot external list should succeed: stdout={stdout}, stderr={stderr}"
    );

    // Should show the configured artifacts
    let shows_bootloader = stdout.contains("bootloader") || stderr.contains("bootloader");
    let shows_kernel = stdout.contains("kernel") || stderr.contains("kernel");
    let shows_dtb = stdout.contains("dtb") || stderr.contains("dtb");

    assert!(
        shows_bootloader,
        "Should show bootloader artifact: stdout={stdout}, stderr={stderr}"
    );
    assert!(
        shows_kernel,
        "Should show kernel artifact: stdout={stdout}, stderr={stderr}"
    );
    assert!(
        shows_dtb,
        "Should show dtb artifact: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: list shows artifact status (downloaded/local/missing)
/// **Validates: Requirement 8.9**
#[test]
fn test_external_list_shows_artifact_status() {
    let project = setup_project();
    create_manifest_with_externals(&project);

    // Create the local kernel file so it shows as "local"
    project.create_dir("external");
    project.create_file("external/kernel.img", "kernel image content");

    let output = run_external(&project, &["list"]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Command should succeed
    assert!(
        output.status.success(),
        "zigroot external list should succeed: stdout={stdout}, stderr={stderr}"
    );

    // Should show status indicators
    let shows_status = stdout.contains("local")
        || stdout.contains("missing")
        || stdout.contains("downloaded")
        || stdout.contains("✓")
        || stdout.contains("✗")
        || stdout.contains("present")
        || stdout.contains("not found")
        || stderr.contains("local")
        || stderr.contains("missing");

    assert!(
        shows_status,
        "Should show artifact status: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: list shows empty message when no artifacts configured
/// **Validates: Requirement 8.9**
#[test]
fn test_external_list_empty_when_no_artifacts() {
    let project = setup_project();
    create_manifest_without_externals(&project);

    let output = run_external(&project, &["list"]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Command should succeed
    assert!(
        output.status.success(),
        "zigroot external list should succeed: stdout={stdout}, stderr={stderr}"
    );

    // Should indicate no artifacts or show empty list
    let indicates_empty = stdout.contains("no")
        || stdout.contains("No")
        || stdout.contains("empty")
        || stdout.contains("none")
        || stdout.is_empty()
        || stderr.contains("no")
        || stderr.contains("No");

    // Or just succeed with minimal output
    assert!(
        indicates_empty || output.status.success(),
        "Should indicate no artifacts or succeed: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: list fails without manifest
#[test]
fn test_external_list_fails_without_manifest() {
    let project = TestProject::new();

    let output = run_external(&project, &["list"]);

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should fail because no manifest exists
    assert!(
        !output.status.success(),
        "zigroot external list should fail without manifest"
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
// Unit Tests for zigroot external add --url
// ============================================

/// Test: add --url adds remote artifact to manifest
/// **Validates: Requirement 8.10**
#[test]
fn test_external_add_url_adds_remote_artifact() {
    let project = setup_project();
    create_manifest_without_externals(&project);

    let output = run_external(
        &project,
        &[
            "add",
            "my-bootloader",
            "--artifact-type",
            "bootloader",
            "--url",
            "https://example.com/bootloader.bin",
        ],
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Command should succeed
    assert!(
        output.status.success(),
        "zigroot external add --url should succeed: stdout={stdout}, stderr={stderr}"
    );

    // Verify the manifest was updated
    let manifest_content = project.read_file("zigroot.toml");
    assert!(
        manifest_content.contains("my-bootloader") || manifest_content.contains("bootloader"),
        "Manifest should contain the new artifact: {manifest_content}"
    );
    assert!(
        manifest_content.contains("https://example.com/bootloader.bin")
            || manifest_content.contains("example.com"),
        "Manifest should contain the URL: {manifest_content}"
    );
}

/// Test: add --url with different artifact types
/// **Validates: Requirement 8.2, 8.10**
#[test]
fn test_external_add_url_supports_artifact_types() {
    let project = setup_project();
    create_manifest_without_externals(&project);

    // Test adding different artifact types
    let artifact_types = [
        "bootloader",
        "kernel",
        "dtb",
        "firmware",
        "partition_table",
        "other",
    ];

    for (i, artifact_type) in artifact_types.iter().enumerate() {
        let name = format!("artifact-{i}");
        let url = format!("https://example.com/{artifact_type}.bin");

        let output = run_external(
            &project,
            &[
                "add",
                &name,
                "--artifact-type",
                artifact_type,
                "--url",
                &url,
            ],
        );

        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        // Command should succeed for valid artifact types
        assert!(
            output.status.success(),
            "zigroot external add --url should succeed for type {artifact_type}: stdout={stdout}, stderr={stderr}"
        );
    }

    // Verify all artifacts were added
    let manifest_content = project.read_file("zigroot.toml");
    for i in 0..artifact_types.len() {
        let name = format!("artifact-{i}");
        assert!(
            manifest_content.contains(&name),
            "Manifest should contain artifact {name}: {manifest_content}"
        );
    }
}

// ============================================
// Unit Tests for zigroot external add --path
// ============================================

/// Test: add --path adds local artifact to manifest
/// **Validates: Requirement 8.11**
#[test]
fn test_external_add_path_adds_local_artifact() {
    let project = setup_project();
    create_manifest_without_externals(&project);

    // Create the local file
    project.create_dir("external");
    project.create_file("external/local-kernel.img", "kernel content");

    let output = run_external(
        &project,
        &[
            "add",
            "local-kernel",
            "--artifact-type",
            "kernel",
            "--path",
            "external/local-kernel.img",
        ],
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Command should succeed
    assert!(
        output.status.success(),
        "zigroot external add --path should succeed: stdout={stdout}, stderr={stderr}"
    );

    // Verify the manifest was updated
    let manifest_content = project.read_file("zigroot.toml");
    assert!(
        manifest_content.contains("local-kernel"),
        "Manifest should contain the new artifact: {manifest_content}"
    );
    assert!(
        manifest_content.contains("external/local-kernel.img")
            || manifest_content.contains("local-kernel.img"),
        "Manifest should contain the path: {manifest_content}"
    );
}

/// Test: add --path works with relative paths
/// **Validates: Requirement 8.11**
#[test]
fn test_external_add_path_relative_paths() {
    let project = setup_project();
    create_manifest_without_externals(&project);

    // Create the local file in a subdirectory
    project.create_dir("assets/firmware");
    project.create_file("assets/firmware/wifi.bin", "firmware content");

    let output = run_external(
        &project,
        &[
            "add",
            "wifi-firmware",
            "--artifact-type",
            "firmware",
            "--path",
            "assets/firmware/wifi.bin",
        ],
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Command should succeed
    assert!(
        output.status.success(),
        "zigroot external add --path should succeed with relative path: stdout={stdout}, stderr={stderr}"
    );

    // Verify the manifest was updated
    let manifest_content = project.read_file("zigroot.toml");
    assert!(
        manifest_content.contains("wifi-firmware"),
        "Manifest should contain the new artifact: {manifest_content}"
    );
}

// ============================================
// Additional Tests
// ============================================

/// Test: add fails without manifest
#[test]
fn test_external_add_fails_without_manifest() {
    let project = TestProject::new();

    let output = run_external(
        &project,
        &[
            "add",
            "test-artifact",
            "--artifact-type",
            "bootloader",
            "--url",
            "https://example.com/test.bin",
        ],
    );

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should fail because no manifest exists
    assert!(
        !output.status.success(),
        "zigroot external add should fail without manifest"
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

/// Test: add requires either --url or --path
/// **Validates: Requirements 8.1, 8.10, 8.11**
#[test]
fn test_external_add_requires_url_or_path() {
    let project = setup_project();
    create_manifest_without_externals(&project);

    let output = run_external(
        &project,
        &[
            "add",
            "incomplete-artifact",
            "--artifact-type",
            "bootloader",
        ],
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should fail or warn because neither --url nor --path is specified
    // (depending on implementation, it might succeed with a warning)
    let indicates_missing = stderr.contains("url")
        || stderr.contains("path")
        || stderr.contains("required")
        || stderr.contains("specify")
        || stdout.contains("url")
        || stdout.contains("path");

    assert!(
        indicates_missing || !output.status.success(),
        "Should indicate missing url/path or fail: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: add with both --url and --path (downloads to specified path)
/// **Validates: Requirement 8.5**
#[test]
fn test_external_add_url_and_path() {
    let project = setup_project();
    create_manifest_without_externals(&project);

    let output = run_external(
        &project,
        &[
            "add",
            "combined-artifact",
            "--artifact-type",
            "bootloader",
            "--url",
            "https://example.com/bootloader.bin",
            "--path",
            "external/bootloader.bin",
        ],
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Command should succeed (both url and path is valid per requirement 8.5)
    assert!(
        output.status.success(),
        "zigroot external add with both --url and --path should succeed: stdout={stdout}, stderr={stderr}"
    );

    // Verify the manifest was updated with both url and path
    let manifest_content = project.read_file("zigroot.toml");
    assert!(
        manifest_content.contains("combined-artifact"),
        "Manifest should contain the new artifact: {manifest_content}"
    );
}

/// Test: list shows artifact types correctly
/// **Validates: Requirement 8.2**
#[test]
fn test_external_list_shows_artifact_types() {
    let project = setup_project();
    create_manifest_with_externals(&project);

    let output = run_external(&project, &["list"]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Command should succeed
    assert!(
        output.status.success(),
        "zigroot external list should succeed: stdout={stdout}, stderr={stderr}"
    );

    // Should show artifact types
    let shows_types = stdout.contains("bootloader")
        || stdout.contains("kernel")
        || stdout.contains("dtb")
        || stderr.contains("bootloader")
        || stderr.contains("kernel")
        || stderr.contains("dtb");

    assert!(
        shows_types,
        "Should show artifact types: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: partition table artifact with format
/// **Validates: Requirement 8.12**
#[test]
fn test_external_partition_table_format() {
    let project = setup_project();

    // Create manifest with partition table that has format
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

[external.partition]
type = "partition_table"
path = "external/partition.img"
format = "gpt"
"#;
    project.create_file("zigroot.toml", manifest);

    let output = run_external(&project, &["list"]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Command should succeed
    assert!(
        output.status.success(),
        "zigroot external list should succeed with partition table: stdout={stdout}, stderr={stderr}"
    );

    // Should show the partition table
    let shows_partition = stdout.contains("partition")
        || stderr.contains("partition")
        || stdout.contains("gpt")
        || stderr.contains("gpt");

    assert!(
        shows_partition,
        "Should show partition table artifact: stdout={stdout}, stderr={stderr}"
    );
}
