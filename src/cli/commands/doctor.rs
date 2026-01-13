//! CLI command for `zigroot doctor`
//!
//! Checks system dependencies and reports issues with suggestions.
//!
//! **Validates: Requirements 14.5, 14.6**

use anyhow::Result;
use std::path::Path;

use crate::core::doctor::run_doctor;

/// Execute the doctor command
pub async fn execute(project_dir: Option<&Path>) -> Result<()> {
    println!("🔍 Checking system dependencies...\n");

    let report = run_doctor(project_dir);

    // Print check results
    for check in &report.checks {
        let status = if check.passed {
            "✓".to_string()
        } else {
            "✗".to_string()
        };

        let version_str = check
            .version
            .as_ref()
            .map(|v| format!(" (v{v})"))
            .unwrap_or_default();

        let required_str = if check.required { "" } else { " [optional]" };

        if check.passed {
            println!("  {status} {}{version_str}{required_str}", check.name);
        } else {
            println!("  {status} {}{required_str}", check.name);
            if let Some(error) = &check.error {
                println!("    Error: {error}");
            }
            if let Some(suggestion) = &check.suggestion {
                println!("    Suggestion: {suggestion}");
            }
        }
    }

    // Print configuration issues
    if !report.config_issues.is_empty() {
        println!("\n⚠️  Configuration issues:");
        for issue in &report.config_issues {
            println!("  • {issue}");
        }
    }

    // Print summary
    println!();
    let passed = report.passed_count();
    let total = report.checks.len();
    let failed_required = report.failed_required();

    if report.all_passed() {
        println!("✅ All checks passed ({passed}/{total})");
        println!("   System is ready for zigroot!");
    } else if failed_required.is_empty() {
        println!(
            "⚠️  {passed}/{total} checks passed (optional dependencies missing)",
        );
        println!("   System is ready for basic zigroot usage.");
    } else {
        println!("❌ {passed}/{total} checks passed");
        println!("   Please install missing required dependencies:");
        for check in failed_required {
            if let Some(suggestion) = &check.suggestion {
                println!("   • {}: {suggestion}", check.name);
            }
        }
        return Err(anyhow::anyhow!(
            "Missing required dependencies. Run 'zigroot doctor' for details."
        ));
    }

    Ok(())
}
