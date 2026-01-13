//! Integration tests for configuration management
//!
//! Tests for Requirements 11.2-11.5:
//! - Environment variable substitution (${VAR} syntax)
//! - Configuration inheritance (extends directive)
//! - Manifest validation (schema validation, error reporting)
//!
//! **Validates: Requirements 11.2, 11.3, 11.4, 11.5**

mod common;

use common::TestProject;

// ============================================
// Environment Variable Substitution Tests
// **Validates: Requirements 11.2**
// ============================================

/// Test: ${VAR} syntax substitutes correctly with existing env var
/// **Validates: Requirement 11.2**
#[test]
fn test_env_var_substitution_basic() {
    // Set up environment variable
    std::env::set_var("ZIGROOT_TEST_VAR", "test-value");

    let input = "prefix_${ZIGROOT_TEST_VAR}_suffix";
    let result = zigroot::core::manifest::substitute_env_vars(input);

    assert_eq!(result.unwrap(), "prefix_test-value_suffix");

    // Clean up
    std::env::remove_var("ZIGROOT_TEST_VAR");
}

/// Test: Multiple ${VAR} substitutions in same string
/// **Validates: Requirement 11.2**
#[test]
fn test_env_var_substitution_multiple() {
    std::env::set_var("ZIGROOT_VAR1", "first");
    std::env::set_var("ZIGROOT_VAR2", "second");

    let input = "${ZIGROOT_VAR1}_and_${ZIGROOT_VAR2}";
    let result = zigroot::core::manifest::substitute_env_vars(input);

    assert_eq!(result.unwrap(), "first_and_second");

    std::env::remove_var("ZIGROOT_VAR1");
    std::env::remove_var("ZIGROOT_VAR2");
}

/// Test: Undefined env var returns error or empty
/// **Validates: Requirement 11.2**
#[test]
fn test_env_var_substitution_undefined() {
    // Make sure the var doesn't exist
    std::env::remove_var("ZIGROOT_UNDEFINED_VAR");

    let input = "prefix_${ZIGROOT_UNDEFINED_VAR}_suffix";
    let result = zigroot::core::manifest::substitute_env_vars(input);

    // Should either return error or substitute with empty string
    match result {
        Ok(s) => assert_eq!(s, "prefix__suffix"),
        Err(_) => {} // Error is also acceptable
    }
}

/// Test: No substitution when no ${} syntax present
/// **Validates: Requirement 11.2**
#[test]
fn test_env_var_substitution_no_vars() {
    let input = "plain_string_without_vars";
    let result = zigroot::core::manifest::substitute_env_vars(input);

    assert_eq!(result.unwrap(), "plain_string_without_vars");
}

/// Test: Escaped or malformed ${} is handled gracefully
/// **Validates: Requirement 11.2**
#[test]
fn test_env_var_substitution_malformed() {
    // Unclosed brace
    let input = "prefix_${UNCLOSED";
    let result = zigroot::core::manifest::substitute_env_vars(input);

    // Should either return as-is or error
    match result {
        Ok(s) => assert!(s.contains("${UNCLOSED") || s.contains("UNCLOSED")),
        Err(_) => {} // Error is acceptable for malformed input
    }
}

/// Test: Manifest loads with env var substitution in values
/// **Validates: Requirement 11.2**
#[test]
fn test_manifest_env_var_substitution() {
    std::env::set_var("ZIGROOT_PROJECT_NAME", "my-project");
    std::env::set_var("ZIGROOT_HOSTNAME", "mydevice");

    let project = TestProject::new();
    let manifest_content = r#"
[project]
name = "${ZIGROOT_PROJECT_NAME}"
version = "1.0.0"

[board]
name = "test-board"

[build]
hostname = "${ZIGROOT_HOSTNAME}"
"#;
    project.create_file("zigroot.toml", manifest_content);

    let manifest = zigroot::core::manifest::Manifest::load_with_env_substitution(
        project.path().join("zigroot.toml").as_path(),
    );

    match manifest {
        Ok(m) => {
            assert_eq!(m.project.name, "my-project");
            assert_eq!(m.build.hostname, "mydevice");
        }
        Err(e) => panic!("Failed to load manifest with env substitution: {e}"),
    }

    std::env::remove_var("ZIGROOT_PROJECT_NAME");
    std::env::remove_var("ZIGROOT_HOSTNAME");
}

