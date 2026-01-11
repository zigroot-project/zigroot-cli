//! Option validation
//!
//! Validates package and board option values against their definitions.
//! Implements option value resolution with priority: CLI > Package > Global.

use crate::error::OptionError;
use regex::Regex;
use std::collections::HashMap;

use crate::core::package::OptionDefinition;

/// Option value source for resolution priority
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OptionSource {
    /// Value from CLI argument (highest priority)
    Cli,
    /// Value from package-specific configuration
    Package,
    /// Value from global configuration
    Global,
    /// Default value from option definition (lowest priority)
    Default,
}

/// Resolved option value with its source
#[derive(Debug, Clone)]
pub struct ResolvedOption {
    /// The resolved value
    pub value: toml::Value,
    /// Where the value came from
    pub source: OptionSource,
}

/// Resolve option values with priority: CLI > Package > Global > Default
///
/// # Arguments
/// * `definition` - The option definition with default value
/// * `cli_value` - Optional value from CLI arguments
/// * `package_value` - Optional value from package configuration
/// * `global_value` - Optional value from global configuration
///
/// # Returns
/// The resolved option value with its source
pub fn resolve_option_value(
    definition: &OptionDefinition,
    cli_value: Option<&toml::Value>,
    package_value: Option<&toml::Value>,
    global_value: Option<&toml::Value>,
) -> ResolvedOption {
    if let Some(value) = cli_value {
        ResolvedOption {
            value: value.clone(),
            source: OptionSource::Cli,
        }
    } else if let Some(value) = package_value {
        ResolvedOption {
            value: value.clone(),
            source: OptionSource::Package,
        }
    } else if let Some(value) = global_value {
        ResolvedOption {
            value: value.clone(),
            source: OptionSource::Global,
        }
    } else {
        ResolvedOption {
            value: definition.default.clone(),
            source: OptionSource::Default,
        }
    }
}

/// Resolve all options for a package/board
///
/// # Arguments
/// * `definitions` - Map of option name to definition
/// * `cli_values` - Map of option name to CLI value
/// * `package_values` - Map of option name to package config value
/// * `global_values` - Map of option name to global config value
///
/// # Returns
/// Map of option name to resolved value
pub fn resolve_all_options(
    definitions: &HashMap<String, OptionDefinition>,
    cli_values: &HashMap<String, toml::Value>,
    package_values: &HashMap<String, toml::Value>,
    global_values: &HashMap<String, toml::Value>,
) -> HashMap<String, ResolvedOption> {
    definitions
        .iter()
        .map(|(name, def)| {
            let resolved = resolve_option_value(
                def,
                cli_values.get(name),
                package_values.get(name),
                global_values.get(name),
            );
            (name.clone(), resolved)
        })
        .collect()
}

/// Validate all resolved options against their definitions
///
/// # Arguments
/// * `definitions` - Map of option name to definition
/// * `resolved` - Map of option name to resolved value
///
/// # Returns
/// Ok(()) if all options are valid, or the first validation error
pub fn validate_all_options(
    definitions: &HashMap<String, OptionDefinition>,
    resolved: &HashMap<String, ResolvedOption>,
) -> Result<(), OptionError> {
    for (name, def) in definitions {
        if let Some(resolved_opt) = resolved.get(name) {
            validate_option(
                name,
                &resolved_opt.value,
                &def.option_type,
                &def.choices,
                def.pattern.as_deref(),
                def.allow_empty,
                def.min,
                def.max,
            )?;
        }
    }
    Ok(())
}

/// Validate an option value against its definition
#[allow(clippy::too_many_arguments)]
pub fn validate_option(
    name: &str,
    value: &toml::Value,
    option_type: &str,
    choices: &[String],
    pattern: Option<&str>,
    allow_empty: bool,
    min: Option<f64>,
    max: Option<f64>,
) -> Result<(), OptionError> {
    match option_type {
        "bool" => validate_bool(name, value),
        "string" => validate_string(name, value, pattern, allow_empty),
        "choice" => validate_choice(name, value, choices),
        "number" => validate_number(name, value, min, max),
        _ => Err(OptionError::InvalidType {
            name: name.to_string(),
            expected: "bool, string, choice, or number".to_string(),
            got: option_type.to_string(),
        }),
    }
}

