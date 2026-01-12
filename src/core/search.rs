//! Search functionality for packages and boards
//!
//! Implements unified search across Package_Registry and Board_Registry.
//!
//! **Validates: Requirements 10.1-10.9**

use crate::registry::client::{BoardIndexEntry, PackageIndexEntry, RegistryClient};
use thiserror::Error;

/// Search errors
#[derive(Error, Debug)]
pub enum SearchError {
    /// Registry error
    #[error("Registry error: {0}")]
    RegistryError(String),

    /// No results found
    #[error("No results found for '{query}'")]
    NoResults { query: String },
}

/// Search result type
#[derive(Debug, Clone)]
pub enum SearchResultType {
    /// Package result
    Package,
    /// Board result
    Board,
}

/// A single search result
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// Result type (package or board)
    pub result_type: SearchResultType,
    /// Name of the package or board
    pub name: String,
    /// Version (for packages) or architecture (for boards)
    pub version_or_arch: String,
    /// Description
    pub description: String,
    /// Keywords for matching
    pub keywords: Vec<String>,
    /// Match score (higher is better)
    pub score: u32,
}

/// Search options
#[derive(Debug, Clone, Default)]
pub struct SearchOptions {
    /// Search only packages
    pub packages_only: bool,
    /// Search only boards
    pub boards_only: bool,
    /// Force refresh of index
    pub refresh: bool,
}

/// Search results container
#[derive(Debug, Clone)]
pub struct SearchResults {
    /// Package results (sorted by relevance)
    pub packages: Vec<SearchResult>,
    /// Board results (sorted by relevance)
    pub boards: Vec<SearchResult>,
    /// The original query
    pub query: String,
    /// Suggestions when no results found
    pub suggestions: Vec<String>,
}

impl SearchResults {
    /// Check if there are any results
    pub fn is_empty(&self) -> bool {
        self.packages.is_empty() && self.boards.is_empty()
    }

    /// Get total number of results
    pub fn total(&self) -> usize {
        self.packages.len() + self.boards.len()
    }
}

