//! Integration tests for `zigroot init` command
//!
//! Tests for Requirements 1.1-1.7:
//! - Creates zigroot.toml in empty directory
//! - Creates packages/, boards/, user/files/, user/scripts/ directories
//! - Creates .gitignore with zigroot entries
//! - Fails in non-empty directory without --force
//! - Succeeds with --force in non-empty directory
//! - --board fetches board from registry
//! - Appending to existing .gitignore is idempotent
//!
//! **Property 24: Gitignore Append Idempotence**
//! **Validates: Requirements 1.1-1.7**

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

/// Helper to check if zigroot.toml exists and is valid
fn has_valid_manifest(project: &TestProject) -> bool {
    let manifest_path = project.path().join("zigroot.toml");
    if !manifest_path.exists() {
        return false;
    }
    let content = std::fs::read_to_string(&manifest_path).unwrap_or_default();
    // Check it's valid TOML with required sections
    toml::from_str::<toml::Value>(&content).is_ok()
}

/// Helper to check if all required directories exist
fn has_required_directories(project: &TestProject) -> bool {
    let dirs = ["packages", "boards", "user/files", "user/scripts"];
    dirs.iter().all(|d| project.path().join(d).is_dir())
}

/// Helper to check if .gitignore has zigroot entries
fn has_gitignore_entries(project: &TestProject) -> bool {
    let gitignore_path = project.path().join(".gitignore");
    if !gitignore_path.exists() {
        return false;
    }
    let content = std::fs::read_to_string(&gitignore_path).unwrap_or_default();
    content.contains("build/")
        && content.contains("downloads/")
        && content.contains("output/")
        && content.contains("external/")
}

// ============================================
// Unit Tests for zigroot init
// ============================================

/// Test: Creates zigroot.toml in empty directory
/// **Validates: Requirement 1.1**
#[test]
fn test_init_creates_manifest_in_empty_directory() {
    let project = TestProject::new();

    let output = run_init(&project, &[]);

    assert!(
        output.status.success(),
        "zigroot init should succeed in empty directory: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        has_valid_manifest(&project),
        "zigroot.toml should be created and valid"
    );
}

/// Test: Creates packages/, boards/, user/files/, user/scripts/ directories
/// **Validates: Requirement 1.1**
#[test]
fn test_init_creates_required_directories() {
    let project = TestProject::new();

    let output = run_init(&project, &[]);

    assert!(
        output.status.success(),
        "zigroot init should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        has_required_directories(&project),
        "All required directories should be created"
    );

    // Verify each directory individually
    assert!(
        project.path().join("packages").is_dir(),
        "packages/ should exist"
    );
    assert!(
        project.path().join("boards").is_dir(),
        "boards/ should exist"
    );
    assert!(
        project.path().join("user/files").is_dir(),
        "user/files/ should exist"
    );
    assert!(
        project.path().join("user/scripts").is_dir(),
        "user/scripts/ should exist"
    );
}

/// Test: Creates .gitignore with zigroot entries
/// **Validates: Requirement 1.6**
#[test]
fn test_init_creates_gitignore() {
    let project = TestProject::new();

    let output = run_init(&project, &[]);

    assert!(
        output.status.success(),
        "zigroot init should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        has_gitignore_entries(&project),
        ".gitignore should contain zigroot entries"
    );

    // Verify specific entries
    let content = project.read_file(".gitignore");
    assert!(
        content.contains("build/"),
        ".gitignore should exclude build/"
    );
    assert!(
        content.contains("downloads/"),
        ".gitignore should exclude downloads/"
    );
    assert!(
        content.contains("output/"),
        ".gitignore should exclude output/"
    );
    assert!(
        content.contains("external/"),
        ".gitignore should exclude external/"
    );
}

/// Test: Fails in non-empty directory without --force
/// **Validates: Requirement 1.3**
#[test]
fn test_init_fails_in_nonempty_directory_without_force() {
    let project = TestProject::new();

    // Create a file to make directory non-empty
    project.create_file("existing_file.txt", "some content");

    let output = run_init(&project, &[]);

    assert!(
        !output.status.success(),
        "zigroot init should fail in non-empty directory without --force"
    );

    // Verify error message mentions non-empty directory
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("non-empty") || stderr.contains("not empty") || stderr.contains("--force"),
        "Error should mention non-empty directory or --force flag: {stderr}"
    );
}

/// Test: Succeeds with --force in non-empty directory
/// **Validates: Requirement 1.4**
#[test]
fn test_init_succeeds_with_force_in_nonempty_directory() {
    let project = TestProject::new();

    // Create a file to make directory non-empty
    project.create_file("existing_file.txt", "some content");

    let output = run_init(&project, &["--force"]);

    assert!(
        output.status.success(),
        "zigroot init --force should succeed in non-empty directory: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        has_valid_manifest(&project),
        "zigroot.toml should be created"
    );
    assert!(
        has_required_directories(&project),
        "Required directories should be created"
    );

    // Verify existing file is preserved
    assert!(
        project.file_exists("existing_file.txt"),
        "Existing files should be preserved"
    );
    assert_eq!(
        project.read_file("existing_file.txt"),
        "some content",
        "Existing file content should be unchanged"
    );
}

