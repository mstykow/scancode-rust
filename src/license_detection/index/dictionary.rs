//! Token string to integer ID mapping.
//!
//! TokenDictionary maps token strings to unique integer IDs. This enables
//! efficient token-based matching and indexing.

use std::collections::HashMap;

/// Token dictionary mapping token strings to unique integer IDs.
///
/// Token IDs are assigned as follows:
/// - IDs 0 to len_legalese-1: Reserved for legalese tokens (high-value words)
/// - IDs len_legalese and above: Assigned to other tokens as encountered
///
/// The `len_legalese` delimiter allows the matching engine to distinguish
/// between high-value (legalese) tokens and regular tokens.
///
/// Based on the Python ScanCode Toolkit implementation at:
/// reference/scancode-toolkit/src/licensedcode/index.py
#[derive(Debug, Clone)]
pub struct TokenDictionary {
    /// Mapping from token string to token ID
    tokens_to_ids: HashMap<String, u16>,

    /// Number of legalese tokens (lower IDs = higher value)
    len_legalese: usize,

    /// Next token ID to assign (for non-legalese tokens)
    next_id: u16,
}

impl TokenDictionary {
    /// Create a new token dictionary initialized with legalese tokens.
    ///
    /// This follows the Python ScanCode TorchToolkit pattern where the dictionary
    /// starts with pre-defined legalese words that get low IDs (high value).
    ///
    /// # Arguments
    /// * `legalese_entries` - Slice of (word, token_id) pairs for legalese words
    ///
    /// # Returns
    /// A new TokenDictionary instance with legalese tokens pre-populated
    pub fn new_with_legalese(legalese_entries: &[(&str, u16)]) -> Self {
        let mut tokens_to_ids = HashMap::new();

        for (word, token_id) in legalese_entries {
            tokens_to_ids.insert(word.to_string(), *token_id);
        }

        let len_legalese = legalese_entries.len();
        let next_id = len_legalese as u16;

        Self {
            tokens_to_ids,
            len_legalese,
            next_id,
        }
    }

    /// Create a new empty token dictionary (for testing).
    ///
    /// # Arguments
    /// * `legalese_count` - Number of reserved legalese token IDs
    ///
    /// # Returns
    /// A new TokenDictionary instance
    pub fn new(legalese_count: usize) -> Self {
        Self {
            tokens_to_ids: HashMap::new(),
            len_legalese: legalese_count,
            next_id: legalese_count as u16,
        }
    }

    /// Get or assign a token ID for a token string.
    ///
    /// If the token already exists, returns its existing ID.
    /// If it's a new token, assigns it the next available ID.
    ///
    /// This follows the Python ScanCode Toolkit pattern in index.py where
    /// new tokens encountered during indexing get sequential IDs.
    ///
    /// # Arguments
    /// * `token` - The token string
    ///
    /// # Returns
    /// The token ID
    pub fn get_or_assign(&mut self, token: &str) -> u16 {
        if let Some(&id) = self.tokens_to_ids.get(token) {
            return id;
        }

        let id = self.next_id;
        self.next_id += 1;
        self.tokens_to_ids.insert(token.to_string(), id);
        id
    }

    /// Get the token ID for a token string if it exists.
    ///
    /// # Arguments
    /// * `token` - The token string
    ///
    /// # Returns
    /// Some(token_id) if the token exists, None otherwise
    pub fn get_token_id(&self, token: &str) -> Option<u16> {
        self.tokens_to_ids.get(token).copied()
    }

    /// Get the token ID (alias for backward compatibility).
    #[inline]
    pub fn get(&self, token: &str) -> Option<u16> {
        self.get_token_id(token)
    }

    /// Check if a token ID is a legalese (high-value) token.
    ///
    /// This follows the Python ScanCode Toolkit pattern where tokens with
    /// IDs < len_legalese are considered high-value legalese tokens.
    ///
    /// # Arguments
    /// * `token_id` - The token ID
    ///
    /// # Returns
    /// true if the token ID is in the legalese range
    #[inline]
    pub const fn is_legalese_token(&self, token_id: u16) -> bool {
        token_id < self.len_legalese as u16
    }