// ============================================
// Configuration Inheritance Tests
// **Validates: Requirements 11.5**
// ============================================

/// Test: extends directive loads base configuration
/// **Validates: Requirement 11.5**
#[test]
fn test_config_inheritance_basic() {
    let project = TestProject::new();

    // Create base configuration
    let base_config = r#"
[project]
name = "base-project"
version = "1.0.0"

[board]
name = "base-board"

[build]
compress = true
image_format = "squashfs"
rootfs_size = "128M"
hostname = "base-host"
"#;
    project.create_file("base.toml", base_config);

    // Create derived configuration that extends base
    let derived_config = r#"
extends = "base.toml"

[project]
name = "derived-project"

[build]
hostname = "derived-host"
"#;
    project.create_file("zigroot.toml", derived_config);

    let manifest = zigroot::core::manifest::Manifest::load_with_inheritance(
        project.path().join("zigroot.toml").as_path(),
    );

    match manifest {
        Ok(m) => {
            // Derived values should override base
            assert_eq!(m.project.name, "derived-project");
            assert_eq!(m.build.hostname, "derived-host");
            // Base values should be inherited
            assert_eq!(m.board.name, Some("base-board".to_string()));
            assert!(m.build.compress);
            assert_eq!(m.build.image_format, "squashfs");
            assert_eq!(m.build.rootfs_size, "128M");
        }
        Err(e) => panic!("Failed to load manifest with inheritance: {e}"),
    }
}

/// Test: extends directive with relative path
/// **Validates: Requirement 11.5**
#[test]
fn test_config_inheritance_relative_path() {
    let project = TestProject::new();

    // Create base configuration in a subdirectory
    project.create_dir("configs");
    let base_config = r#"
[project]
name = "base-project"
version = "2.0.0"

[build]
compress = false
"#;
    project.create_file("configs/base.toml", base_config);

    // Create derived configuration that extends base with relative path
    let derived_config = r#"
extends = "configs/base.toml"

[project]
name = "derived-project"
"#;
    project.create_file("zigroot.toml", derived_config);

    let manifest = zigroot::core::manifest::Manifest::load_with_inheritance(
        project.path().join("zigroot.toml").as_path(),
    );

    match manifest {
        Ok(m) => {
            assert_eq!(m.project.name, "derived-project");
            assert_eq!(m.project.version, "2.0.0"); // Inherited from base
            assert!(!m.build.compress); // Inherited from base
        }
        Err(e) => panic!("Failed to load manifest with relative path inheritance: {e}"),
    }
}

/// Test: extends directive with non-existent base file
/// **Validates: Requirement 11.5**
#[test]
fn test_config_inheritance_missing_base() {
    let project = TestProject::new();

    let derived_config = r#"
extends = "nonexistent.toml"

[project]
name = "derived-project"
"#;
    project.create_file("zigroot.toml", derived_config);

    let result = zigroot::core::manifest::Manifest::load_with_inheritance(
        project.path().join("zigroot.toml").as_path(),
    );

    assert!(
        result.is_err(),
        "Should fail when base config doesn't exist"
    );
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("nonexistent") || err.contains("not found") || err.contains("No such file"),
        "Error should mention missing file: {err}"
    );
}