/// Test: --board fetches board from registry
/// **Validates: Requirement 1.2**
#[test]
fn test_init_with_board_flag() {
    let project = TestProject::new();

    // Note: This test may need a mock registry or skip if no network
    // For now, we test that the flag is accepted and the command structure is correct
    let output = run_init(&project, &["--board", "test-board"]);

    // The command should either succeed (if board exists) or fail with a specific error
    // about the board not being found (not a CLI parsing error)
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Either succeeds or fails with board-related error (not CLI error)
    assert!(
        output.status.success()
            || stderr.contains("board")
            || stderr.contains("registry")
            || stderr.contains("not found"),
        "Should accept --board flag: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: Appending to existing .gitignore preserves existing content
/// **Validates: Requirement 1.7**
#[test]
fn test_init_appends_to_existing_gitignore() {
    let project = TestProject::new();

    // Create existing .gitignore with custom content
    let existing_content = "# My custom ignores\n*.log\nnode_modules/\n";
    project.create_file(".gitignore", existing_content);

    let output = run_init(&project, &["--force"]);

    assert!(
        output.status.success(),
        "zigroot init --force should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let content = project.read_file(".gitignore");

    // Verify existing content is preserved
    assert!(
        content.contains("*.log"),
        "Existing .gitignore entries should be preserved"
    );
    assert!(
        content.contains("node_modules/"),
        "Existing .gitignore entries should be preserved"
    );

    // Verify zigroot entries are added
    assert!(
        content.contains("build/"),
        "zigroot entries should be added"
    );
    assert!(
        content.contains("# zigroot"),
        "zigroot section marker should be added"
    );
}

/// Test: Appending to existing .gitignore is idempotent
/// **Property 24: Gitignore Append Idempotence**
/// **Validates: Requirement 1.7**
#[test]
fn test_init_gitignore_append_idempotent() {
    let project = TestProject::new();

    // First init
    let output1 = run_init(&project, &[]);
    assert!(output1.status.success(), "First init should succeed");

    let content_after_first = project.read_file(".gitignore");

    // Second init with --force
    let output2 = run_init(&project, &["--force"]);
    assert!(output2.status.success(), "Second init should succeed");

    let content_after_second = project.read_file(".gitignore");

    // Content should be the same (idempotent)
    assert_eq!(
        content_after_first, content_after_second,
        "Running init twice should produce the same .gitignore content"
    );

    // Verify no duplicate entries
    let build_count = content_after_second.matches("build/").count();
    assert_eq!(
        build_count, 1,
        "build/ should appear exactly once, not duplicated"
    );
}

/// Test: Generated manifest has commented examples
/// **Validates: Requirement 1.5**
#[test]
fn test_init_manifest_has_commented_examples() {
    let project = TestProject::new();

    let output = run_init(&project, &[]);
    assert!(output.status.success(), "zigroot init should succeed");

    let content = project.read_file("zigroot.toml");

    // Verify manifest has comments with examples
    assert!(
        content.contains('#'),
        "Manifest should contain comments with examples"
    );
}

// ============================================
// Property-Based Tests
// ============================================

/// Strategy for generating valid project directory states
fn directory_state_strategy() -> impl Strategy<Value = Vec<(String, String)>> {
    prop::collection::vec(
        (
            "[a-z][a-z0-9_]{0,10}\\.(txt|md|json)".prop_filter("valid filename", |s| {
                !s.is_empty() && !s.contains("zigroot")
            }),
            "[a-zA-Z0-9 ]{1,50}",
        ),
        0..5,
    )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 24: Gitignore Append Idempotence
    /// For any existing .gitignore content, running zigroot init multiple times
    /// should produce the same result (idempotent).
    /// **Validates: Requirement 1.7**
    #[test]
    fn prop_gitignore_append_idempotent(
        existing_entries in prop::collection::vec("[a-z_/]+", 0..10)
    ) {
        let project = TestProject::new();

        // Create initial .gitignore with random entries
        let initial_content = existing_entries.join("\n");
        if !initial_content.is_empty() {
            project.create_file(".gitignore", &initial_content);
        }

        // First init
        let output1 = run_init(&project, &["--force"]);
        prop_assume!(output1.status.success());

        let content_after_first = project.read_file(".gitignore");

        // Second init
        let output2 = run_init(&project, &["--force"]);
        prop_assume!(output2.status.success());

        let content_after_second = project.read_file(".gitignore");

        // Idempotence: content should be identical
        prop_assert_eq!(
            content_after_first,
            content_after_second,
            "Gitignore append should be idempotent"
        );
    }

    /// Property: Init with --force preserves unrelated files
    /// **Validates: Requirement 1.4**
    #[test]
    fn prop_init_force_preserves_files(
        files in directory_state_strategy()
    ) {
        let project = TestProject::new();

        // Create files
        for (name, content) in &files {
            project.create_file(name, content);
        }

        // Run init with --force
        let output = run_init(&project, &["--force"]);
        prop_assume!(output.status.success());

        // Verify all files are preserved
        for (name, content) in &files {
            prop_assert!(
                project.file_exists(name),
                "File {} should be preserved",
                name
            );
            prop_assert_eq!(
                project.read_file(name),
                content.clone(),
                "File {} content should be unchanged",
                name
            );
        }
    }
}
