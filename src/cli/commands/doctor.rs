//! CLI command for `zigroot doctor`
//!
//! Checks system dependencies and reports issues with suggestions.
//!
//! **Validates: Requirements 14.5, 14.6**

use anyhow::Result;
use std::path::Path;

use crate::cli::output::{is_json, is_quiet, print_detail, print_info, print_success, print_warning, status};
use crate::core::doctor::run_doctor;

/// Execute the doctor command
pub async fn execute(project_dir: Option<&Path>) -> Result<()> {
    let report = run_doctor(project_dir);

    // JSON output mode
    if is_json() {
        let json_result = serde_json::json!({
            "status": if report.all_passed() { "success" } else if report.failed_required().is_empty() { "warning" } else { "error" },
            "checks": report.checks.iter().map(|c| serde_json::json!({
                "name": c.name,
                "passed": c.passed,
                "required": c.required,
                "version": c.version,
                "error": c.error,
                "suggestion": c.suggestion
            })).collect::<Vec<_>>(),
            "config_issues": report.config_issues,
            "passed_count": report.passed_count(),
            "total_count": report.checks.len()
        });
        println!("{}", serde_json::to_string_pretty(&json_result).unwrap_or_default());

        if !report.failed_required().is_empty() {
            return Err(anyhow::anyhow!("Missing required dependencies"));
        }
        return Ok(());
    }

    // Quiet mode - only show errors
    if is_quiet() {
        let failed_required = report.failed_required();
        if !failed_required.is_empty() {
            for check in failed_required {
                eprintln!("{} Missing required: {}", status::ERROR, check.name);
            }
            return Err(anyhow::anyhow!("Missing required dependencies"));
        }
        return Ok(());
    }

    // Normal output mode
    print_info("Checking system dependencies...");
    println!();

    // Print check results
    for check in &report.checks {
        let version_str = check
            .version
            .as_ref()
            .map(|v| format!(" (v{v})"))
            .unwrap_or_default();

        let required_str = if check.required { "" } else { " [optional]" };

        if check.passed {
            println!("  {} {}{version_str}{required_str}", status::SUCCESS, check.name);
        } else {
            println!("  {} {}{required_str}", status::ERROR, check.name);
            if let Some(error) = &check.error {
                print_detail(&format!("Error: {error}"));
            }
            if let Some(suggestion) = &check.suggestion {
                print_detail(&format!("Suggestion: {suggestion}"));
            }
        }
    }

    // Print configuration issues
    if !report.config_issues.is_empty() {
        println!();
        print_warning("Configuration issues:");
        for issue in &report.config_issues {
            print_detail(&format!("• {issue}"));
        }
    }

    // Print summary
    println!();
    let passed = report.passed_count();
    let total = report.checks.len();
    let failed_required = report.failed_required();

    if report.all_passed() {
        print_success(&format!("All checks passed ({passed}/{total})"));
        print_detail("System is ready for zigroot!");
    } else if failed_required.is_empty() {
        print_warning(&format!(
            "{passed}/{total} checks passed (optional dependencies missing)"
        ));
        print_detail("System is ready for basic zigroot usage.");
    } else {
        println!("{} {passed}/{total} checks passed", status::ERROR);
        print_detail("Please install missing required dependencies:");
        for check in &failed_required {
            if let Some(suggestion) = &check.suggestion {
                print_detail(&format!("• {}: {suggestion}", check.name));
            }
        }
        return Err(anyhow::anyhow!(
            "Missing required dependencies. Run 'zigroot doctor' for details."
        ));
    }

    Ok(())
}