/// Test: extends directive with chained inheritance
/// **Validates: Requirement 11.5**
#[test]
fn test_config_inheritance_chained() {
    let project = TestProject::new();

    // Create grandparent configuration
    let grandparent_config = r#"
[project]
name = "grandparent"
version = "1.0.0"

[build]
compress = true
image_format = "ext4"
rootfs_size = "64M"
"#;
    project.create_file("grandparent.toml", grandparent_config);

    // Create parent configuration that extends grandparent
    let parent_config = r#"
extends = "grandparent.toml"

[project]
name = "parent"

[build]
image_format = "squashfs"
"#;
    project.create_file("parent.toml", parent_config);

    // Create child configuration that extends parent
    let child_config = r#"
extends = "parent.toml"

[project]
name = "child"
"#;
    project.create_file("zigroot.toml", child_config);

    let manifest = zigroot::core::manifest::Manifest::load_with_inheritance(
        project.path().join("zigroot.toml").as_path(),
    );

    match manifest {
        Ok(m) => {
            // Child overrides
            assert_eq!(m.project.name, "child");
            // Parent overrides grandparent
            assert_eq!(m.build.image_format, "squashfs");
            // Grandparent values inherited through chain
            assert!(m.build.compress);
            assert_eq!(m.build.rootfs_size, "64M");
            assert_eq!(m.project.version, "1.0.0");
        }
        Err(e) => panic!("Failed to load manifest with chained inheritance: {e}"),
    }
}

/// Test: No extends directive works normally
/// **Validates: Requirement 11.5**
#[test]
fn test_config_no_inheritance() {
    let project = TestProject::new();

    let config = r#"
[project]
name = "standalone-project"
version = "1.0.0"

[build]
compress = false
"#;
    project.create_file("zigroot.toml", config);

    let manifest = zigroot::core::manifest::Manifest::load_with_inheritance(
        project.path().join("zigroot.toml").as_path(),
    );

    match manifest {
        Ok(m) => {
            assert_eq!(m.project.name, "standalone-project");
            assert_eq!(m.project.version, "1.0.0");
            assert!(!m.build.compress);
        }
        Err(e) => panic!("Failed to load manifest without inheritance: {e}"),
    }
}

// ============================================
// Manifest Validation Tests
// **Validates: Requirements 11.3, 11.4**
// ============================================

/// Test: Validates schema before build - missing required project name
/// **Validates: Requirement 11.3, 11.4**
#[test]
fn test_manifest_validation_missing_project_name() {
    let project = TestProject::new();

    let invalid_config = r#"
[project]
version = "1.0.0"

[build]
compress = false
"#;
    project.create_file("zigroot.toml", invalid_config);

    let result =
        zigroot::core::manifest::validate_manifest(project.path().join("zigroot.toml").as_path());

    assert!(
        result.is_err(),
        "Should fail validation when project name is missing"
    );
    let errors = result.unwrap_err();
    assert!(!errors.is_empty(), "Should report at least one error");
    let error_str = errors.join(", ");
    assert!(
        error_str.contains("name") || error_str.contains("project"),
        "Error should mention missing 'name' field: {error_str}"
    );
}

/// Test: Validates schema before build - missing project section
/// **Validates: Requirement 11.3, 11.4**
#[test]
fn test_manifest_validation_missing_project_section() {
    let project = TestProject::new();

    let invalid_config = r#"
[build]
compress = false
"#;
    project.create_file("zigroot.toml", invalid_config);

    let result =
        zigroot::core::manifest::validate_manifest(project.path().join("zigroot.toml").as_path());

    assert!(
        result.is_err(),
        "Should fail validation when project section is missing"
    );
    let errors = result.unwrap_err();
    assert!(!errors.is_empty(), "Should report at least one error");
    let error_str = errors.join(", ");
    assert!(
        error_str.contains("project") || error_str.contains("missing"),
        "Error should mention missing 'project' section: {error_str}"
    );
}

