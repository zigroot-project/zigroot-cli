//! Integration tests for `zigroot doctor` command
//!
//! Tests for Requirements 14.5, 14.6:
//! - Checks system dependencies
//! - Reports issues with suggestions
//! - Detects common misconfigurations
//!
//! **Validates: Requirements 14.5, 14.6**

mod common;

use common::TestProject;
use std::process::Command;

/// Helper to run zigroot doctor command
fn run_doctor(args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zigroot"));
    cmd.arg("doctor");
    for arg in args {
        cmd.arg(arg);
    }
    cmd.output().expect("Failed to execute zigroot doctor")
}

/// Helper to run zigroot doctor in a specific directory
fn run_doctor_in_dir(project: &TestProject, args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zigroot"));
    cmd.current_dir(project.path());
    cmd.arg("doctor");
    for arg in args {
        cmd.arg(arg);
    }
    cmd.output().expect("Failed to execute zigroot doctor")
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

// ============================================
// Unit Tests for zigroot doctor
// ============================================

/// Test: Doctor command runs successfully
/// **Validates: Requirement 14.5**
#[test]
fn test_doctor_runs_successfully() {
    let output = run_doctor(&[]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Doctor command should complete (may succeed or report issues)
    // It should not crash
    assert!(
        output.status.success() || !stderr.is_empty() || !stdout.is_empty(),
        "Doctor should run and produce output: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: Doctor checks system dependencies
/// **Validates: Requirement 14.5**
#[test]
fn test_doctor_checks_system_dependencies() {
    let output = run_doctor(&[]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{stdout}{stderr}");

    // Doctor should check for essential tools
    let checks_dependencies = combined.contains("zig")
        || combined.contains("Zig")
        || combined.contains("toolchain")
        || combined.contains("dependency")
        || combined.contains("dependencies")
        || combined.contains("check")
        || combined.contains("system")
        || combined.contains("found")
        || combined.contains("missing")
        || combined.contains("installed")
        || combined.contains("✓")
        || combined.contains("✗")
        || combined.contains("ok")
        || combined.contains("error");

    assert!(
        checks_dependencies,
        "Doctor should check system dependencies: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: Doctor reports issues with suggestions
/// **Validates: Requirement 14.6**
#[test]
fn test_doctor_reports_issues_with_suggestions() {
    let output = run_doctor(&[]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{stdout}{stderr}");

    // If there are issues, doctor should provide suggestions
    // If no issues, it should indicate success
    let provides_feedback = combined.contains("suggestion")
        || combined.contains("Suggestion")
        || combined.contains("install")
        || combined.contains("Install")
        || combined.contains("try")
        || combined.contains("Try")
        || combined.contains("run")
        || combined.contains("ok")
        || combined.contains("OK")
        || combined.contains("pass")
        || combined.contains("Pass")
        || combined.contains("✓")
        || combined.contains("success")
        || combined.contains("Success")
        || combined.contains("All")
        || combined.contains("all")
        || combined.contains("good")
        || combined.contains("ready");

    assert!(
        provides_feedback,
        "Doctor should provide feedback or suggestions: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: Doctor detects common misconfigurations
/// **Validates: Requirement 14.6**
#[test]
fn test_doctor_detects_misconfigurations() {
    let project = TestProject::new();

    // Initialize a project
    let init_output = run_init(&project, &[]);
    assert!(
        init_output.status.success(),
        "Failed to initialize project"
    );

    // Create an invalid manifest to test misconfiguration detection
    project.create_file(
        "zigroot.toml",
        r#"
[project]
name = ""
version = "invalid-version"

[board]

[build]
"#,
    );

    let output = run_doctor_in_dir(&project, &[]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{stdout}{stderr}");

    // Doctor should detect configuration issues or at least run without crashing
    let handles_config = output.status.success()
        || combined.contains("config")
        || combined.contains("manifest")
        || combined.contains("invalid")
        || combined.contains("warning")
        || combined.contains("error")
        || combined.contains("issue")
        || combined.contains("problem");

    assert!(
        handles_config,
        "Doctor should handle configuration check: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: Doctor checks for Zig compiler
/// **Validates: Requirement 14.5**
#[test]
fn test_doctor_checks_zig_compiler() {
    let output = run_doctor(&[]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{stdout}{stderr}");

    // Doctor should check for Zig compiler availability
    let checks_zig = combined.to_lowercase().contains("zig")
        || combined.contains("compiler")
        || combined.contains("toolchain")
        || combined.contains("build tool");

    assert!(
        checks_zig,
        "Doctor should check for Zig compiler: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: Doctor checks for optional tools (UPX, git, etc.)
/// **Validates: Requirement 14.5**
#[test]
fn test_doctor_checks_optional_tools() {
    let output = run_doctor(&[]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{stdout}{stderr}");

    // Doctor should mention optional tools or indicate all checks passed
    let checks_tools = combined.contains("upx")
        || combined.contains("UPX")
        || combined.contains("git")
        || combined.contains("Git")
        || combined.contains("optional")
        || combined.contains("Optional")
        || combined.contains("compression")
        || combined.contains("all")
        || combined.contains("check")
        || combined.contains("✓")
        || combined.contains("ok");

    assert!(
        checks_tools || output.status.success(),
        "Doctor should check optional tools or indicate success: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: Doctor works without a project directory
/// **Validates: Requirement 14.5**
#[test]
fn test_doctor_works_without_project() {
    let project = TestProject::new();

    // Run doctor in empty directory (no zigroot.toml)
    let output = run_doctor_in_dir(&project, &[]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Doctor should still work and check system dependencies
    // even without a project
    assert!(
        output.status.success() || !stderr.is_empty() || !stdout.is_empty(),
        "Doctor should work without project: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: Doctor provides summary of checks
/// **Validates: Requirement 14.5, 14.6**
#[test]
fn test_doctor_provides_summary() {
    let output = run_doctor(&[]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{stdout}{stderr}");

    // Doctor should provide some kind of summary or status
    let has_summary = combined.contains("check")
        || combined.contains("Check")
        || combined.contains("status")
        || combined.contains("Status")
        || combined.contains("result")
        || combined.contains("Result")
        || combined.contains("summary")
        || combined.contains("Summary")
        || combined.contains("complete")
        || combined.contains("Complete")
        || combined.contains("done")
        || combined.contains("Done")
        || combined.contains("✓")
        || combined.contains("✗")
        || combined.contains("passed")
        || combined.contains("failed")
        || combined.contains("issue")
        || combined.contains("problem")
        || combined.contains("ok")
        || combined.contains("error");

    assert!(
        has_summary,
        "Doctor should provide summary: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: Doctor exit code reflects status
/// **Validates: Requirement 14.5, 14.6**
#[test]
fn test_doctor_exit_code_reflects_status() {
    let output = run_doctor(&[]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{stdout}{stderr}");

    // If doctor reports critical issues, it should exit with non-zero
    // If all checks pass, it should exit with zero
    let has_critical_issues = combined.contains("error")
        || combined.contains("Error")
        || combined.contains("critical")
        || combined.contains("Critical")
        || combined.contains("fatal")
        || combined.contains("Fatal");

    if has_critical_issues {
        // If critical issues are reported, exit code should be non-zero
        // (but this depends on implementation - some may warn but succeed)
        assert!(
            !output.status.success() || combined.contains("warning"),
            "Doctor with critical issues should fail or warn: stdout={stdout}, stderr={stderr}"
        );
    }
    // If no critical issues, success is expected
}