    /// Check if a token ID is a legalese (high-value) token (alias).
    #[inline]
    pub const fn is_legalese(&self, token_id: u16) -> bool {
        self.is_legalese_token(token_id)
    }

    /// Get the number of registered tokens.
    pub fn len(&self) -> usize {
        self.tokens_to_ids.len()
    }

    /// Check if the dictionary is empty.
    pub fn is_empty(&self) -> bool {
        self.tokens_to_ids.is_empty()
    }

    /// Get the number of legalese tokens.
    pub const fn legalese_count(&self) -> usize {
        self.len_legalese
    }
}

impl Default for TokenDictionary {
    fn default() -> Self {
        Self::new(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_dictionary_new() {
        let dict = TokenDictionary::new(10);
        assert_eq!(dict.legalese_count(), 10);
        assert_eq!(dict.len(), 0);
        assert!(dict.is_empty());
    }

    #[test]
    fn test_new_with_legalese() {
        let legalese = [
            ("license".to_string(), 0u16),
            ("copyright".to_string(), 1u16),
            ("permission".to_string(), 2u16),
        ];

        let mut dict = TokenDictionary::new_with_legalese(
            &legalese
                .iter()
                .map(|(s, i)| (s.as_str(), *i))
                .collect::<Vec<_>>(),
        );

        assert_eq!(dict.legalese_count(), 3);
        assert_eq!(dict.len(), 3);
        assert!(!dict.is_empty());

        // Check that legalese tokens are registered
        assert_eq!(dict.get_token_id("license"), Some(0));
        assert_eq!(dict.get_token_id("copyright"), Some(1));
        assert_eq!(dict.get_token_id("permission"), Some(2));

        // Check that new tokens get IDs starting after legalese
        let test_id = dict.get_or_assign("test");
        assert_eq!(test_id, 3);
    }

    #[test]
    fn test_new_with_legalese_sorted() {
        let legalese = [
            ("copyright".to_string(), 5u16),
            ("license".to_string(), 0u16),
            ("permission".to_string(), 10u16),
        ];

        let mut dict = TokenDictionary::new_with_legalese(
            &legalese
                .iter()
                .map(|(s, i)| (s.as_str(), *i))
                .collect::<Vec<_>>(),
        );

        assert_eq!(dict.legalese_count(), 3);
        assert_eq!(dict.len(), 3);

        // Check legalese IDs are correct regardless of input order
        assert_eq!(dict.get_token_id("copyright"), Some(5));
        assert_eq!(dict.get_token_id("license"), Some(0));
        assert_eq!(dict.get_token_id("permission"), Some(10));

        // Next ID should be the count, not max + 1
        let test_id = dict.get_or_assign("test");
        assert_eq!(test_id, 3);
    }

    #[test]
    fn test_get_or_assign_new_token() {
        let mut dict = TokenDictionary::new(5);

        let id1 = dict.get_or_assign("hello");
        let id2 = dict.get_or_assign("world");

        // Should assign IDs starting at legalese_count (5)
        assert_eq!(id1, 5);
        assert_eq!(id2, 6);
        assert_eq!(dict.len(), 2);
    }

    #[test]
    fn test_get_or_assign_existing_token() {
        let mut dict = TokenDictionary::new(5);

        let id1 = dict.get_or_assign("hello");
        let id2 = dict.get_or_assign("hello");

        // Should return the same ID for the same token
        assert_eq!(id1, id2);
        assert_eq!(dict.len(), 1);
    }

    #[test]
    fn test_get_or_assign_with_preexisting_legalese() {
        let legalese = [("license".to_string(), 0u16)];
        let mut dict = TokenDictionary::new_with_legalese(
            &legalese
                .iter()
                .map(|(s, i)| (s.as_str(), *i))
                .collect::<Vec<_>>(),
        );

        // Legalese tokens should already exist
        let id = dict.get_or_assign("license");
        assert_eq!(id, 0);
        assert_eq!(dict.len(), 1);

        // New tokens should get IDs after legalese
        let new_id = dict.get_or_assign("new");
        assert_eq!(new_id, 1);
        assert_eq!(dict.len(), 2);
    }

    #[test]
    fn test_get_existing_token() {
        let mut dict = TokenDictionary::new(5);

        dict.get_or_assign("hello");
        assert_eq!(dict.get_token_id("hello"), Some(5));
    }

    #[test]
    fn test_get_nonexistent_token() {
        let dict = TokenDictionary::new(5);
        assert_eq!(dict.get_token_id("hello"), None);
    }

    #[test]
    fn test_is_legalese_token() {
        let dict = TokenDictionary::new(10);

        // IDs 0-9 are legalese
        assert!(dict.is_legalese_token(0));
        assert!(dict.is_legalese_token(5));
        assert!(dict.is_legalese_token(9));

        // ID 10+ are not legalese
        assert!(!dict.is_legalese_token(10));
        assert!(!dict.is_legalese_token(100));
    }

    #[test]
    fn test_is_legalese_token_with_actual_legalese() {
        let legalese = [
            ("license".to_string(), 0u16),
            ("copyright".to_string(), 1u16),
        ];

        let mut dict = TokenDictionary::new_with_legalese(
            &legalese
                .iter()
                .map(|(s, i)| (s.as_str(), *i))
                .collect::<Vec<_>>(),
        );

        // Legalese tokens should have IDs in the legalese range
        assert!(dict.is_legalese_token(dict.get_token_id("license").unwrap()));
        assert!(dict.is_legalese_token(dict.get_token_id("copyright").unwrap()));

        // Regular tokens should not be legalese
        let regular_id = dict.get_or_assign("regular");
        assert!(!dict.is_legalese_token(regular_id));
    }

    #[test]
    fn test_token_dictionary_default() {
        let dict = TokenDictionary::default();
        assert_eq!(dict.legalese_count(), 0);
        assert!(dict.is_empty());
    }

    #[test]
    fn test_get_alias() {
        let mut dict = TokenDictionary::new(5);
        dict.get_or_assign("hello");

        // get() should be an alias for get_token_id()
        assert_eq!(dict.get("hello"), dict.get_token_id("hello"));
    }

    #[test]
    fn test_is_legalese_alias() {
        let dict = TokenDictionary::new(10);

        // is_legalese() should be an alias for is_legalese_token()
        for id in 0..20 {
            assert_eq!(dict.is_legalese(id), dict.is_legalese_token(id));
        }
    }

    #[test]
    fn test_with_actual_legalese_module() {
        use crate::license_detection::rules::legalese;

        let legalese_words = legalese::get_legalese_words();
        assert!(!legalese_words.is_empty(), "Should have legalese words");

        let mut dict = TokenDictionary::new_with_legalese(&legalese_words);

        // Verify dictionary has the right structure
        assert_eq!(dict.legalese_count(), legalese_words.len());
        assert_eq!(dict.len(), legalese_words.len());

        // Verify some legalese words are correctly registered
        let license_id = dict.get_token_id("license");
        assert!(license_id.is_some(), "License should be in dictionary");
        assert!(
            dict.is_legalese_token(license_id.unwrap()),
            "License should be a legalese token"
        );

        // Note: Standalone "copyright" is NOT in the Python reference dictionary
        // Only compound words like "copyrighted", "copyrights" are present
        let copyrighted_id = dict.get_token_id("copyrighted");
        assert!(
            copyrighted_id.is_some(),
            "Copyrighted should be in dictionary"
        );
        assert!(
            dict.is_legalese_token(copyrighted_id.unwrap()),
            "Copyrighted should be a legalese token"
        );

        // New tokens should get IDs after legalese
        let hello_id = dict.get_or_assign("hello");
        assert!(hello_id >= dict.legalese_count() as u16);
        assert!(!dict.is_legalese_token(hello_id));

        // Token with same ID should have legalese status
        assert!(dict.is_legalese(license_id.unwrap()));
        assert!(dict.is_legalese(copyrighted_id.unwrap()));
        assert!(!dict.is_legalese(hello_id));
    }
}