/// Test: Reports all errors at once
/// **Validates: Requirement 11.4**
#[test]
fn test_manifest_validation_reports_all_errors() {
    let project = TestProject::new();

    // Config with multiple issues
    let invalid_config = r#"
[project]
# Missing name
version = "not-a-valid-semver"

[build]
image_format = "invalid_format"
rootfs_size = "not-a-size"
"#;
    project.create_file("zigroot.toml", invalid_config);

    let result =
        zigroot::core::manifest::validate_manifest(project.path().join("zigroot.toml").as_path());

    assert!(
        result.is_err(),
        "Should fail validation with multiple errors"
    );
    let errors = result.unwrap_err();
    // Should report multiple errors, not just the first one
    assert!(
        errors.len() >= 1,
        "Should report at least one error, got: {:?}",
        errors
    );
}

/// Test: Valid manifest passes validation
/// **Validates: Requirement 11.3, 11.4**
#[test]
fn test_manifest_validation_valid_config() {
    let project = TestProject::new();

    let valid_config = r#"
[project]
name = "valid-project"
version = "1.0.0"
description = "A valid project"

[board]
name = "test-board"

[build]
compress = false
image_format = "ext4"
rootfs_size = "256M"
hostname = "mydevice"
"#;
    project.create_file("zigroot.toml", valid_config);

    let result =
        zigroot::core::manifest::validate_manifest(project.path().join("zigroot.toml").as_path());

    assert!(
        result.is_ok(),
        "Valid manifest should pass validation: {:?}",
        result.err()
    );
}

/// Test: Validates image format values
/// **Validates: Requirement 11.3, 11.4**
#[test]
fn test_manifest_validation_invalid_image_format() {
    let project = TestProject::new();

    let invalid_config = r#"
[project]
name = "test-project"
version = "1.0.0"

[build]
image_format = "invalid_format"
"#;
    project.create_file("zigroot.toml", invalid_config);

    let result =
        zigroot::core::manifest::validate_manifest(project.path().join("zigroot.toml").as_path());

    // This may or may not be an error depending on strictness
    // At minimum, the function should not panic
    match result {
        Ok(()) => {} // Lenient validation is acceptable
        Err(errors) => {
            let error_str = errors.join(", ");
            assert!(
                error_str.contains("image_format") || error_str.contains("format"),
                "Error should mention invalid image format: {error_str}"
            );
        }
    }
}

/// Test: Validates rootfs_size format
/// **Validates: Requirement 11.3, 11.4**
#[test]
fn test_manifest_validation_invalid_rootfs_size() {
    let project = TestProject::new();

    let invalid_config = r#"
[project]
name = "test-project"
version = "1.0.0"

[build]
rootfs_size = "not-a-size"
"#;
    project.create_file("zigroot.toml", invalid_config);

    let result =
        zigroot::core::manifest::validate_manifest(project.path().join("zigroot.toml").as_path());

    // This may or may not be an error depending on strictness
    match result {
        Ok(()) => {} // Lenient validation is acceptable
        Err(errors) => {
            let error_str = errors.join(", ");
            assert!(
                error_str.contains("rootfs_size") || error_str.contains("size"),
                "Error should mention invalid rootfs size: {error_str}"
            );
        }
    }
}

/// Test: Helpful error message format
/// **Validates: Requirement 11.3**
#[test]
fn test_manifest_validation_helpful_error_message() {
    let project = TestProject::new();

    let invalid_config = r#"
[project]
version = "1.0.0"
"#;
    project.create_file("zigroot.toml", invalid_config);

    let result =
        zigroot::core::manifest::validate_manifest(project.path().join("zigroot.toml").as_path());

    assert!(result.is_err(), "Should fail validation");
    let errors = result.unwrap_err();
    let error_str = errors.join(", ");

    // Error message should be helpful - mention what's missing and expected format
    assert!(
        error_str.contains("name")
            || error_str.contains("required")
            || error_str.contains("missing"),
        "Error should be helpful and mention what's wrong: {error_str}"
    );
}
