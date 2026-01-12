//! Search command implementation
//!
//! Implements `zigroot search` for unified search across packages and boards.
//!
//! **Validates: Requirements 10.1-10.9**

use anyhow::Result;

use crate::core::search::{self, SearchOptions, SearchResultType};
use crate::registry::client::RegistryClient;

/// Execute the search command
pub async fn execute(
    query: &str,
    packages_only: bool,
    boards_only: bool,
    refresh: bool,
) -> Result<()> {
    let client = RegistryClient::new();

    let options = SearchOptions {
        packages_only,
        boards_only,
        refresh,
    };

    tracing::info!("Searching for '{}'...", query);

    let results = search::search(&client, query, &options)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    if results.is_empty() {
        println!("No results found for '{}'", query);
        println!();

        if !results.suggestions.is_empty() {
            println!("Suggestions:");
            for suggestion in &results.suggestions {
                println!("  • {suggestion}");
            }
        }

        return Ok(());
    }

    // Display results grouped by type (packages first, then boards)
    // This satisfies Requirement 10.2

    // Display package results
    if !results.packages.is_empty() {
        println!("Packages ({} found):", results.packages.len());
        println!();

        for result in &results.packages {
            display_result(result, query);
        }

        if !results.boards.is_empty() {
            println!(); // Separator between groups
        }
    }

    // Display board results
    if !results.boards.is_empty() {
        println!("Boards ({} found):", results.boards.len());
        println!();

        for result in &results.boards {
            display_result(result, query);
        }
    }

    println!();
    println!(
        "Found {} result(s) for '{}'",
        results.total(),
        results.query
    );

    Ok(())
}

/// Display a single search result with highlighting
fn display_result(result: &search::SearchResult, query: &str) {
    let type_label = match result.result_type {
        SearchResultType::Package => "[package]",
        SearchResultType::Board => "[board]",
    };

    // Highlight the query in the name if present
    let highlighted_name = highlight_match(&result.name, query);

    // Format version/arch info
    let version_info = match result.result_type {
        SearchResultType::Package => format!("v{}", result.version_or_arch),
        SearchResultType::Board => result.version_or_arch.clone(),
    };

    // Print the result
    println!(
        "  {} {} {} - {}",
        type_label, highlighted_name, version_info, result.description
    );

    // Show keywords if any match the query
    let matching_keywords: Vec<&String> = result
        .keywords
        .iter()
        .filter(|k| k.to_lowercase().contains(&query.to_lowercase()))
        .collect();

    if !matching_keywords.is_empty() {
        println!(
            "    Keywords: {}",
            matching_keywords
                .iter()
                .map(|k| k.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
}

/// Highlight matching text in a string
/// For terminal output, we use ANSI escape codes for bold
fn highlight_match(text: &str, query: &str) -> String {
    let text_lower = text.to_lowercase();
    let query_lower = query.to_lowercase();

    if let Some(pos) = text_lower.find(&query_lower) {
        let before = &text[..pos];
        let matched = &text[pos..pos + query.len()];
        let after = &text[pos + query.len()..];

        // Use ANSI bold for highlighting
        format!("{before}\x1b[1m{matched}\x1b[0m{after}")
    } else {
        text.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlight_match() {
        let result = highlight_match("busybox", "busy");
        assert!(result.contains("\x1b[1m"));
        assert!(result.contains("busy"));
    }

    #[test]
    fn test_highlight_match_no_match() {
        let result = highlight_match("busybox", "xyz");
        assert_eq!(result, "busybox");
    }

    #[test]
    fn test_highlight_match_case_insensitive() {
        let result = highlight_match("BusyBox", "busy");
        assert!(result.contains("\x1b[1m"));
    }
}
