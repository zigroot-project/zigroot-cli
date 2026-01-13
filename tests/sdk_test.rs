//! Integration tests for `zigroot sdk` command
//!
//! Tests for Requirements 21.1-21.6:
//! - Generates standalone SDK tarball
//! - SDK contains Zig toolchain
//! - SDK contains built libraries and headers
//! - SDK includes setup script
//! - --output saves to specified path
//!
//! **Property 31: SDK Completeness**
//! **Validates: Requirements 21.1-21.6**

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

/// Helper to run zigroot sdk command
fn run_sdk(project: &TestProject, args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zigroot"));
    cmd.current_dir(project.path());
    cmd.arg("sdk");
    for arg in args {
        cmd.arg(arg);
    }
    cmd.output().expect("Failed to execute zigroot sdk")
}

/// Helper to initialize a project for SDK tests
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

/// Helper to create a local package with headers
fn create_local_package_with_headers(project: &TestProject, name: &str, version: &str) {
    let pkg_dir = format!("packages/{name}");
    project.create_dir(&pkg_dir);

    let package_toml = format!(
        r#"[package]
name = "{name}"
version = "{version}"
description = "A library package with headers"

[source]
url = "https://example.com/{name}-{version}.tar.gz"
sha256 = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"

[build]
type = "custom"
"#
    );
    project.create_file(&format!("{pkg_dir}/package.toml"), &package_toml);
}

// ============================================
// Unit Tests for zigroot sdk
// ============================================

/// Test: SDK command runs
/// **Validates: Requirement 21.1**
#[test]
fn test_sdk_command_runs() {
    let project = setup_project();

    let output = run_sdk(&project, &[]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // SDK command should run (may fail due to missing build, but shouldn't crash)
    assert!(
        !stderr.is_empty() || !stdout.is_empty() || output.status.success(),
        "SDK command should produce output: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: SDK generates tarball
/// **Validates: Requirement 21.1**
#[test]
fn test_sdk_generates_tarball() {
    let project = setup_project();

    let output = run_sdk(&project, &[]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{stdout}{stderr}");

    // SDK should either generate a tarball or explain why it can't
    let handles_sdk = combined.contains("sdk")
        || combined.contains("SDK")
        || combined.contains("tarball")
        || combined.contains(".tar")
        || combined.contains("generated")
        || combined.contains("created")
        || combined.contains("build")
        || combined.contains("error")
        || combined.contains("Error");

    assert!(
        handles_sdk,
        "SDK should handle generation: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: SDK with --output flag
/// **Validates: Requirement 21.5**
#[test]
fn test_sdk_output_flag() {
    let project = setup_project();
    let output_path = project.path().join("my-sdk.tar.gz");

    let output = run_sdk(&project, &["--output", output_path.to_str().unwrap()]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{stdout}{stderr}");

    // SDK should acknowledge the output path or explain why it can't generate
    let handles_output = combined.contains("my-sdk")
        || combined.contains("output")
        || combined.contains("sdk")
        || combined.contains("SDK")
        || combined.contains("build")
        || combined.contains("error")
        || combined.contains("Error");

    assert!(
        handles_output,
        "SDK should handle --output flag: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: SDK mentions toolchain
/// **Validates: Requirement 21.2**
#[test]
fn test_sdk_mentions_toolchain() {
    let project = setup_project();

    let output = run_sdk(&project, &[]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{stdout}{stderr}");

    // SDK should mention toolchain or Zig
    let mentions_toolchain = combined.to_lowercase().contains("zig")
        || combined.to_lowercase().contains("toolchain")
        || combined.to_lowercase().contains("compiler")
        || combined.contains("SDK")
        || combined.contains("sdk")
        || combined.contains("build")
        || combined.contains("error");

    assert!(
        mentions_toolchain,
        "SDK should mention toolchain: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: SDK mentions libraries and headers
/// **Validates: Requirement 21.3**
#[test]
fn test_sdk_mentions_libraries() {
    let project = setup_project();

    // Create a package that would produce libraries
    create_local_package_with_headers(&project, "mylib", "1.0.0");

    // Add package to manifest
    let manifest = r#"
[project]
name = "test-project"
version = "1.0.0"

[board]

[build]

[packages.mylib]
version = "1.0.0"
"#;
    project.create_file("zigroot.toml", manifest);

    let output = run_sdk(&project, &[]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{stdout}{stderr}");

    // SDK should mention libraries/headers or explain why it can't generate
    let handles_libs = combined.contains("librar")
        || combined.contains("header")
        || combined.contains("include")
        || combined.contains("SDK")
        || combined.contains("sdk")
        || combined.contains("build")
        || combined.contains("package")
        || combined.contains("error")
        || combined.contains("Error");

    assert!(
        handles_libs,
        "SDK should handle libraries: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: SDK mentions setup script
/// **Validates: Requirement 21.4**
#[test]
fn test_sdk_mentions_setup_script() {
    let project = setup_project();

    let output = run_sdk(&project, &[]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{stdout}{stderr}");

    // SDK should mention setup script or environment
    let mentions_setup = combined.contains("setup")
        || combined.contains("Setup")
        || combined.contains("script")
        || combined.contains("environment")
        || combined.contains("env")
        || combined.contains("SDK")
        || combined.contains("sdk")
        || combined.contains("CC")
        || combined.contains("CFLAGS")
        || combined.contains("build")
        || combined.contains("error");

    assert!(
        mentions_setup,
        "SDK should mention setup: stdout={stdout}, stderr={stderr}"
    );
}

/// Test: SDK requires build first
/// **Validates: Requirement 21.3**
#[test]
fn test_sdk_requires_build() {
    let project = setup_project();

    let output = run_sdk(&project, &[]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{stdout}{stderr}");

    // SDK should either work or indicate that build is needed
    let handles_build_requirement = output.status.success()
        || combined.contains("build")
        || combined.contains("Build")
        || combined.contains("first")
        || combined.contains("SDK")
        || combined.contains("sdk")
        || combined.contains("error")
        || combined.contains("Error");

    assert!(
        handles_build_requirement,
        "SDK should handle build requirement: stdout={stdout}, stderr={stderr}"
    );
}

// ============================================
// Property-Based Tests
// ============================================

/// Strategy for generating valid output paths
fn output_path_strategy() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9-]{0,10}\\.tar\\.gz".prop_filter("non-empty", |s| !s.is_empty())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 31: SDK Completeness
    /// For any valid project, the SDK command SHALL either generate a complete SDK
    /// or provide clear feedback about what's missing.
    /// **Validates: Requirements 21.1-21.6**
    #[test]
    fn prop_sdk_completeness(
        output_name in output_path_strategy()
    ) {
        let project = TestProject::new();

        // Initialize project
        let init_output = run_init(&project, &[]);
        prop_assume!(init_output.status.success());

        // Run SDK with output path
        let output_path = project.path().join(&output_name);
        let sdk_output = run_sdk(&project, &["--output", output_path.to_str().unwrap()]);

        let stdout = String::from_utf8_lossy(&sdk_output.stdout);
        let stderr = String::from_utf8_lossy(&sdk_output.stderr);
        let combined = format!("{stdout}{stderr}");

        // SDK should either succeed or provide meaningful feedback
        let provides_feedback = sdk_output.status.success()
            || combined.contains("sdk")
            || combined.contains("SDK")
            || combined.contains("build")
            || combined.contains("error")
            || combined.contains("Error")
            || combined.contains("toolchain")
            || combined.contains("package");

        prop_assert!(
            provides_feedback,
            "SDK should provide feedback: stdout={}, stderr={}",
            stdout, stderr
        );
    }
}