fn validate_bool(name: &str, value: &toml::Value) -> Result<(), OptionError> {
    if value.is_bool() {
        Ok(())
    } else {
        Err(OptionError::InvalidType {
            name: name.to_string(),
            expected: "boolean".to_string(),
            got: format!("{value:?}"),
        })
    }
}

fn validate_string(
    name: &str,
    value: &toml::Value,
    pattern: Option<&str>,
    allow_empty: bool,
) -> Result<(), OptionError> {
    let s = value.as_str().ok_or_else(|| OptionError::InvalidType {
        name: name.to_string(),
        expected: "string".to_string(),
        got: format!("{value:?}"),
    })?;

    if !allow_empty && s.is_empty() {
        return Err(OptionError::EmptyNotAllowed {
            name: name.to_string(),
        });
    }

    if let Some(pat) = pattern {
        let re = Regex::new(pat).map_err(|e| OptionError::InvalidPattern {
            name: name.to_string(),
            pattern: pat.to_string(),
            error: e.to_string(),
        })?;

        if !re.is_match(s) {
            return Err(OptionError::PatternMismatch {
                name: name.to_string(),
                value: s.to_string(),
                pattern: pat.to_string(),
            });
        }
    }

    Ok(())
}

fn validate_choice(name: &str, value: &toml::Value, choices: &[String]) -> Result<(), OptionError> {
    let s = value.as_str().ok_or_else(|| OptionError::InvalidType {
        name: name.to_string(),
        expected: "string".to_string(),
        got: format!("{value:?}"),
    })?;

    if choices.contains(&s.to_string()) {
        Ok(())
    } else {
        Err(OptionError::InvalidChoice {
            name: name.to_string(),
            value: s.to_string(),
            choices: choices.to_vec(),
        })
    }
}

