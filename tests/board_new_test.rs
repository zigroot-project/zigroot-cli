//! Integration tests for `zigroot board new` command
//!
//! Tests for Requirement 29.1:
//! - Creates board template in boards/<name>/
//!
//! **Validates: Requirements 29.1**

mod common;

use common::TestProject;
use std::process::Command;

/// Helper to run zigroot board new command
fn run_board_new(project: &TestProject, name: &str) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zigroot"));
    cmd.current_dir(project.path());
    cmd.args(["board", "new", name]);
    cmd.output().expect("Failed to execute zigroot board new")
}

/// Helper to check if board template was created correctly
fn has_valid_board_template(project: &TestProject, name: &str) -> bool {
    let board_dir = project.path().join("boards").join(name);
    let board_toml_path = board_dir.join("board.toml");

    board_dir.is_dir() && board_toml_path.exists()
}

/// Helper to check if board.toml has required fields
fn has_valid_board_toml(project: &TestProject, name: &str) -> bool {
    let board_toml_path = project
        .path()
        .join("boards")
        .join(name)
        .join("board.toml");

    if !board_toml_path.exists() {
        return false;
    }

    let content = std::fs::read_to_string(&board_toml_path).unwrap_or_default();

    // Check for required fields per Requirement 29.3
    content.contains("[board]")
        && content.contains("name")
        && content.contains("description")
        && content.contains("target")
        && content.contains("cpu")
        && content.contains("[defaults]")
}

// ============================================
// Unit Tests for zigroot board new
// ============================================

/// Test: Creates board template in boards/<name>/
/// **Validates: Requirement 29.1**
#[test]
fn test_board_new_creates_template_directory() {
    let project = TestProject::new();

    // Initialize project first
    let init_output = Command::new(env!("CARGO_BIN_EXE_zigroot"))
        .current_dir(project.path())
        .arg("init")
        .output()
        .expect("Failed to run init");
    assert!(init_output.status.success(), "Init should succeed");

    let output = run_board_new(&project, "my-board");

    assert!(
        output.status.success(),
        "zigroot board new should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify board directory was created
    let board_dir = project.path().join("boards").join("my-board");
    assert!(
        board_dir.is_dir(),
        "boards/my-board/ directory should be created"
    );
}

/// Test: Creates board.toml file
/// **Validates: Requirement 29.1**
#[test]
fn test_board_new_creates_board_toml() {
    let project = TestProject::new();

    // Initialize project first
    let init_output = Command::new(env!("CARGO_BIN_EXE_zigroot"))
        .current_dir(project.path())
        .arg("init")
        .output()
        .expect("Failed to run init");
    assert!(init_output.status.success(), "Init should succeed");

    let output = run_board_new(&project, "test-board");

    assert!(
        output.status.success(),
        "zigroot board new should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify board.toml was created
    let board_toml_path = project
        .path()
        .join("boards")
        .join("test-board")
        .join("board.toml");
    assert!(
        board_toml_path.exists(),
        "board.toml should be created in boards/test-board/"
    );

    // Verify board.toml has required fields
    assert!(
        has_valid_board_toml(&project, "test-board"),
        "board.toml should contain required fields (name, description, target, cpu)"
    );
}

/// Test: Board name is used in generated files
/// **Validates: Requirement 29.1**
#[test]
fn test_board_new_uses_board_name() {
    let project = TestProject::new();

    // Initialize project first
    let init_output = Command::new(env!("CARGO_BIN_EXE_zigroot"))
        .current_dir(project.path())
        .arg("init")
        .output()
        .expect("Failed to run init");
    assert!(init_output.status.success(), "Init should succeed");

    let output = run_board_new(&project, "custom-board");

    assert!(
        output.status.success(),
        "zigroot board new should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify board name is in board.toml
    let board_content = project.read_file("boards/custom-board/board.toml");
    assert!(
        board_content.contains("custom-board"),
        "board.toml should contain the board name"
    );
}

/// Test: Complete board template is valid
/// **Validates: Requirement 29.1**
#[test]
fn test_board_new_creates_complete_template() {
    let project = TestProject::new();

    // Initialize project first
    let init_output = Command::new(env!("CARGO_BIN_EXE_zigroot"))
        .current_dir(project.path())
        .arg("init")
        .output()
        .expect("Failed to run init");
    assert!(init_output.status.success(), "Init should succeed");

    let output = run_board_new(&project, "complete-board");

    assert!(
        output.status.success(),
        "zigroot board new should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify complete template
    assert!(
        has_valid_board_template(&project, "complete-board"),
        "Board template should have all required files"
    );
}

/// Test: Fails gracefully when board already exists
/// **Validates: Requirement 29.1**
#[test]
fn test_board_new_fails_if_exists() {
    let project = TestProject::new();

    // Initialize project first
    let init_output = Command::new(env!("CARGO_BIN_EXE_zigroot"))
        .current_dir(project.path())
        .arg("init")
        .output()
        .expect("Failed to run init");
    assert!(init_output.status.success(), "Init should succeed");

    // Create board first time
    let output1 = run_board_new(&project, "existing-board");
    assert!(output1.status.success(), "First creation should succeed");

    // Try to create same board again
    let output2 = run_board_new(&project, "existing-board");

    // Should fail or warn about existing board
    let stderr = String::from_utf8_lossy(&output2.stderr);
    let stdout = String::from_utf8_lossy(&output2.stdout);

    assert!(
        !output2.status.success()
            || stderr.contains("exists")
            || stderr.contains("already")
            || stdout.contains("exists")
            || stdout.contains("already"),
        "Should fail or warn when board already exists: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: Works without initialized project (creates boards/ if needed)
/// **Validates: Requirement 29.1**
#[test]
fn test_board_new_without_init() {
    let project = TestProject::new();

    // Don't initialize - just run board new directly
    let output = run_board_new(&project, "standalone-board");

    // Should either succeed (creating boards/ dir) or fail with helpful message
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Either succeeds or gives helpful error
    assert!(
        output.status.success()
            || stderr.contains("init")
            || stderr.contains("zigroot.toml")
            || stderr.contains("project"),
        "Should succeed or give helpful error: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: Generated board.toml has defaults section
/// **Validates: Requirement 29.1**
#[test]
fn test_board_new_has_defaults_section() {
    let project = TestProject::new();

    // Initialize project first
    let init_output = Command::new(env!("CARGO_BIN_EXE_zigroot"))
        .current_dir(project.path())
        .arg("init")
        .output()
        .expect("Failed to run init");
    assert!(init_output.status.success(), "Init should succeed");

    let output = run_board_new(&project, "defaults-board");

    assert!(
        output.status.success(),
        "zigroot board new should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify board.toml has defaults section
    let board_content = project.read_file("boards/defaults-board/board.toml");
    assert!(
        board_content.contains("[defaults]"),
        "board.toml should contain [defaults] section"
    );
    assert!(
        board_content.contains("image_format"),
        "board.toml should contain image_format in defaults"
    );
    assert!(
        board_content.contains("rootfs_size"),
        "board.toml should contain rootfs_size in defaults"
    );
    assert!(
        board_content.contains("hostname"),
        "board.toml should contain hostname in defaults"
    );
}
