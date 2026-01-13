//! CLI command for `zigroot cache`
//!
//! Manages build cache for faster rebuilds and cache sharing.
//!
//! **Validates: Requirements 24.1-24.8**

use anyhow::Result;
use std::path::Path;

use crate::core::cache::{clean_cache, export_cache, get_cache_info, import_cache};

/// Execute cache info subcommand
pub async fn execute_info(project_dir: &Path) -> Result<()> {
    println!("📦 Cache Information\n");

    let info = get_cache_info(project_dir);

    println!("Location: {}", info.path.display());
    println!("Size: {}", info.format_size());
    println!("Items: {}", info.item_count);

    if !info.exists {
        println!("\n⚠️  Cache directory does not exist (empty cache)");
    }

    Ok(())
}

/// Execute cache clean subcommand
pub async fn execute_clean(project_dir: &Path) -> Result<()> {
    println!("🧹 Cleaning cache...\n");

    match clean_cache(project_dir) {
        Ok(size_freed) => {
            if size_freed > 0 {
                let size_str = format_size(size_freed);
                println!("✅ Cache cleared ({size_str} freed)");
            } else {
                println!("✅ Cache was already empty");
            }
            Ok(())
        }
        Err(e) => {
            println!("❌ Failed to clean cache: {e}");
            Err(e.into())
        }
    }
}

/// Execute cache export subcommand
pub async fn execute_export(project_dir: &Path, output: &str) -> Result<()> {
    println!("📤 Exporting cache...\n");

    let output_path = Path::new(output);

    match export_cache(project_dir, output_path) {
        Ok(size) => {
            if size > 0 {
                let size_str = format_size(size);
                println!("✅ Cache exported to: {output}");
                println!("   Size: {size_str}");
            } else {
                println!("✅ Empty cache exported to: {output}");
            }
            Ok(())
        }
        Err(e) => {
            println!("❌ Failed to export cache: {e}");
            Err(e.into())
        }
    }
}

/// Execute cache import subcommand
pub async fn execute_import(project_dir: &Path, input: &str) -> Result<()> {
    println!("📥 Importing cache...\n");

    let input_path = Path::new(input);

    match import_cache(project_dir, input_path) {
        Ok(_) => {
            println!("✅ Cache imported from: {input}");
            Ok(())
        }
        Err(e) => {
            println!("❌ Failed to import cache: {e}");
            Err(e.into())
        }
    }
}

/// Format size for display
fn format_size(size_bytes: u64) -> String {
    if size_bytes == 0 {
        "0 bytes".to_string()
    } else if size_bytes < 1024 {
        format!("{} bytes", size_bytes)
    } else if size_bytes < 1024 * 1024 {
        format!("{:.1} KB", size_bytes as f64 / 1024.0)
    } else if size_bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", size_bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", size_bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}
