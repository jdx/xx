//! String similarity and suggestion utilities
//!
//! This module provides functions for finding similar strings, useful for
//! "did you mean?" suggestions in command-line tools.
//!
//! ## Examples
//!
//! ```rust
//! use xx::suggest;
//!
//! let candidates = vec!["install", "uninstall", "update", "upgrade"];
//! if let Some(suggestion) = suggest::similar("instal", &candidates) {
//!     println!("Did you mean '{}'?", suggestion);
//! }
//!
//! // Get multiple suggestions
//! let suggestions = suggest::similar_n("updat", &candidates, 2);
//! // Returns ["update", "upgrade"]
//! ```

use strsim::jaro_winkler;

/// Default threshold for similarity matching (0.0 to 1.0)
pub const DEFAULT_THRESHOLD: f64 = 0.7;

/// Default maximum number of suggestions
pub const DEFAULT_MAX_SUGGESTIONS: usize = 3;

/// Find the most similar string to the input from a list of candidates
///
/// Returns the best match if its similarity score is above the threshold.
///
/// # Arguments
/// * `input` - The string to find a match for
/// * `candidates` - A list of possible matches
///
/// # Example
/// ```
/// use xx::suggest;
///
/// let commands = vec!["build", "test", "run", "clean"];
/// assert_eq!(suggest::similar("biuld", &commands), Some("build".to_string()));
/// assert_eq!(suggest::similar("xyz", &commands), None);
/// ```
pub fn similar<S, T>(input: S, candidates: &[T]) -> Option<String>
where
    S: AsRef<str>,
    T: AsRef<str>,
{
    similar_with_threshold(input, candidates, DEFAULT_THRESHOLD)
}

/// Find the most similar string with a custom threshold
///
/// # Arguments
/// * `input` - The string to find a match for
/// * `candidates` - A list of possible matches
/// * `threshold` - Minimum similarity score (0.0 to 1.0)
///
/// # Example
/// ```
/// use xx::suggest;
///
/// let commands = vec!["build", "test"];
/// // With lower threshold, more matches are possible
/// assert!(suggest::similar_with_threshold("bld", &commands, 0.5).is_some());
/// // With higher threshold, fewer matches
/// assert!(suggest::similar_with_threshold("bld", &commands, 0.9).is_none());
/// ```
pub fn similar_with_threshold<S, T>(input: S, candidates: &[T], threshold: f64) -> Option<String>
where
    S: AsRef<str>,
    T: AsRef<str>,
{
    let input = input.as_ref().to_lowercase();
    let mut best_match: Option<(String, f64)> = None;

    for candidate in candidates {
        let candidate_str = candidate.as_ref();
        let candidate_lower = candidate_str.to_lowercase();
        let score = jaro_winkler(&input, &candidate_lower);

        if score >= threshold {
            if let Some((_, best_score)) = &best_match {
                if score > *best_score {
                    best_match = Some((candidate_str.to_string(), score));
                }
            } else {
                best_match = Some((candidate_str.to_string(), score));
            }
        }
    }

    best_match.map(|(s, _)| s)
}

/// Find the N most similar strings to the input
///
/// Returns up to `n` matches sorted by similarity (best first).
///
/// # Arguments
/// * `input` - The string to find matches for
/// * `candidates` - A list of possible matches
/// * `n` - Maximum number of suggestions to return
///
/// # Example
/// ```
/// use xx::suggest;
///
/// let items = vec!["apple", "application", "apply", "banana"];
/// let suggestions = suggest::similar_n("app", &items, 3);
/// assert_eq!(suggestions.len(), 3);
/// // Returns ["apply", "apple", "application"] or similar order based on score
/// ```
pub fn similar_n<S, T>(input: S, candidates: &[T], n: usize) -> Vec<String>
where
    S: AsRef<str>,
    T: AsRef<str>,
{
    similar_n_with_threshold(input, candidates, n, DEFAULT_THRESHOLD)
}

/// Find the N most similar strings with a custom threshold
///
/// # Arguments
/// * `input` - The string to find matches for
/// * `candidates` - A list of possible matches
/// * `n` - Maximum number of suggestions to return
/// * `threshold` - Minimum similarity score (0.0 to 1.0)
pub fn similar_n_with_threshold<S, T>(
    input: S,
    candidates: &[T],
    n: usize,
    threshold: f64,
) -> Vec<String>
where
    S: AsRef<str>,
    T: AsRef<str>,
{
    let input = input.as_ref().to_lowercase();
    let mut scored: Vec<(String, f64)> = candidates
        .iter()
        .map(|c| {
            let candidate_str = c.as_ref();
            let score = jaro_winkler(&input, &candidate_str.to_lowercase());
            (candidate_str.to_string(), score)
        })
        .filter(|(_, score)| *score >= threshold)
        .collect();

    // Sort by score descending
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    scored.into_iter().take(n).map(|(s, _)| s).collect()
}

