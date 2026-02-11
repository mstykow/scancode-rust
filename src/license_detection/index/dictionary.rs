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
    /// Create a new token dictionary.
    ///
    /// # Arguments
    /// * `legalese_count` - Number of legalese tokens (reserved IDs 0..legalese_count-1)
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
    pub fn get(&self, token: &str) -> Option<u16> {
        self.tokens_to_ids.get(token).copied()
    }

    /// Check if a token ID is a legalese (high-value) token.
    ///
    /// # Arguments
    /// * `token_id` - The token ID
    ///
    /// # Returns
    /// true if the token ID is in the legalese range
    #[inline]
    pub const fn is_legalese(&self, token_id: u16) -> bool {
        token_id < self.len_legalese as u16
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
    fn test_get_existing_token() {
        let mut dict = TokenDictionary::new(5);

        dict.get_or_assign("hello");
        assert_eq!(dict.get("hello"), Some(5));
    }

    #[test]
    fn test_get_nonexistent_token() {
        let dict = TokenDictionary::new(5);
        assert_eq!(dict.get("hello"), None);
    }

    #[test]
    fn test_is_legalese() {
        let dict = TokenDictionary::new(10);

        // IDs 0-9 are legalese
        assert!(dict.is_legalese(0));
        assert!(dict.is_legalese(5));
        assert!(dict.is_legalese(9));

        // ID 10+ are not legalese
        assert!(!dict.is_legalese(10));
        assert!(!dict.is_legalese(100));
    }

    #[test]
    fn test_token_dictionary_default() {
        let dict = TokenDictionary::default();
        assert_eq!(dict.legalese_count(), 0);
        assert!(dict.is_empty());
    }
}
