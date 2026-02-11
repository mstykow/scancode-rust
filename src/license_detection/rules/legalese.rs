//! Common license-specific word dictionary (legalese).
//!
//! This module defines legalese tokens - common words specific to licenses
//! that are high-value for license detection. These words get lower token IDs,
//! making them more significant during matching.

use std::collections::HashMap;
use std::sync::LazyLock;

/// Legalese dictionary mapping common legal words to token IDs.
///
/// Legalese words are high-value tokens that get lower IDs (used during
/// matching to prioritize matches with more legalese words).
///
/// Multiple words can map to the same token ID when they are considered
/// equivalent (e.g., different spellings or British/US variants).
///
/// # TODO
/// This is a minimal sample. Full implementation will include all ~4000+
/// legalese words from the Python reference.
pub static LEGALESE: LazyLock<HashMap<String, u16>> = LazyLock::new(|| {
    let mut map = HashMap::new();

    // Most common legal terms (sample of ~100 words)
    map.insert("license".to_string(), 0);
    map.insert("licence".to_string(), 0);
    map.insert("copyright".to_string(), 1);
    map.insert("redistribute".to_string(), 2);
    map.insert("permit".to_string(), 3);
    map.insert("permission".to_string(), 4);
    map.insert("derivative".to_string(), 5);
    map.insert("derivative_works".to_string(), 5);
    map.insert("commercial".to_string(), 6);
    map.insert("noncommercial".to_string(), 7);
    map.insert("agreement".to_string(), 8);
    map.insert("warranty".to_string(), 9);
    map.insert("disclaimer".to_string(), 10);
    map.insert("liability".to_string(), 11);
    map.insert("contribute".to_string(), 12);
    map.insert("contribution".to_string(), 13);
    map.insert("modification".to_string(), 14);
    map.insert("modify".to_string(), 15);
    map.insert("restriction".to_string(), 16);
    map.insert("intellectual".to_string(), 17);
    map.insert("property".to_string(), 18);
    map.insert("patent".to_string(), 19);
    map.insert("trademark".to_string(), 20);
    map.insert("notice".to_string(), 21);
    map.insert("conditions".to_string(), 22);
    map.insert("obligate".to_string(), 23);
    map.insert("obligation".to_string(), 24);
    map.insert("enforceable".to_string(), 25);
    map.insert("statutory".to_string(), 26);
    map.insert("consequential".to_string(), 27);
    map.insert("indemnify".to_string(), 28);
    map.insert("indemnification".to_string(), 29);
    map.insert("accordance".to_string(), 30);
    map.insert("pursuant".to_string(), 31);
    map.insert("hereby".to_string(), 32);
    map.insert("hereunder".to_string(), 33);
    map.insert("hereinafter".to_string(), 34);
    map.insert("foregoing".to_string(), 35);
    map.insert("aforementioned".to_string(), 36);
    map.insert("notwithstanding".to_string(), 37);
    map.insert("terminate".to_string(), 38);
    map.insert("termination".to_string(), 39);
    map.insert("grant".to_string(), 40);
    map.insert("granted".to_string(), 41);
    map.insert("guarantee".to_string(), 42);
    map.insert("guaranty".to_string(), 42);
    map.insert("acknowledge".to_string(), 43);
    map.insert("acknowledgement".to_string(), 44);
    map.insert("warranty".to_string(), 45);
    map.insert("warranties".to_string(), 45);
    map.insert("express".to_string(), 46);
    map.insert("implied".to_string(), 47);
    map.insert("contract".to_string(), 48);
    map.insert("binding".to_string(), 49);

    // GPL-specific
    map.insert("gpl".to_string(), 50);
    map.insert("gnu".to_string(), 51);
    map.insert("general".to_string(), 52);
    map.insert("public".to_string(), 53);
    map.insert("copyleft".to_string(), 54);

    // MIT/BSD-specific
    map.insert("mit".to_string(), 55);
    map.insert("bsd".to_string(), 56);
    map.insert("apache".to_string(), 57);
    map.insert("mozilla".to_string(), 58);

    // Common legal phrases
    map.insert("as_is".to_string(), 59);
    map.insert("without_warranty".to_string(), 60);
    map.insert("all_rights_reserved".to_string(), 61);
    map.insert("permission_is_hereby".to_string(), 62);

    map
});

/// Get the legalese token ID for a word.
///
/// Returns Some(id) if the word is in the legalese dictionary,
/// or None if it's not a legalese word.
pub fn get_legalese_token(word: &str) -> Option<u16> {
    LEGALESE.get(word).copied()
}

/// Check if a word is a legalese word.
pub fn is_legalese(word: &str) -> bool {
    LEGALESE.contains_key(word)
}

/// Get the number of legalese tokens in the dictionary.
pub fn legalese_count() -> usize {
    LEGALESE.len()
}

/// Get the legalese words and their token IDs as a vector.
///
/// Returns a vector of (word, token_id) pairs sorted by token ID.
/// This is used to initialize the token dictionary with pre-assigned legalese tokens.
pub fn get_legalese_words() -> Vec<(&'static str, u16)> {
    let mut entries: Vec<(&str, u16)> = LEGALESE.iter().map(|(k, v)| (k.as_str(), *v)).collect();
    entries.sort_by_key(|&(_, id)| id);
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_legalese_token_for_known_word() {
        assert_eq!(get_legalese_token("license"), Some(0));
        assert_eq!(get_legalese_token("licence"), Some(0));
        assert_eq!(get_legalese_token("copyright"), Some(1));
    }

    #[test]
    fn test_get_legalese_token_for_unknown_word() {
        assert_eq!(get_legalese_token("hello"), None);
        assert_eq!(get_legalese_token("widget"), None);
    }

    #[test]
    fn test_is_legalese() {
        assert!(is_legalese("license"));
        assert!(is_legalese("copyright"));
        assert!(!is_legalese("hello"));
        assert!(!is_legalese("world"));
    }

    #[test]
    fn test_legalese_count() {
        // Ensure we have at least a reasonable number of sample words
        assert!(legalese_count() > 50);
    }

    #[test]
    fn test_license_equivalence() {
        // "license" and "licence" should map to the same token ID
        let id1 = get_legalese_token("license");
        let id2 = get_legalese_token("licence");
        assert_eq!(id1, id2);
        assert_eq!(id1, Some(0));
    }

    #[test]
    fn test_spdx_license_abbreviations() {
        // GPL abbreviations
        assert!(get_legalese_token("gpl").is_some());
        assert!(get_legalese_token("gnu").is_some());

        // Common short forms
        assert!(get_legalese_token("mit").is_some());
        assert!(get_legalese_token("bsd").is_some());
    }
}
