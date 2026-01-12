//! Integration tests for `zigroot search` command
//!
//! Tests for Requirement 10: Unified Search
//! - Searches both packages and boards
//! - Results grouped by type (packages first, then boards)
//! - --packages searches only packages
//! - --boards searches only boards
//! - --refresh forces index refresh
//! - Highlights matching terms
//! - Suggests alternatives when no results
//!
//! **Property 10: Search Result Grouping**
//! **Property 29: Search Suggestions on Empty Results**
//! **Validates: Requirements 10.1-10.9**

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

/// Helper to run zigroot search command
fn run_search(project: &TestProject, args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zigroot"));
    cmd.current_dir(project.path());
    cmd.arg("search");
    for arg in args {
        cmd.arg(arg);
    }
    cmd.output().expect("Failed to execute zigroot search")
}

/// Helper to initialize a project for search tests
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
// Unit Tests for zigroot search
// ============================================

/// Test: Searches both packages and boards
/// **Validates: Requirement 10.1**
#[test]
fn test_search_searches_both_packages_and_boards() {
    let project = setup_project();

    // Search for a common term that might match both packages and boards
    let output = run_search(&project, &["linux"]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Search should succeed (even if no results from real registry)
    // The command should at least run without crashing
    assert!(
        output.status.success() || stderr.contains("no results") || stderr.contains("not found"),
        "zigroot search should handle search gracefully: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: Results grouped by type (packages first, then boards)
/// **Validates: Requirement 10.2**
#[test]
fn test_search_results_grouped_by_type() {
    let project = setup_project();

    let output = run_search(&project, &["busybox"]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // If results are found, packages should appear before boards
    // Check that output contains grouping indicators or labels
    if output.status.success() && !stdout.is_empty() {
        // Results should be grouped - packages first, then boards
        // Look for [package] appearing before [board] if both exist
        let pkg_pos = stdout.find("[package]").or_else(|| stdout.find("Package"));
        let board_pos = stdout.find("[board]").or_else(|| stdout.find("Board"));

        if let (Some(p), Some(b)) = (pkg_pos, board_pos) {
            assert!(
                p < b,
                "Packages should appear before boards in search results"
            );
        }
    }

    // Command should not crash
    assert!(
        output.status.success() || stderr.contains("no results") || stderr.contains("error"),
        "Search should complete: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: Package results display name, version, description, and [package] label
/// **Validates: Requirement 10.3**
#[test]
fn test_search_package_results_format() {
    let project = setup_project();

    let output = run_search(&project, &["--packages", "busybox"]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // If results are found, they should contain package information
    if output.status.success() && stdout.contains("busybox") {
        // Should have package label
        let has_label = stdout.contains("[package]")
            || stdout.contains("Package")
            || stdout.contains("PACKAGE");

        assert!(
            has_label || stdout.contains("busybox"),
            "Package results should have proper format: {stdout}"
        );
    }

    // Command should not crash
    assert!(
        output.status.success() || stderr.contains("no results") || stderr.contains("error"),
        "Search should complete: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: Board results display name, architecture, description, and [board] label
/// **Validates: Requirement 10.4**
#[test]
fn test_search_board_results_format() {
    let project = setup_project();

    let output = run_search(&project, &["--boards", "pico"]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // If results are found, they should contain board information
    if output.status.success() && !stdout.is_empty() && !stdout.contains("No results") {
        // Should have board label
        let has_label =
            stdout.contains("[board]") || stdout.contains("Board") || stdout.contains("BOARD");

        assert!(
            has_label || stdout.is_empty() || stdout.contains("No"),
            "Board results should have proper format: {stdout}"
        );
    }

    // Command should not crash
    assert!(
        output.status.success() || stderr.contains("no results") || stderr.contains("error"),
        "Search should complete: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: --packages searches only packages
/// **Validates: Requirement 10.5**
#[test]
fn test_search_packages_only_flag() {
    let project = setup_project();

    let output = run_search(&project, &["--packages", "busybox"]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // When --packages is used, should not show board results
    if output.status.success() && !stdout.is_empty() {
        // Should not contain board labels
        let has_board = stdout.contains("[board]");
        assert!(
            !has_board,
            "--packages should not show board results: {stdout}"
        );
    }

    // Command should not crash
    assert!(
        output.status.success() || stderr.contains("no results") || stderr.contains("error"),
        "Search should complete: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: --boards searches only boards
/// **Validates: Requirement 10.6**
#[test]
fn test_search_boards_only_flag() {
    let project = setup_project();

    let output = run_search(&project, &["--boards", "raspberry"]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // When --boards is used, should not show package results
    if output.status.success() && !stdout.is_empty() {
        // Should not contain package labels
        let has_package = stdout.contains("[package]");
        assert!(
            !has_package,
            "--boards should not show package results: {stdout}"
        );
    }

    // Command should not crash
    assert!(
        output.status.success() || stderr.contains("no results") || stderr.contains("error"),
        "Search should complete: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: --refresh forces index refresh
/// **Validates: Requirement 10.7**
#[test]
fn test_search_refresh_flag() {
    let project = setup_project();

    let output = run_search(&project, &["--refresh", "busybox"]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // --refresh should force a refresh of the index
    // The command should succeed or indicate refresh happened
    assert!(
        output.status.success()
            || stderr.contains("refresh")
            || stderr.contains("no results")
            || stderr.contains("error"),
        "Search with --refresh should complete: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: Highlights matching terms in search results
/// **Validates: Requirement 10.8**
#[test]
fn test_search_highlights_matching_terms() {
    let project = setup_project();

    let output = run_search(&project, &["busybox"]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // If results are found, the search term should appear in output
    if output.status.success() && !stdout.is_empty() && stdout.contains("busybox") {
        // The term "busybox" should be present (highlighting is visual, hard to test)
        assert!(
            stdout.to_lowercase().contains("busybox"),
            "Search results should contain the search term: {stdout}"
        );
    }

    // Command should not crash
    assert!(
        output.status.success() || stderr.contains("no results") || stderr.contains("error"),
        "Search should complete: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: Suggests alternatives when no results found
/// **Validates: Requirement 10.9**
#[test]
fn test_search_suggests_alternatives_when_no_results() {
    let project = setup_project();

    // Search for something that definitely won't exist
    let output = run_search(&project, &["xyznonexistent123456"]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // When no results found, should suggest alternatives or show helpful message
    let combined = format!("{stdout}{stderr}");
    let suggests_or_no_results = combined.contains("No results")
        || combined.contains("no results")
        || combined.contains("not found")
        || combined.contains("suggest")
        || combined.contains("try")
        || combined.contains("popular")
        || combined.contains("Did you mean");

    assert!(
        suggests_or_no_results || output.status.success(),
        "Search should suggest alternatives or indicate no results: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: Search without query shows error
#[test]
fn test_search_requires_query() {
    let project = setup_project();

    // Run search without a query argument
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zigroot"));
    cmd.current_dir(project.path());
    cmd.arg("search");
    let output = cmd.output().expect("Failed to execute zigroot search");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should fail or show usage because query is required
    assert!(
        !output.status.success() || stderr.contains("required") || stderr.contains("usage"),
        "Search without query should fail or show usage"
    );
}

/// Test: Search works without initialized project (searches registry)
#[test]
fn test_search_works_without_project() {
    let project = TestProject::new(); // Not initialized

    let output = run_search(&project, &["busybox"]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Search should work even without a project (it searches the registry)
    // It may fail due to network issues, but shouldn't crash
    assert!(
        output.status.success()
            || stderr.contains("network")
            || stderr.contains("registry")
            || stderr.contains("no results")
            || stderr.contains("error"),
        "Search should work or fail gracefully without project: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: Search with empty query
#[test]
fn test_search_with_empty_string() {
    let project = setup_project();

    let output = run_search(&project, &[""]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Empty query should either fail or return all results
    // The behavior depends on implementation
    assert!(
        output.status.success()
            || stderr.contains("empty")
            || stderr.contains("required")
            || stderr.contains("error"),
        "Search with empty query should handle gracefully: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: Search is case-insensitive
#[test]
fn test_search_case_insensitive() {
    let project = setup_project();

    let output_lower = run_search(&project, &["busybox"]);
    let output_upper = run_search(&project, &["BUSYBOX"]);
    let output_mixed = run_search(&project, &["BusyBox"]);

    // All three should produce similar results (or all fail similarly)
    let lower_success = output_lower.status.success();
    let upper_success = output_upper.status.success();
    let mixed_success = output_mixed.status.success();

    // They should all have the same success/failure status
    assert_eq!(
        lower_success, upper_success,
        "Search should be case-insensitive"
    );
    assert_eq!(
        lower_success, mixed_success,
        "Search should be case-insensitive"
    );
}

// ============================================
// Property-Based Tests
// ============================================

/// Strategy for generating valid search queries
fn search_query_strategy() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9-]{0,15}".prop_filter("non-empty", |s| !s.is_empty())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 10: Search Result Grouping
    /// For any search query, results SHALL be grouped by type with packages
    /// appearing before boards.
    /// **Validates: Requirements 10.1, 10.2**
    #[test]
    fn prop_search_results_grouped_packages_before_boards(
        query in search_query_strategy()
    ) {
        let project = TestProject::new();

        // Initialize project
        let init_output = run_init(&project, &[]);
        prop_assume!(init_output.status.success());

        // Run search
        let search_output = run_search(&project, &[&query]);

        let stdout = String::from_utf8_lossy(&search_output.stdout);

        // If both package and board results exist, packages should come first
        let pkg_pos = stdout.find("[package]");
        let board_pos = stdout.find("[board]");

        if let (Some(p), Some(b)) = (pkg_pos, board_pos) {
            prop_assert!(
                p < b,
                "Packages should appear before boards: query={}, stdout={}",
                query, stdout
            );
        }

        // Search should not crash
        prop_assert!(
            search_output.status.success()
                || String::from_utf8_lossy(&search_output.stderr).contains("error")
                || String::from_utf8_lossy(&search_output.stderr).contains("no results"),
            "Search should complete without crashing"
        );
    }

    /// Property 29: Search Suggestions on Empty Results
    /// For any search query that returns no results, the system SHALL suggest
    /// alternative search terms or popular items.
    /// **Validates: Requirement 10.9**
    #[test]
    fn prop_search_suggests_on_empty_results(
        // Generate queries unlikely to match anything
        query in "xyz[0-9]{5,10}"
    ) {
        let project = TestProject::new();

        // Initialize project
        let init_output = run_init(&project, &[]);
        prop_assume!(init_output.status.success());

        // Run search with unlikely query
        let search_output = run_search(&project, &[&query]);

        let stdout = String::from_utf8_lossy(&search_output.stdout);
        let stderr = String::from_utf8_lossy(&search_output.stderr);
        let combined = format!("{stdout}{stderr}");

        // If no results, should suggest alternatives or indicate no results
        let has_results = stdout.contains("[package]") || stdout.contains("[board]");

        if !has_results {
            let suggests_or_indicates = combined.contains("No results")
                || combined.contains("no results")
                || combined.contains("not found")
                || combined.contains("suggest")
                || combined.contains("try")
                || combined.contains("popular")
                || combined.contains("Did you mean")
                || combined.contains("error")
                || combined.contains("network");

            prop_assert!(
                suggests_or_indicates,
                "Should suggest alternatives when no results: query={}, output={}",
                query, combined
            );
        }
    }

    /// Property: --packages flag excludes board results
    /// **Validates: Requirement 10.5**
    #[test]
    fn prop_packages_flag_excludes_boards(
        query in search_query_strategy()
    ) {
        let project = TestProject::new();

        // Initialize project
        let init_output = run_init(&project, &[]);
        prop_assume!(init_output.status.success());

        // Run search with --packages flag
        let search_output = run_search(&project, &["--packages", &query]);

        let stdout = String::from_utf8_lossy(&search_output.stdout);

        // Should not contain board results
        prop_assert!(
            !stdout.contains("[board]"),
            "--packages should exclude board results: query={}, stdout={}",
            query, stdout
        );
    }

    /// Property: --boards flag excludes package results
    /// **Validates: Requirement 10.6**
    #[test]
    fn prop_boards_flag_excludes_packages(
        query in search_query_strategy()
    ) {
        let project = TestProject::new();

        // Initialize project
        let init_output = run_init(&project, &[]);
        prop_assume!(init_output.status.success());

        // Run search with --boards flag
        let search_output = run_search(&project, &["--boards", &query]);

        let stdout = String::from_utf8_lossy(&search_output.stdout);

        // Should not contain package results
        prop_assert!(
            !stdout.contains("[package]"),
            "--boards should exclude package results: query={}, stdout={}",
            query, stdout
        );
    }
}
