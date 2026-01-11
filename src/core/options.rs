//! Option validation
//!
//! Validates package and board option values against their definitions.

use crate::error::OptionError;
use regex::Regex;

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