/// Perform a search across packages and boards
pub async fn search(
    client: &RegistryClient,
    query: &str,
    options: &SearchOptions,
) -> Result<SearchResults, SearchError> {
    let query_lower = query.to_lowercase();
    let mut packages = Vec::new();
    let mut boards = Vec::new();

    // Refresh indexes if requested
    if options.refresh {
        client
            .refresh()
            .await
            .map_err(|e| SearchError::RegistryError(e.to_string()))?;
    }

    // Search packages unless boards_only is set
    if !options.boards_only {
        match client.fetch_package_index().await {
            Ok(index) => {
                for pkg in &index.packages {
                    if let Some(score) = calculate_match_score(&query_lower, pkg) {
                        packages.push(SearchResult {
                            result_type: SearchResultType::Package,
                            name: pkg.name.clone(),
                            version_or_arch: pkg.latest.clone(),
                            description: pkg.description.clone(),
                            keywords: pkg.keywords.clone(),
                            score,
                        });
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Failed to fetch package index: {}", e);
            }
        }
    }

    // Search boards unless packages_only is set
    if !options.packages_only {
        match client.fetch_board_index().await {
            Ok(index) => {
                for board in &index.boards {
                    if let Some(score) = calculate_board_match_score(&query_lower, board) {
                        boards.push(SearchResult {
                            result_type: SearchResultType::Board,
                            name: board.name.clone(),
                            version_or_arch: board.arch.clone(),
                            description: board.description.clone(),
                            keywords: board.keywords.clone(),
                            score,
                        });
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Failed to fetch board index: {}", e);
            }
        }
    }

    // Sort by score (descending)
    packages.sort_by(|a, b| b.score.cmp(&a.score));
    boards.sort_by(|a, b| b.score.cmp(&a.score));

    // Generate suggestions if no results
    let suggestions = if packages.is_empty() && boards.is_empty() {
        generate_suggestions(&query_lower, client).await
    } else {
        Vec::new()
    };

    Ok(SearchResults {
        packages,
        boards,
        query: query.to_string(),
        suggestions,
    })
}

/// Calculate match score for a package
fn calculate_match_score(query: &str, pkg: &PackageIndexEntry) -> Option<u32> {
    let name_lower = pkg.name.to_lowercase();
    let desc_lower = pkg.description.to_lowercase();

    let mut score = 0u32;

    // Exact name match (highest priority)
    if name_lower == query {
        score += 100;
    }
    // Name starts with query
    else if name_lower.starts_with(query) {
        score += 80;
    }
    // Name contains query
    else if name_lower.contains(query) {
        score += 60;
    }

    // Description contains query
    if desc_lower.contains(query) {
        score += 20;
    }

    // Keyword matches
    for keyword in &pkg.keywords {
        let kw_lower = keyword.to_lowercase();
        if kw_lower == query {
            score += 40;
        } else if kw_lower.contains(query) {
            score += 15;
        }
    }

    if score > 0 {
        Some(score)
    } else {
        None
    }
}

/// Calculate match score for a board
fn calculate_board_match_score(query: &str, board: &BoardIndexEntry) -> Option<u32> {
    let name_lower = board.name.to_lowercase();
    let desc_lower = board.description.to_lowercase();
    let arch_lower = board.arch.to_lowercase();

    let mut score = 0u32;

    // Exact name match (highest priority)
    if name_lower == query {
        score += 100;
    }
    // Name starts with query
    else if name_lower.starts_with(query) {
        score += 80;
    }
    // Name contains query
    else if name_lower.contains(query) {
        score += 60;
    }

    // Architecture matches
    if arch_lower == query || arch_lower.contains(query) {
        score += 30;
    }

    // Description contains query
    if desc_lower.contains(query) {
        score += 20;
    }

    // Keyword matches
    for keyword in &board.keywords {
        let kw_lower = keyword.to_lowercase();
        if kw_lower == query {
            score += 40;
        } else if kw_lower.contains(query) {
            score += 15;
        }
    }

    if score > 0 {
        Some(score)
    } else {
        None
    }
}

/// Generate suggestions when no results found
async fn generate_suggestions(query: &str, client: &RegistryClient) -> Vec<String> {
    let mut suggestions = Vec::new();

    // Try to find similar package names
    if let Ok(index) = client.fetch_package_index().await {
        for pkg in &index.packages {
            let name_lower = pkg.name.to_lowercase();
            // Check for partial matches or similar names
            if levenshtein_distance(query, &name_lower) <= 3 {
                suggestions.push(format!("Did you mean '{}'?", pkg.name));
            }
        }
    }

    // Try to find similar board names
    if let Ok(index) = client.fetch_board_index().await {
        for board in &index.boards {
            let name_lower = board.name.to_lowercase();
            if levenshtein_distance(query, &name_lower) <= 3 {
                suggestions.push(format!("Did you mean '{}'?", board.name));
            }
        }
    }

    // Add generic suggestions if no specific ones found
    if suggestions.is_empty() {
        suggestions.push("Try a different search term".to_string());
        suggestions.push("Use 'zigroot search --packages <query>' to search only packages".to_string());
        suggestions.push("Use 'zigroot search --boards <query>' to search only boards".to_string());
    }

    // Limit suggestions
    suggestions.truncate(5);
    suggestions
}

/// Calculate Levenshtein distance between two strings
fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let len1 = s1.chars().count();
    let len2 = s2.chars().count();

    if len1 == 0 {
        return len2;
    }
    if len2 == 0 {
        return len1;
    }

    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();

    let mut matrix = vec![vec![0usize; len2 + 1]; len1 + 1];

    for i in 0..=len1 {
        matrix[i][0] = i;
    }
    for j in 0..=len2 {
        matrix[0][j] = j;
    }

    for i in 1..=len1 {
        for j in 1..=len2 {
            let cost = if s1_chars[i - 1] == s2_chars[j - 1] {
                0
            } else {
                1
            };
            matrix[i][j] = (matrix[i - 1][j] + 1)
                .min(matrix[i][j - 1] + 1)
                .min(matrix[i - 1][j - 1] + cost);
        }
    }

    matrix[len1][len2]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(levenshtein_distance("", ""), 0);
        assert_eq!(levenshtein_distance("abc", ""), 3);
        assert_eq!(levenshtein_distance("", "abc"), 3);
        assert_eq!(levenshtein_distance("abc", "abc"), 0);
        assert_eq!(levenshtein_distance("abc", "abd"), 1);
        assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
    }

    #[test]
    fn test_calculate_match_score_exact() {
        let pkg = PackageIndexEntry {
            name: "busybox".to_string(),
            description: "Swiss army knife".to_string(),
            license: None,
            keywords: vec![],
            versions: vec![],
            latest: "1.0.0".to_string(),
        };

        let score = calculate_match_score("busybox", &pkg);
        assert!(score.is_some());
        assert!(score.unwrap() >= 100);
    }

    #[test]
    fn test_calculate_match_score_partial() {
        let pkg = PackageIndexEntry {
            name: "busybox".to_string(),
            description: "Swiss army knife".to_string(),
            license: None,
            keywords: vec![],
            versions: vec![],
            latest: "1.0.0".to_string(),
        };

        let score = calculate_match_score("busy", &pkg);
        assert!(score.is_some());
        assert!(score.unwrap() >= 60);
    }

    #[test]
    fn test_calculate_match_score_no_match() {
        let pkg = PackageIndexEntry {
            name: "busybox".to_string(),
            description: "Swiss army knife".to_string(),
            license: None,
            keywords: vec![],
            versions: vec![],
            latest: "1.0.0".to_string(),
        };

        let score = calculate_match_score("xyz123", &pkg);
        assert!(score.is_none());
    }

    #[test]
    fn test_calculate_match_score_keyword() {
        let pkg = PackageIndexEntry {
            name: "busybox".to_string(),
            description: "Swiss army knife".to_string(),
            license: None,
            keywords: vec!["shell".to_string(), "coreutils".to_string()],
            versions: vec![],
            latest: "1.0.0".to_string(),
        };

        let score = calculate_match_score("shell", &pkg);
        assert!(score.is_some());
        assert!(score.unwrap() >= 40);
    }
}
