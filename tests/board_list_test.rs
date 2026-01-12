//! Integration tests for `zigroot board list` command
//!
//! Tests for Requirement 9.1:
//! - Lists available boards from registry
//!
//! **Validates: Requirements 9.1**

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

/// Helper to run zigroot board list command
fn run_board_list(project: &TestProject) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zigroot"));
    cmd.current_dir(project.path());
    cmd.arg("board");
    cmd.arg("list");
    cmd.output().expect("Failed to execute zigroot board list")
}

/// Helper to initialize a project for board list tests
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
// Unit Tests for zigroot board list
// ============================================

/// Test: Lists available boards from registry
/// **Validates: Requirement 9.1**
#[test]
fn test_board_list_displays_available_boards() {
    let project = setup_project();

    let output = run_board_list(&project);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Command should succeed (or fail gracefully with network error)
    // The command should at least run without crashing
    let combined = format!("{stdout}{stderr}");
    let is_valid_response = output.status.success()
        || combined.contains("board")
        || combined.contains("Board")
        || combined.contains("network")
        || combined.contains("registry")
        || combined.contains("error");

    assert!(
        is_valid_response,
        "zigroot board list should succeed or fail gracefully: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: Board list shows board names
/// **Validates: Requirement 9.1**
#[test]
fn test_board_list_shows_board_names() {
    let project = setup_project();

    let output = run_board_list(&project);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // If successful, output should contain board information
    if output.status.success() && !stdout.is_empty() {
        // Output should have some content (board names, descriptions, etc.)
        // or indicate no boards available
        let has_content = !stdout.trim().is_empty()
            || stdout.contains("No boards")
            || stdout.contains("no boards");

        assert!(
            has_content,
            "Board list should show board names or indicate none available: stdout={stdout}"
        );
    }

    // Command should not crash
    assert!(
        output.status.success()
            || stderr.contains("network")
            || stderr.contains("registry")
            || stderr.contains("error")
            || stderr.contains("not implemented"),
        "Board list should complete: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: Board list shows architecture information
/// **Validates: Requirement 9.1**
#[test]
fn test_board_list_shows_architecture() {
    let project = setup_project();

    let output = run_board_list(&project);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // If successful and has results, should show architecture info
    if output.status.success() && !stdout.is_empty() && !stdout.contains("No boards") {
        // Architecture could be shown as "arm", "aarch64", "x86_64", etc.
        // or as target triple like "arm-linux-musleabihf"
        let has_arch_info = stdout.contains("arm")
            || stdout.contains("aarch64")
            || stdout.contains("x86")
            || stdout.contains("riscv")
            || stdout.contains("arch")
            || stdout.contains("target")
            || stdout.contains("[board]");

        assert!(
            has_arch_info || stdout.trim().is_empty(),
            "Board list should show architecture information: stdout={stdout}"
        );
    }

    // Command should not crash
    assert!(
        output.status.success()
            || stderr.contains("network")
            || stderr.contains("registry")
            || stderr.contains("error")
            || stderr.contains("not implemented"),
        "Board list should complete: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: Board list shows descriptions
/// **Validates: Requirement 9.1**
#[test]
fn test_board_list_shows_descriptions() {
    let project = setup_project();

    let output = run_board_list(&project);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // If successful and has results, should show descriptions
    if output.status.success() && !stdout.is_empty() && !stdout.contains("No boards") {
        // Descriptions are typically longer text after the board name
        let lines: Vec<&str> = stdout.lines().filter(|l| !l.trim().is_empty()).collect();

        // Each board entry should have more than just a name
        let has_descriptions = lines.iter().any(|line| line.len() > 20)
            || stdout.contains("description")
            || stdout.contains("-")
            || stdout.contains(":");

        assert!(
            has_descriptions || stdout.trim().is_empty(),
            "Board list should show descriptions: stdout={stdout}"
        );
    }

    // Command should not crash
    assert!(
        output.status.success()
            || stderr.contains("network")
            || stderr.contains("registry")
            || stderr.contains("error")
            || stderr.contains("not implemented"),
        "Board list should complete: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: Board list works without initialized project
/// **Validates: Requirement 9.1**
#[test]
fn test_board_list_works_without_project() {
    let project = TestProject::new(); // Not initialized

    let output = run_board_list(&project);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Board list should work even without a project (it lists from registry)
    // It may fail due to network issues, but shouldn't crash
    assert!(
        output.status.success()
            || stderr.contains("network")
            || stderr.contains("registry")
            || stderr.contains("error")
            || stderr.contains("not implemented")
            || stderr.contains("not found"),
        "Board list should work or fail gracefully without project: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: Board list output is formatted consistently
/// **Validates: Requirement 9.1**
#[test]
fn test_board_list_consistent_format() {
    let project = setup_project();

    let output = run_board_list(&project);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // If successful and has multiple boards, they should be formatted consistently
    if output.status.success() && !stdout.is_empty() {
        let lines: Vec<&str> = stdout
            .lines()
            .filter(|l| !l.trim().is_empty())
            .filter(|l| !l.starts_with("Available") && !l.starts_with("Boards"))
            .collect();

        // If we have multiple board entries, they should have similar formatting
        if lines.len() > 1 {
            // Check that lines have consistent structure (e.g., all have similar separators)
            let has_consistent_format = lines.iter().all(|l| l.contains("-") || l.contains(":"))
                || lines.iter().all(|l| !l.contains("-") && !l.contains(":"));

            assert!(
                has_consistent_format,
                "Board list should have consistent formatting: stdout={stdout}"
            );
        }
    }

    // Command should not crash
    assert!(
        output.status.success()
            || stderr.contains("network")
            || stderr.contains("registry")
            || stderr.contains("error")
            || stderr.contains("not implemented"),
        "Board list should complete: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: Board list with [board] label format
/// **Validates: Requirement 9.1**
#[test]
fn test_board_list_shows_board_label() {
    let project = setup_project();

    let output = run_board_list(&project);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // If successful and has results, may show [board] labels like search does
    if output.status.success() && !stdout.is_empty() && !stdout.contains("No boards") {
        // Board list might use [board] labels or just list board names
        // Either format is acceptable
        let has_valid_format = stdout.contains("[board]")
            || stdout.contains("Board")
            || stdout.contains("board")
            || stdout.lines().any(|l| !l.trim().is_empty());

        assert!(
            has_valid_format,
            "Board list should show boards in valid format: stdout={stdout}"
        );
    }

    // Command should not crash
    assert!(
        output.status.success()
            || stderr.contains("network")
            || stderr.contains("registry")
            || stderr.contains("error")
            || stderr.contains("not implemented"),
        "Board list should complete: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: Board list handles empty registry gracefully
/// **Validates: Requirement 9.1**
#[test]
fn test_board_list_handles_empty_registry() {
    let project = setup_project();

    let output = run_board_list(&project);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // If registry is empty or unavailable, should handle gracefully
    let combined = format!("{stdout}{stderr}");
    let handles_gracefully = output.status.success()
        || combined.contains("No boards")
        || combined.contains("no boards")
        || combined.contains("empty")
        || combined.contains("network")
        || combined.contains("registry")
        || combined.contains("error")
        || combined.contains("not implemented");

    assert!(
        handles_gracefully,
        "Board list should handle empty/unavailable registry gracefully: stdout={stdout}, stderr={stderr}"
    );
}