fn validate_number(
    name: &str,
    value: &toml::Value,
    min: Option<f64>,
    max: Option<f64>,
) -> Result<(), OptionError> {
    let n = if let Some(i) = value.as_integer() {
        #[allow(clippy::cast_precision_loss)]
        {
            i as f64
        }
    } else if let Some(f) = value.as_float() {
        f
    } else {
        return Err(OptionError::InvalidType {
            name: name.to_string(),
            expected: "number".to_string(),
            got: format!("{value:?}"),
        });
    };

    if let Some(min_val) = min {
        if n < min_val {
            return Err(OptionError::OutOfRange {
                name: name.to_string(),
                value: n,
                min,
                max,
            });
        }
    }

    if let Some(max_val) = max {
        if n > max_val {
            return Err(OptionError::OutOfRange {
                name: name.to_string(),
                value: n,
                min,
                max,
            });
        }
    }

    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // ============================================
    // Unit Tests - Bool option validation
    // ============================================

    #[test]
    fn test_bool_option_validates_true() {
        let result = validate_option(
            "debug",
            &toml::Value::Boolean(true),
            "bool",
            &[],
            None,
            true,
            None,
            None,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_bool_option_validates_false() {
        let result = validate_option(
            "debug",
            &toml::Value::Boolean(false),
            "bool",
            &[],
            None,
            true,
            None,
            None,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_bool_option_rejects_string() {
        let result = validate_option(
            "debug",
            &toml::Value::String("true".to_string()),
            "bool",
            &[],
            None,
            true,
            None,
            None,
        );
        assert!(result.is_err());
        match result.unwrap_err() {
            OptionError::InvalidType { name, expected, .. } => {
                assert_eq!(name, "debug");
                assert_eq!(expected, "boolean");
            }
            _ => panic!("Expected InvalidType error"),
        }
    }

    #[test]
    fn test_bool_option_rejects_number() {
        let result = validate_option(
            "debug",
            &toml::Value::Integer(1),
            "bool",
            &[],
            None,
            true,
            None,
            None,
        );
        assert!(result.is_err());
    }

    // ============================================
    // Unit Tests - String option validation
    // ============================================

    #[test]
    fn test_string_option_validates_correctly() {
        let result = validate_option(
            "hostname",
            &toml::Value::String("myhost".to_string()),
            "string",
            &[],
            None,
            true,
            None,
            None,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_string_option_with_pattern_validates() {
        let result = validate_option(
            "hostname",
            &toml::Value::String("myhost123".to_string()),
            "string",
            &[],
            Some("^[a-z][a-z0-9]*$"),
            true,
            None,
            None,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_string_option_with_pattern_rejects_invalid() {
        let result = validate_option(
            "hostname",
            &toml::Value::String("123invalid".to_string()),
            "string",
            &[],
            Some("^[a-z][a-z0-9]*$"),
            true,
            None,
            None,
        );
        assert!(result.is_err());
        match result.unwrap_err() {
            OptionError::PatternMismatch { name, value, pattern } => {
                assert_eq!(name, "hostname");
                assert_eq!(value, "123invalid");
                assert_eq!(pattern, "^[a-z][a-z0-9]*$");
            }
            _ => panic!("Expected PatternMismatch error"),
        }
    }

    #[test]
    fn test_string_option_allow_empty_true() {
        let result = validate_option(
            "optional_field",
            &toml::Value::String(String::new()),
            "string",
            &[],
            None,
            true,
            None,
            None,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_string_option_allow_empty_false_rejects_empty() {
        let result = validate_option(
            "required_field",
            &toml::Value::String(String::new()),
            "string",
            &[],
            None,
            false,
            None,
            None,
        );
        assert!(result.is_err());
        match result.unwrap_err() {
            OptionError::EmptyNotAllowed { name } => {
                assert_eq!(name, "required_field");
            }
            _ => panic!("Expected EmptyNotAllowed error"),
        }
    }

    #[test]
    fn test_string_option_rejects_non_string() {
        let result = validate_option(
            "hostname",
            &toml::Value::Integer(123),
            "string",
            &[],
            None,
            true,
            None,
            None,
        );
        assert!(result.is_err());
        match result.unwrap_err() {
            OptionError::InvalidType { name, expected, .. } => {
                assert_eq!(name, "hostname");
                assert_eq!(expected, "string");
            }
            _ => panic!("Expected InvalidType error"),
        }
    }

    // ============================================
    // Unit Tests - Choice option validation
    // ============================================

    #[test]
    fn test_choice_option_validates_valid_choice() {
        let choices = vec!["sd".to_string(), "emmc".to_string(), "spi".to_string()];
        let result = validate_option(
            "boot_mode",
            &toml::Value::String("sd".to_string()),
            "choice",
            &choices,
            None,
            true,
            None,
            None,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_choice_option_rejects_invalid_value() {
        let choices = vec!["sd".to_string(), "emmc".to_string(), "spi".to_string()];
        let result = validate_option(
            "boot_mode",
            &toml::Value::String("usb".to_string()),
            "choice",
            &choices,
            None,
            true,
            None,
            None,
        );
        assert!(result.is_err());
        match result.unwrap_err() {
            OptionError::InvalidChoice { name, value, choices: c } => {
                assert_eq!(name, "boot_mode");
                assert_eq!(value, "usb");
                assert_eq!(c, choices);
            }
            _ => panic!("Expected InvalidChoice error"),
        }
    }

    #[test]
    fn test_choice_option_rejects_non_string() {
        let choices = vec!["sd".to_string(), "emmc".to_string()];
        let result = validate_option(
            "boot_mode",
            &toml::Value::Integer(1),
            "choice",
            &choices,
            None,
            true,
            None,
            None,
        );
        assert!(result.is_err());
    }

    // ============================================
    // Unit Tests - Number option validation
    // ============================================

    #[test]
    fn test_number_option_validates_integer() {
        let result = validate_option(
            "jobs",
            &toml::Value::Integer(4),
            "number",
            &[],
            None,
            true,
            None,
            None,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_number_option_validates_float() {
        let result = validate_option(
            "timeout",
            &toml::Value::Float(3.14),
            "number",
            &[],
            None,
            true,
            None,
            None,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_number_option_respects_min_bound() {
        let result = validate_option(
            "jobs",
            &toml::Value::Integer(0),
            "number",
            &[],
            None,
            true,
            Some(1.0),
            None,
        );
        assert!(result.is_err());
        match result.unwrap_err() {
            OptionError::OutOfRange { name, value, min, max } => {
                assert_eq!(name, "jobs");
                assert!((value - 0.0).abs() < f64::EPSILON);
                assert_eq!(min, Some(1.0));
                assert_eq!(max, None);
            }
            _ => panic!("Expected OutOfRange error"),
        }
    }

    #[test]
    fn test_number_option_respects_max_bound() {
        let result = validate_option(
            "jobs",
            &toml::Value::Integer(100),
            "number",
            &[],
            None,
            true,
            None,
            Some(16.0),
        );
        assert!(result.is_err());
        match result.unwrap_err() {
            OptionError::OutOfRange { name, value, min, max } => {
                assert_eq!(name, "jobs");
                assert!((value - 100.0).abs() < f64::EPSILON);
                assert_eq!(min, None);
                assert_eq!(max, Some(16.0));
            }
            _ => panic!("Expected OutOfRange error"),
        }
    }

    #[test]
    fn test_number_option_within_bounds() {
        let result = validate_option(
            "jobs",
            &toml::Value::Integer(8),
            "number",
            &[],
            None,
            true,
            Some(1.0),
            Some(16.0),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_number_option_at_min_bound() {
        let result = validate_option(
            "jobs",
            &toml::Value::Integer(1),
            "number",
            &[],
            None,
            true,
            Some(1.0),
            Some(16.0),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_number_option_at_max_bound() {
        let result = validate_option(
            "jobs",
            &toml::Value::Integer(16),
            "number",
            &[],
            None,
            true,
            Some(1.0),
            Some(16.0),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_number_option_rejects_string() {
        let result = validate_option(
            "jobs",
            &toml::Value::String("4".to_string()),
            "number",
            &[],
            None,
            true,
            None,
            None,
        );
        assert!(result.is_err());
        match result.unwrap_err() {
            OptionError::InvalidType { name, expected, .. } => {
                assert_eq!(name, "jobs");
                assert_eq!(expected, "number");
            }
            _ => panic!("Expected InvalidType error"),
        }
    }

    // ============================================
    // Unit Tests - Invalid option type
    // ============================================

    #[test]
    fn test_invalid_option_type_produces_error() {
        let result = validate_option(
            "unknown",
            &toml::Value::String("value".to_string()),
            "invalid_type",
            &[],
            None,
            true,
            None,
            None,
        );
        assert!(result.is_err());
        match result.unwrap_err() {
            OptionError::InvalidType { name, expected, got } => {
                assert_eq!(name, "unknown");
                assert_eq!(expected, "bool, string, choice, or number");
                assert_eq!(got, "invalid_type");
            }
            _ => panic!("Expected InvalidType error"),
        }
    }

    // ============================================
    // Unit Tests - Error message specificity
    // ============================================

    #[test]
    fn test_error_messages_include_option_name() {
        let result = validate_option(
            "my_option",
            &toml::Value::String("bad".to_string()),
            "choice",
            &["good".to_string()],
            None,
            true,
            None,
            None,
        );
        let err = result.unwrap_err();
        let err_msg = err.to_string();
        assert!(
            err_msg.contains("my_option"),
            "Error message should contain option name: {err_msg}"
        );
    }

    #[test]
    fn test_pattern_error_includes_pattern() {
        let result = validate_option(
            "hostname",
            &toml::Value::String("BAD".to_string()),
            "string",
            &[],
            Some("^[a-z]+$"),
            true,
            None,
            None,
        );
        let err = result.unwrap_err();
        let err_msg = err.to_string();
        assert!(
            err_msg.contains("^[a-z]+$"),
            "Error message should contain pattern: {err_msg}"
        );
    }

    #[test]
    fn test_choice_error_includes_valid_choices() {
        let choices = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let result = validate_option(
            "opt",
            &toml::Value::String("d".to_string()),
            "choice",
            &choices,
            None,
            true,
            None,
            None,
        );
        let err = result.unwrap_err();
        let err_msg = err.to_string();
        assert!(
            err_msg.contains("a") && err_msg.contains("b") && err_msg.contains("c"),
            "Error message should contain valid choices: {err_msg}"
        );
    }

    // ============================================
    // Property-Based Tests
    // ============================================

    /// Strategy for generating valid option names
    fn option_name_strategy() -> impl Strategy<Value = String> {
        "[a-z][a-z0-9_]{0,20}"
            .prop_filter("Name must not be empty", |s| !s.is_empty())
    }

    /// Strategy for generating valid string values
    fn string_value_strategy() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9_-]{1,50}"
    }

    /// Strategy for generating number values within bounds
    fn bounded_number_strategy(min: f64, max: f64) -> impl Strategy<Value = f64> {
        min..=max
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 13: Option Validation
        /// For any option with validation constraints (pattern, min, max, allow_empty),
        /// invalid values SHALL be rejected with a specific error.
        /// **Validates: Requirements 18.39, 18.40, 18.41, 18.42**
        #[test]
        fn prop_option_validation_rejects_invalid_values(
            name in option_name_strategy(),
            value in string_value_strategy(),
        ) {
            // Test that choice validation rejects values not in the choices list
            let choices = vec!["valid1".to_string(), "valid2".to_string()];
            
            // If value is not in choices, it should be rejected
            if !choices.contains(&value) {
                let result = validate_option(
                    &name,
                    &toml::Value::String(value.clone()),
                    "choice",
                    &choices,
                    None,
                    true,
                    None,
                    None,
                );
                prop_assert!(result.is_err(), "Invalid choice should be rejected");
                
                // Verify error contains the option name
                let err_msg = result.unwrap_err().to_string();
                prop_assert!(
                    err_msg.contains(&name),
                    "Error should contain option name"
                );
            }
        }

        /// Property: Valid bool values are always accepted
        #[test]
        fn prop_bool_accepts_valid_values(name in option_name_strategy(), value: bool) {
            let result = validate_option(
                &name,
                &toml::Value::Boolean(value),
                "bool",
                &[],
                None,
                true,
                None,
                None,
            );
            prop_assert!(result.is_ok(), "Valid bool should be accepted");
        }

        /// Property: Numbers within bounds are always accepted
        #[test]
        fn prop_number_within_bounds_accepted(
            name in option_name_strategy(),
            value in bounded_number_strategy(1.0, 100.0),
        ) {
            let result = validate_option(
                &name,
                &toml::Value::Float(value),
                "number",
                &[],
                None,
                true,
                Some(1.0),
                Some(100.0),
            );
            prop_assert!(result.is_ok(), "Number within bounds should be accepted");
        }

        /// Property: Numbers below min are always rejected
        #[test]
        fn prop_number_below_min_rejected(
            name in option_name_strategy(),
            min in 10.0f64..100.0f64,
        ) {
            let value = min - 1.0;
            let result = validate_option(
                &name,
                &toml::Value::Float(value),
                "number",
                &[],
                None,
                true,
                Some(min),
                None,
            );
            prop_assert!(result.is_err(), "Number below min should be rejected");
        }

        /// Property: Numbers above max are always rejected
        #[test]
        fn prop_number_above_max_rejected(
            name in option_name_strategy(),
            max in 10.0f64..100.0f64,
        ) {
            let value = max + 1.0;
            let result = validate_option(
                &name,
                &toml::Value::Float(value),
                "number",
                &[],
                None,
                true,
                None,
                Some(max),
            );
            prop_assert!(result.is_err(), "Number above max should be rejected");
        }

        /// Property: Empty strings rejected when allow_empty is false
        #[test]
        fn prop_empty_string_rejected_when_not_allowed(name in option_name_strategy()) {
            let result = validate_option(
                &name,
                &toml::Value::String(String::new()),
                "string",
                &[],
                None,
                false,
                None,
                None,
            );
            prop_assert!(result.is_err(), "Empty string should be rejected when allow_empty=false");
            
            match result.unwrap_err() {
                OptionError::EmptyNotAllowed { name: n } => {
                    prop_assert_eq!(n, name);
                }
                _ => prop_assert!(false, "Expected EmptyNotAllowed error"),
            }
        }

        /// Property: Valid choices are always accepted
        #[test]
        fn prop_valid_choice_accepted(name in option_name_strategy()) {
            let choices = vec!["opt1".to_string(), "opt2".to_string(), "opt3".to_string()];
            for choice in &choices {
                let result = validate_option(
                    &name,
                    &toml::Value::String(choice.clone()),
                    "choice",
                    &choices,
                    None,
                    true,
                    None,
                    None,
                );
                prop_assert!(result.is_ok(), "Valid choice should be accepted");
            }
        }

        /// Property: Strings matching pattern are accepted
        #[test]
        fn prop_string_matching_pattern_accepted(name in option_name_strategy()) {
            // Use a simple pattern that we know matches "abc123"
            let pattern = "^[a-z]+[0-9]*$";
            let value = "abc123";
            
            let result = validate_option(
                &name,
                &toml::Value::String(value.to_string()),
                "string",
                &[],
                Some(pattern),
                true,
                None,
                None,
            );
            prop_assert!(result.is_ok(), "String matching pattern should be accepted");
        }
    }

    // ============================================
    // Unit Tests - Option Resolution
    // ============================================

    #[test]
    fn test_resolve_option_cli_has_highest_priority() {
        let def = OptionDefinition {
            option_type: "string".to_string(),
            default: toml::Value::String("default".to_string()),
            description: "Test option".to_string(),
            choices: vec![],
            pattern: None,
            allow_empty: true,
            min: None,
            max: None,
        };

        let cli_value = toml::Value::String("cli".to_string());
        let package_value = toml::Value::String("package".to_string());
        let global_value = toml::Value::String("global".to_string());

        let resolved = resolve_option_value(
            &def,
            Some(&cli_value),
            Some(&package_value),
            Some(&global_value),
        );

        assert_eq!(resolved.value, cli_value);
        assert_eq!(resolved.source, OptionSource::Cli);
    }

    #[test]
    fn test_resolve_option_package_over_global() {
        let def = OptionDefinition {
            option_type: "string".to_string(),
            default: toml::Value::String("default".to_string()),
            description: "Test option".to_string(),
            choices: vec![],
            pattern: None,
            allow_empty: true,
            min: None,
            max: None,
        };

        let package_value = toml::Value::String("package".to_string());
        let global_value = toml::Value::String("global".to_string());

        let resolved = resolve_option_value(&def, None, Some(&package_value), Some(&global_value));

        assert_eq!(resolved.value, package_value);
        assert_eq!(resolved.source, OptionSource::Package);
    }

    #[test]
    fn test_resolve_option_global_over_default() {
        let def = OptionDefinition {
            option_type: "string".to_string(),
            default: toml::Value::String("default".to_string()),
            description: "Test option".to_string(),
            choices: vec![],
            pattern: None,
            allow_empty: true,
            min: None,
            max: None,
        };

        let global_value = toml::Value::String("global".to_string());

        let resolved = resolve_option_value(&def, None, None, Some(&global_value));

        assert_eq!(resolved.value, global_value);
        assert_eq!(resolved.source, OptionSource::Global);
    }

    #[test]
    fn test_resolve_option_falls_back_to_default() {
        let def = OptionDefinition {
            option_type: "string".to_string(),
            default: toml::Value::String("default".to_string()),
            description: "Test option".to_string(),
            choices: vec![],
            pattern: None,
            allow_empty: true,
            min: None,
            max: None,
        };

        let resolved = resolve_option_value(&def, None, None, None);

        assert_eq!(resolved.value, toml::Value::String("default".to_string()));
        assert_eq!(resolved.source, OptionSource::Default);
    }

    #[test]
    fn test_resolve_all_options() {
        let mut definitions = HashMap::new();
        definitions.insert(
            "opt1".to_string(),
            OptionDefinition {
                option_type: "string".to_string(),
                default: toml::Value::String("default1".to_string()),
                description: "Option 1".to_string(),
                choices: vec![],
                pattern: None,
                allow_empty: true,
                min: None,
                max: None,
            },
        );
        definitions.insert(
            "opt2".to_string(),
            OptionDefinition {
                option_type: "bool".to_string(),
                default: toml::Value::Boolean(false),
                description: "Option 2".to_string(),
                choices: vec![],
                pattern: None,
                allow_empty: true,
                min: None,
                max: None,
            },
        );

        let mut cli_values = HashMap::new();
        cli_values.insert("opt1".to_string(), toml::Value::String("cli1".to_string()));

        let mut package_values = HashMap::new();
        package_values.insert("opt2".to_string(), toml::Value::Boolean(true));

        let resolved = resolve_all_options(&definitions, &cli_values, &package_values, &HashMap::new());

        assert_eq!(resolved.len(), 2);
        assert_eq!(
            resolved.get("opt1").unwrap().value,
            toml::Value::String("cli1".to_string())
        );
        assert_eq!(resolved.get("opt1").unwrap().source, OptionSource::Cli);
        assert_eq!(
            resolved.get("opt2").unwrap().value,
            toml::Value::Boolean(true)
        );
        assert_eq!(resolved.get("opt2").unwrap().source, OptionSource::Package);
    }

    #[test]
    fn test_validate_all_options_success() {
        let mut definitions = HashMap::new();
        definitions.insert(
            "opt1".to_string(),
            OptionDefinition {
                option_type: "string".to_string(),
                default: toml::Value::String("default".to_string()),
                description: "Option 1".to_string(),
                choices: vec![],
                pattern: None,
                allow_empty: true,
                min: None,
                max: None,
            },
        );

        let mut resolved = HashMap::new();
        resolved.insert(
            "opt1".to_string(),
            ResolvedOption {
                value: toml::Value::String("valid".to_string()),
                source: OptionSource::Cli,
            },
        );

        let result = validate_all_options(&definitions, &resolved);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_all_options_failure() {
        let mut definitions = HashMap::new();
        definitions.insert(
            "opt1".to_string(),
            OptionDefinition {
                option_type: "choice".to_string(),
                default: toml::Value::String("a".to_string()),
                description: "Option 1".to_string(),
                choices: vec!["a".to_string(), "b".to_string()],
                pattern: None,
                allow_empty: true,
                min: None,
                max: None,
            },
        );

        let mut resolved = HashMap::new();
        resolved.insert(
            "opt1".to_string(),
            ResolvedOption {
                value: toml::Value::String("invalid".to_string()),
                source: OptionSource::Cli,
            },
        );

        let result = validate_all_options(&definitions, &resolved);
        assert!(result.is_err());
    }
}