/// Calculate the similarity score between two strings
///
/// Returns a score between 0.0 (completely different) and 1.0 (identical).
/// Uses the Jaro-Winkler algorithm which gives higher scores to strings
/// that match from the beginning.
///
/// # Example
/// ```
/// use xx::suggest;
///
/// let score = suggest::similarity("hello", "hallo");
/// assert!(score > 0.8);
///
/// let score = suggest::similarity("abc", "xyz");
/// assert!(score < 0.5);
/// ```
pub fn similarity<S, T>(a: S, b: T) -> f64
where
    S: AsRef<str>,
    T: AsRef<str>,
{
    jaro_winkler(a.as_ref(), b.as_ref())
}

/// Format a "did you mean?" message
///
/// Returns None if no similar string is found.
///
/// # Example
/// ```
/// use xx::suggest;
///
/// let commands = vec!["build", "test", "run"];
/// if let Some(msg) = suggest::did_you_mean("biuld", &commands) {
///     println!("{}", msg); // "Did you mean 'build'?"
/// }
/// ```
pub fn did_you_mean<S, T>(input: S, candidates: &[T]) -> Option<String>
where
    S: AsRef<str>,
    T: AsRef<str>,
{
    similar(&input, candidates).map(|s| format!("Did you mean '{}'?", s))
}

/// Format a "did you mean?" message with multiple suggestions
///
/// # Example
/// ```
/// use xx::suggest;
///
/// let items = vec!["apple", "application", "apply"];
/// if let Some(msg) = suggest::did_you_mean_n("app", &items, 2) {
///     println!("{}", msg); // "Did you mean one of: 'apply', 'apple'?"
/// }
/// ```
pub fn did_you_mean_n<S, T>(input: S, candidates: &[T], n: usize) -> Option<String>
where
    S: AsRef<str>,
    T: AsRef<str>,
{
    let suggestions = similar_n(&input, candidates, n);
    if suggestions.is_empty() {
        None
    } else if suggestions.len() == 1 {
        Some(format!("Did you mean '{}'?", suggestions[0]))
    } else {
        let formatted: Vec<String> = suggestions.iter().map(|s| format!("'{}'", s)).collect();
        Some(format!("Did you mean one of: {}?", formatted.join(", ")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_similar() {
        let candidates = vec!["install", "uninstall", "update", "upgrade"];

        assert_eq!(similar("instal", &candidates), Some("install".to_string()));
        assert_eq!(
            similar("unintsall", &candidates),
            Some("uninstall".to_string())
        );
        assert_eq!(similar("updat", &candidates), Some("update".to_string()));
        assert_eq!(similar("xyz123", &candidates), None);
    }

    #[test]
    fn test_similar_case_insensitive() {
        let candidates = vec!["Install", "Update"];

        assert_eq!(similar("install", &candidates), Some("Install".to_string()));
        assert_eq!(similar("UPDATE", &candidates), Some("Update".to_string()));
    }

    #[test]
    fn test_similar_n() {
        let candidates = vec!["apple", "application", "apply", "banana", "appreciate"];

        let suggestions = similar_n("app", &candidates, 3);
        assert!(suggestions.len() <= 3);
        // All suggestions should be app-related
        for s in &suggestions {
            assert!(s.to_lowercase().starts_with("app"));
        }
    }

    #[test]
    fn test_similarity_score() {
        assert!(similarity("hello", "hello") > 0.99);
        assert!(similarity("hello", "hallo") > 0.8);
        assert!(similarity("abc", "xyz") < 0.5);
    }

    #[test]
    fn test_did_you_mean() {
        let commands = vec!["build", "test", "run"];

        let msg = did_you_mean("biuld", &commands);
        assert!(msg.is_some());
        assert!(msg.unwrap().contains("build"));

        let msg = did_you_mean("xyz", &commands);
        assert!(msg.is_none());
    }

    #[test]
    fn test_did_you_mean_n() {
        let items = vec!["apple", "application", "apply"];

        let msg = did_you_mean_n("app", &items, 2);
        assert!(msg.is_some());
        let msg = msg.unwrap();
        assert!(msg.contains("one of") || msg.contains("Did you mean"));
    }

    #[test]
    fn test_threshold() {
        let candidates = vec!["test"];

        // Low threshold should match
        assert!(similar_with_threshold("tst", &candidates, 0.5).is_some());

        // Very high threshold should not match
        assert!(similar_with_threshold("tst", &candidates, 0.99).is_none());
    }
}
