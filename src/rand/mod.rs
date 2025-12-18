//! Random generation utilities
//!
//! This module provides utilities for generating random values,
//! including human-readable random names.

use rand::prelude::*;

// Word lists inspired by https://github.com/nishanths/rust-haikunator
mod adjectives;
mod adverbs;
mod nouns;

use adjectives::ADJECTIVES;
use adverbs::ADVERBS;
use nouns::NOUNS;

/// Options for generating haiku-style random names
#[derive(Debug, Clone)]
pub struct HaikuOptions<'a> {
    /// Number of words to include (default: 2)
    pub words: usize,
    /// Separator between words (default: "-")
    pub separator: &'a str,
    /// Number of digits to append, or 0 for none (default: 2)
    pub digits: usize,
}

impl Default for HaikuOptions<'_> {
    fn default() -> Self {
        Self {
            words: 2,
            separator: "-",
            digits: 2,
        }
    }
}

/// Generate a haiku-style random name
///
/// Generates a poetic-themed random name by combining adverbs, adjectives,
/// and nouns, optionally followed by a random number. Useful for generating
/// unique identifiers with memorable, human-readable names.
///
/// The word pattern is:
/// - 1 word: noun
/// - 2 words: adjective-noun
/// - 3 words: adverb-adjective-noun
/// - 4+ words: adverb-adjective-noun-noun...
///
/// # Examples
///
/// ```
/// use xx::rand::{haiku, HaikuOptions};
///
/// // Default: 2 words + 2-digit number
/// let name = haiku(&HaikuOptions::default());
/// // e.g., "silent-forest-42"
///
/// // Custom: 3 words, no number
/// let name = haiku(&HaikuOptions {
///     words: 3,
///     digits: 0,
///     ..Default::default()
/// });
/// // e.g., "softly-falling-rain"
///
/// // Custom separator and more digits
/// let name = haiku(&HaikuOptions {
///     separator: "_",
///     digits: 4,
///     ..Default::default()
/// });
/// // e.g., "misty_dawn_8472"
/// ```
pub fn haiku(options: &HaikuOptions) -> String {
    let mut rng = rand::rng();
    let words = options.words.max(1);
    let mut parts: Vec<String> = Vec::with_capacity(words + 1);

    // Fixed pattern: [adverb]-[adjective]-noun[-noun...]
    // 1 word: noun
    // 2 words: adjective-noun
    // 3 words: adverb-adjective-noun
    // 4+ words: adverb-adjective-noun-noun...
    for i in 0..words {
        let word = match (words, i) {
            (1, 0) => *NOUNS.choose(&mut rng).unwrap(),
            (2, 0) => *ADJECTIVES.choose(&mut rng).unwrap(),
            (2, 1) => *NOUNS.choose(&mut rng).unwrap(),
            (_, 0) => *ADVERBS.choose(&mut rng).unwrap(),
            (_, 1) => *ADJECTIVES.choose(&mut rng).unwrap(),
            (_, _) => *NOUNS.choose(&mut rng).unwrap(),
        };
        parts.push(word.to_string());
    }

    if options.digits > 0 {
        let max = 10_u32.pow(options.digits as u32);
        let num: u32 = rng.random_range(0..max);
        parts.push(format!("{:0width$}", num, width = options.digits));
    }

    parts.join(options.separator)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_haiku_default() {
        let name = haiku(&HaikuOptions::default());
        let parts: Vec<&str> = name.split('-').collect();
        assert_eq!(parts.len(), 3);
        assert!(!parts[0].is_empty());
        assert!(!parts[1].is_empty());
        assert_eq!(parts[2].len(), 2); // 2 digits
        assert!(parts[2].parse::<u32>().is_ok());
    }

    #[test]
    fn test_haiku_no_number() {
        let name = haiku(&HaikuOptions {
            digits: 0,
            ..Default::default()
        });
        let parts: Vec<&str> = name.split('-').collect();
        assert_eq!(parts.len(), 2);
        assert!(parts.iter().all(|p| p.parse::<u32>().is_err()));
    }

    #[test]
    fn test_haiku_custom_separator() {
        let name = haiku(&HaikuOptions {
            separator: "_",
            ..Default::default()
        });
        assert!(name.contains('_'));
        assert!(!name.contains('-'));
    }

    #[test]
    fn test_haiku_three_words() {
        let name = haiku(&HaikuOptions {
            words: 3,
            digits: 0,
            ..Default::default()
        });
        let parts: Vec<&str> = name.split('-').collect();
        assert_eq!(parts.len(), 3);
    }

    #[test]
    fn test_haiku_four_digits() {
        let name = haiku(&HaikuOptions {
            digits: 4,
            ..Default::default()
        });
        let parts: Vec<&str> = name.split('-').collect();
        assert_eq!(parts[2].len(), 4);
    }
}
