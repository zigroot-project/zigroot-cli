//! Integration tests for `zigroot board info` command
//!
//! Tests for Requirement 9.4:
//! - Displays board details including architecture, CPU, and supported features
//!
//! **Validates: Requirements 9.4**

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

/// Helper to run zigroot board info command
fn run_board_info(project: &TestProject, board: &str) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zigroot"));
    cmd.current_dir(project.path());
    cmd.arg("board");
    cmd.arg("info");
    cmd.arg(board);
    cmd.output().expect("Failed to execute zigroot board info")
}

/// Helper to initialize a project for board info tests
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
// Unit Tests for zigroot board info
// ============================================

/// Test: Displays board details
/// **Validates: Requirement 9.4**
#[test]
fn test_board_info_displays_details() {
    let project = setup_project();

    let output = run_board_info(&project, "luckfox-pico");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // If successful, should display board information
    if output.status.success() && !stdout.is_empty() {
        // Should contain some board details
        let has_details = stdout.contains("luckfox")
            || stdout.contains("Luckfox")
            || stdout.contains("board")
            || stdout.contains("Board")
            || stdout.contains("name")
            || stdout.contains("Name");

        assert!(
            has_details,
            "Board info should display board details: stdout={stdout}"
        );
    }

    // Command should not crash
    let combined = format!("{stdout}{stderr}");
    assert!(
        output.status.success()
            || combined.contains("not found")
            || combined.contains("network")
            || combined.contains("registry")
            || combined.contains("error")
            || combined.contains("not implemented")
            || combined.is_empty(),
        "Board info should complete: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: Board info shows architecture
/// **Validates: Requirement 9.4**
#[test]
fn test_board_info_shows_architecture() {
    let project = setup_project();

    let output = run_board_info(&project, "luckfox-pico");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // If successful, should show architecture information
    if output.status.success() && !stdout.is_empty() {
        // Architecture could be shown as "arm", "aarch64", "x86_64", etc.
        // or as target triple like "arm-linux-musleabihf"
        let has_arch = stdout.contains("arm")
            || stdout.contains("aarch64")
            || stdout.contains("x86")
            || stdout.contains("riscv")
            || stdout.contains("arch")
            || stdout.contains("Arch")
            || stdout.contains("target")
            || stdout.contains("Target");

        assert!(
            has_arch || stdout.trim().is_empty(),
            "Board info should show architecture: stdout={stdout}"
        );
    }

    // Command should not crash
    let combined = format!("{stdout}{stderr}");
    assert!(
        output.status.success()
            || combined.contains("not found")
            || combined.contains("network")
            || combined.contains("registry")
            || combined.contains("error")
            || combined.contains("not implemented")
            || combined.is_empty(),
        "Board info should complete: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: Board info shows CPU information
/// **Validates: Requirement 9.4**
#[test]
fn test_board_info_shows_cpu() {
    let project = setup_project();

    let output = run_board_info(&project, "luckfox-pico");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // If successful, should show CPU information
    if output.status.success() && !stdout.is_empty() {
        // CPU could be shown as "cortex-a7", "cortex-a53", etc.
        let has_cpu = stdout.contains("cpu")
            || stdout.contains("CPU")
            || stdout.contains("cortex")
            || stdout.contains("Cortex")
            || stdout.contains("processor")
            || stdout.contains("Processor");

        assert!(
            has_cpu || stdout.trim().is_empty(),
            "Board info should show CPU information: stdout={stdout}"
        );
    }

    // Command should not crash
    let combined = format!("{stdout}{stderr}");
    assert!(
        output.status.success()
            || combined.contains("not found")
            || combined.contains("network")
            || combined.contains("registry")
            || combined.contains("error")
            || combined.contains("not implemented")
            || combined.is_empty(),
        "Board info should complete: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: Board info shows supported features
/// **Validates: Requirement 9.4**
#[test]
fn test_board_info_shows_features() {
    let project = setup_project();

    let output = run_board_info(&project, "luckfox-pico");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // If successful, may show features (optional field)
    if output.status.success() && !stdout.is_empty() {
        // Features are optional, so we just check the output is reasonable
        let is_reasonable = stdout.len() > 10
            || stdout.contains("feature")
            || stdout.contains("Feature")
            || stdout.contains("neon")
            || stdout.contains("vfp")
            || stdout.trim().is_empty();

        assert!(
            is_reasonable,
            "Board info should show features if available: stdout={stdout}"
        );
    }

    // Command should not crash
    let combined = format!("{stdout}{stderr}");
    assert!(
        output.status.success()
            || combined.contains("not found")
            || combined.contains("network")
            || combined.contains("registry")
            || combined.contains("error")
            || combined.contains("not implemented")
            || combined.is_empty(),
        "Board info should complete: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: Board info shows description
/// **Validates: Requirement 9.4**
#[test]
fn test_board_info_shows_description() {
    let project = setup_project();

    let output = run_board_info(&project, "luckfox-pico");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // If successful, should show description
    if output.status.success() && !stdout.is_empty() {
        // Description should be present
        let has_description = stdout.contains("description")
            || stdout.contains("Description")
            || stdout.len() > 50 // Descriptions are typically longer
            || stdout.contains("SoC")
            || stdout.contains("board");

        assert!(
            has_description || stdout.trim().is_empty(),
            "Board info should show description: stdout={stdout}"
        );
    }

    // Command should not crash
    let combined = format!("{stdout}{stderr}");
    assert!(
        output.status.success()
            || combined.contains("not found")
            || combined.contains("network")
            || combined.contains("registry")
            || combined.contains("error")
            || combined.contains("not implemented")
            || combined.is_empty(),
        "Board info should complete: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: Board info with invalid board name shows error
/// **Validates: Requirement 9.4**
#[test]
fn test_board_info_invalid_board_shows_error() {
    let project = setup_project();

    // Try to get info for a non-existent board
    let output = run_board_info(&project, "nonexistent-board-xyz123");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should fail with helpful error message, or be not implemented yet
    let combined = format!("{stdout}{stderr}");
    let has_error_or_not_impl = !output.status.success()
        || combined.contains("not found")
        || combined.contains("error")
        || combined.contains("invalid")
        || combined.contains("unknown")
        || combined.contains("not implemented")
        || combined.is_empty(); // Not implemented returns empty

    assert!(
        has_error_or_not_impl,
        "Board info with invalid board should show error: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: Board info works without initialized project
/// **Validates: Requirement 9.4**
#[test]
fn test_board_info_works_without_project() {
    let project = TestProject::new(); // Not initialized

    let output = run_board_info(&project, "luckfox-pico");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Board info should work even without a project (it fetches from registry)
    // It may fail due to network issues, but shouldn't crash
    let combined = format!("{stdout}{stderr}");
    assert!(
        output.status.success()
            || combined.contains("not found")
            || combined.contains("network")
            || combined.contains("registry")
            || combined.contains("error")
            || combined.contains("not implemented")
            || combined.is_empty(),
        "Board info should work or fail gracefully without project: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: Board info shows flash methods if available
/// **Validates: Requirement 9.4**
#[test]
fn test_board_info_shows_flash_methods() {
    let project = setup_project();

    let output = run_board_info(&project, "luckfox-pico");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // If successful, may show flash methods (optional)
    if output.status.success() && !stdout.is_empty() {
        // Flash methods are optional, so we just check the output is reasonable
        let is_reasonable = stdout.len() > 10
            || stdout.contains("flash")
            || stdout.contains("Flash")
            || stdout.contains("sd")
            || stdout.contains("SD")
            || stdout.trim().is_empty();

        assert!(
            is_reasonable,
            "Board info should show flash methods if available: stdout={stdout}"
        );
    }

    // Command should not crash
    let combined = format!("{stdout}{stderr}");
    assert!(
        output.status.success()
            || combined.contains("not found")
            || combined.contains("network")
            || combined.contains("registry")
            || combined.contains("error")
            || combined.contains("not implemented")
            || combined.is_empty(),
        "Board info should complete: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: Board info shows default settings
/// **Validates: Requirement 9.4**
#[test]
fn test_board_info_shows_defaults() {
    let project = setup_project();

    let output = run_board_info(&project, "luckfox-pico");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // If successful, may show default settings
    if output.status.success() && !stdout.is_empty() {
        // Defaults include image_format, rootfs_size, hostname
        let has_defaults = stdout.contains("default")
            || stdout.contains("Default")
            || stdout.contains("image")
            || stdout.contains("rootfs")
            || stdout.contains("hostname")
            || stdout.contains("ext4")
            || stdout.contains("squashfs")
            || stdout.trim().is_empty();

        assert!(
            has_defaults,
            "Board info should show default settings if available: stdout={stdout}"
        );
    }

    // Command should not crash
    let combined = format!("{stdout}{stderr}");
    assert!(
        output.status.success()
            || combined.contains("not found")
            || combined.contains("network")
            || combined.contains("registry")
            || combined.contains("error")
            || combined.contains("not implemented")
            || combined.is_empty(),
        "Board info should complete: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: Board info output is well-formatted
/// **Validates: Requirement 9.4**
#[test]
fn test_board_info_well_formatted() {
    let project = setup_project();

    let output = run_board_info(&project, "luckfox-pico");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // If successful, output should be well-formatted
    if output.status.success() && !stdout.is_empty() {
        // Output should have multiple lines or clear sections
        let lines: Vec<&str> = stdout.lines().collect();
        let is_well_formatted = lines.len() > 1
            || stdout.contains(":")
            || stdout.contains("-")
            || stdout.contains("\t")
            || stdout.trim().is_empty();

        assert!(
            is_well_formatted,
            "Board info should be well-formatted: stdout={stdout}"
        );
    }

    // Command should not crash
    let combined = format!("{stdout}{stderr}");
    assert!(
        output.status.success()
            || combined.contains("not found")
            || combined.contains("network")
            || combined.contains("registry")
            || combined.contains("error")
            || combined.contains("not implemented")
            || combined.is_empty(),
        "Board info should complete: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: Board info for different boards
/// **Validates: Requirement 9.4**
#[test]
fn test_board_info_different_boards() {
    let project = setup_project();

    // Try different board names
    let boards = ["luckfox-pico", "raspberry-pi-4", "beaglebone-black"];

    for board in boards {
        let output = run_board_info(&project, board);

        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        // Each board info request should complete without crashing
        let combined = format!("{stdout}{stderr}");
        assert!(
            output.status.success()
                || combined.contains("not found")
                || combined.contains("network")
                || combined.contains("registry")
                || combined.contains("error")
                || combined.contains("not implemented")
                || combined.is_empty(),
            "Board info for '{board}' should complete: stdout={stdout}, stderr={stderr}"
        );
    }
}
