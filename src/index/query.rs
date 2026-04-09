//! Search and query operations

use crate::domain::entry::{IndexEntry, EntryType};
use crate::persistence::entry_repo::EntryRepo;
use crate::Result;

/// Search options
pub struct SearchOptions {
    /// Minimum file size filter
    pub min_size: Option<u64>,
    /// Maximum file size filter
    pub max_size: Option<u64>,
    /// Extension filter
    pub ext: Option<String>,
    /// Entry type filter
    pub entry_type: Option<EntryType>,
    /// Limit results
    pub limit: usize,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            min_size: None,
            max_size: None,
            ext: None,
            entry_type: None,
            limit: 50,
        }
    }
}

/// Search result with match score
pub struct SearchResult {
    pub entry: IndexEntry,
    pub score: u32,
}

/// Calculate a simple match score based on how well the keyword matches
fn calculate_score(file_name: &str, keyword: &str) -> u32 {
    let file_lower = file_name.to_lowercase();
    let keyword_lower = keyword.to_lowercase();

    // Exact match gets highest score
    if file_lower == keyword_lower {
        return 1000;
    }

    // Starts with keyword gets high score
    if file_lower.starts_with(&keyword_lower) {
        return 800;
    }

    // Contains keyword gets medium score
    if file_lower.contains(&keyword_lower) {
        // Bonus for earlier position
        let pos = file_lower.find(&keyword_lower).unwrap_or(0);
        return 500 + (100 - pos.min(100) as u32);
    }

    // No match
    0
}

/// Search entries by keyword with simple scoring
pub fn search(repo: &EntryRepo, keyword: &str, options: SearchOptions) -> Result<Vec<SearchResult>> {
    // Phase 1: SQL pre-filter with LIKE
    let sql_results = repo.search_by_name(keyword, options.limit * 2)?;

    // Phase 2: Apply additional filters and calculate scores
    let mut scored: Vec<SearchResult> = sql_results
        .into_iter()
        .filter(|e| {
            // Size filter
            if let Some(min) = options.min_size {
                if e.size < min {
                    return false;
                }
            }
            if let Some(max) = options.max_size {
                if e.size > max {
                    return false;
                }
            }

            // Extension filter
            if let Some(ext) = &options.ext {
                if e.extension() != Some(ext.trim_start_matches('.')) {
                    return false;
                }
            }

            // Entry type filter
            if let Some(t) = options.entry_type {
                if e.entry_type != t {
                    return false;
                }
            }

            true
        })
        .filter_map(|entry| {
            let score = calculate_score(&entry.file_name, keyword);
            if score > 0 {
                Some(SearchResult { entry, score })
            } else {
                None
            }
        })
        .collect();

    // Sort by score (higher = better match)
    scored.sort_by(|a, b| b.score.cmp(&a.score));

    // Apply limit
    scored.truncate(options.limit);

    Ok(scored)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_score() {
        // exact match (whole filename equals keyword)
        assert_eq!(calculate_score("test", "test"), 1000);
        // starts with keyword
        assert_eq!(calculate_score("test_file.txt", "test"), 800);
        // contains keyword
        assert!(calculate_score("my_test_file.txt", "test") >= 500);
    }
}