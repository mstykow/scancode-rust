//! License index construction and querying.

pub mod dictionary;
pub mod token_sets;

use crate::license_detection::index::dictionary::TokenDictionary;
use crate::license_detection::models::License;
use aho_corasick::AhoCorasick;
use std::collections::{HashMap, HashSet};

/// Type alias for Aho-Corasick automaton.
///
/// The automaton is built from u16 token sequences encoded as bytes.
/// Each token is encoded as 2 bytes in little-endian format.
///
/// Based on the Python ScanCode Toolkit implementation at:
/// reference/scancode-toolkit/src/licensedcode/match_aho.py
pub type Automaton = AhoCorasick;

/// License index containing all data structures for efficient license detection.
///
/// The LicenseIndex holds multiple index structures that enable different matching
/// strategies: hash-based exact matching, Aho-Corasick automaton matching, set-based
/// candidate selection, and sequence matching.
///
/// Based on the Python ScanCode Toolkit implementation at:
/// reference/scancode-toolkit/src/licensedcode/index.py
///
/// # Index Structures
///
/// The index maintains several data structures for different matching strategies:
///
/// - **Hash matching**: `rid_by_hash` for exact hash-based matches
/// - **Automaton matching**: `rules_automaton` and `unknown_automaton` for pattern matching
/// - **Candidate selection**: `sets_by_rid` and `msets_by_rid` for set-based ranking
/// - **Sequence matching**: `high_postings_by_rid` for high-value token position tracking
/// - **Rule classification**: `regular_rids`, `false_positive_rids`, `approx_matchable_rids`
#[derive(Debug, Clone)]
pub struct LicenseIndex {
    /// Token dictionary mapping token strings to integer IDs.
    ///
    /// IDs 0 to len_legalese-1 are reserved for legalese tokens (high-value words).
    /// IDs len_legalese and above are assigned to other tokens as encountered.
    pub dictionary: TokenDictionary,

    /// Number of legalese tokens.
    ///
    /// Tokens with ID < len_legalese are considered high-value legalese words.
    /// Tokens with ID >= len_legalese are considered low-value tokens.
    ///
    /// Corresponds to Python: `self.len_legalese = 0` (line 185)
    pub len_legalese: usize,

    /// Set of token IDs made entirely of digits.
    ///
    /// These tokens can create worst-case behavior when there are long runs of them.
    ///
    /// Corresponds to Python: `self.digit_only_tids = set()` (line 191)
    pub digit_only_tids: HashSet<u16>,

    /// Mapping from rule hash to rule ID for hash-based exact matching.
    ///
    /// This enables fast exact matches using a hash of the rule\'s token IDs.
    /// Each hash maps to exactly one rule ID.
    ///
    /// Note: The hash is a 20-byte SHA1 digest, stored as a key in HashMap.
    /// In practice, we use a HashMap<[u8; 20], usize>.
    ///
    /// Corresponds to Python: `self.rid_by_hash = {}` (line 216)
    pub rid_by_hash: HashMap<[u8; 20], usize>,

    /// Rules indexed by rule ID.
    ///
    /// Maps rule IDs to Rule objects for quick lookup.
    ///
    /// Corresponds to Python: `self.rules_by_rid = []` (line 201)
    pub rules_by_rid: Vec<crate::license_detection::models::Rule>,

    /// Token ID sequences indexed by rule ID.
    ///
    /// Maps rule IDs to their token ID sequences.
    ///
    /// Corresponds to Python: `self.tids_by_rid = []` (line 204)
    pub tids_by_rid: Vec<Vec<u16>>,

    /// Aho-Corasick automaton built from all rule token sequences.
    ///
    /// Supports efficient multi-pattern matching of token ID sequences.
    /// Used for exact matching of complete rules or rule fragments in query text.
    ///
    /// Corresponds to Python: `self.rules_automaton = match_aho.get_automaton()` (line 219)
    pub rules_automaton: Automaton,

    /// Aho-Corasick automaton for unknown license detection.
    ///
    /// Separate automaton used to detect license-like text that doesn\'t match
    /// any known rule. Populated with ngrams from all approx-matchable rules.
    ///
    /// Corresponds to Python: `self.unknown_automaton = match_unknown.get_automaton()` (line 222)
    pub unknown_automaton: Automaton,

    /// Token ID sets per rule for candidate selection.
    ///
    /// Maps rule IDs to sets of unique token IDs present in that rule.
    /// Used for efficient candidate selection based on token overlap.
    ///
    /// Corresponds to Python: `self.sets_by_rid = []` (line 212)
    pub sets_by_rid: HashMap<usize, HashSet<u16>>,

    /// Token ID multisets per rule for candidate ranking.
    ///
    /// Maps rule IDs to multisets (bags) of token IDs with their frequencies.
    /// Used for ranking candidates by token frequency overlap.
    ///
    /// Corresponds to Python: `self.msets_by_rid = []` (line 213)
    pub msets_by_rid: HashMap<usize, HashMap<u16, usize>>,

    /// Inverted index of high-value token positions per rule.
    ///
    /// Maps rule IDs to a mapping from high-value token IDs to their positions
    /// within the rule. Only contains positions for tokens with IDs < len_legalese.
    ///
    /// This structure speeds up sequence matching by allowing quick lookup of
    /// where high-value tokens appear in each rule.
    ///
    /// Corresponds to Python: `self.high_postings_by_rid = []` (line 209)
    /// In Python: `postings = {tid: array('h', [positions, ...])}`
    pub high_postings_by_rid: HashMap<usize, HashMap<u16, Vec<usize>>>,

    /// Set of rule IDs for regular (non-false-positive) rules.
    ///
    /// Regular rules participate in all matching strategies including set
    /// matching and sequence matching.
    ///
    /// Corresponds to Python: `self.regular_rids = set()` (line 228)
    pub regular_rids: HashSet<usize>,

    /// Set of rule IDs for false positive rules.
    ///
    /// False positive rules are used for exact matching and post-matching
    /// filtering to subtract spurious matches.
    ///
    /// Corresponds to Python: `self.false_positive_rids = set()` (line 230)
    pub false_positive_rids: HashSet<usize>,

    /// Set of rule IDs that can be matched approximately.
    ///
    /// Only rules marked as approx-matchable participate in sequence matching.
    /// Other rules can only be matched exactly using the automaton.
    ///
    /// Corresponds to Python: `self.approx_matchable_rids = set()` (line 234)
    pub approx_matchable_rids: HashSet<usize>,

    /// Mapping from ScanCode license key to License object.
    ///
    /// Provides access to license metadata for building SPDX mappings
    /// and validating license expressions.
    ///
    /// Corresponds to Python: `get_licenses_db()` in models.py
    pub licenses_by_key: HashMap<String, crate::license_detection::models::License>,
}

impl LicenseIndex {
    /// Get the rule ID for a hash, if it exists.
    ///
    /// # Arguments
    /// * `hash` - The 20-byte SHA1 hash
    ///
    /// # Returns
    /// Option containing the rule ID, or None if hash not found
    pub fn get_rid_by_hash(&self, hash: &[u8; 20]) -> Option<&usize> {
        self.rid_by_hash.get(hash)
    }

    /// Get a license by its key.
    ///
    /// # Arguments
    /// * `key` - ScanCode license key (lowercase)
    ///
    /// # Returns
    /// Option containing the License, or None if not found
    pub fn get_license(&self, key: &str) -> Option<&License> {
        self.licenses_by_key.get(key)
    }

    /// Add a license to the index.
    ///
    /// # Arguments
    /// * `license` - License to add
    pub fn add_license(&mut self, license: License) {
        self.licenses_by_key.insert(license.key.clone(), license);
    }

    /// Add multiple licenses to the index.
    ///
    /// # Arguments
    /// * `licenses` - Slice of licenses to add
    pub fn add_licenses(&mut self, licenses: &[License]) {
        for license in licenses {
            self.add_license(license.clone());
        }
    }

    /// Get all ScanCode license keys.
    ///
    /// # Returns
    /// Iterator over all ScanCode license keys
    pub fn license_keys(&self) -> impl Iterator<Item = &String> {
        self.licenses_by_key.keys()
    }

    /// Get the number of licenses in the index.
    ///
    /// # Returns
    /// Count of licenses
    pub fn license_count(&self) -> usize {
        self.licenses_by_key.len()
    }
}

impl LicenseIndex {
    /// Create a new empty license index.
    ///
    /// This constructor initializes all index structures with empty collections.
    /// The index can be populated with rules using the indexing methods (to be
    /// implemented in future phases).
    ///
    /// # Returns
    /// A new LicenseIndex instance with empty index structures
    pub fn new(dictionary: TokenDictionary) -> Self {
        let len_legalese = dictionary.legalese_count();
        Self {
            dictionary,
            len_legalese,
            digit_only_tids: HashSet::new(),
            rid_by_hash: HashMap::new(),
            rules_by_rid: Vec::new(),
            tids_by_rid: Vec::new(),
            rules_automaton: Automaton::new(std::iter::empty::<&[u8]>())
                .expect("Failed to create empty automaton"),
            unknown_automaton: Automaton::new(std::iter::empty::<&[u8]>())
                .expect("Failed to create empty automaton"),
            sets_by_rid: HashMap::new(),
            msets_by_rid: HashMap::new(),
            high_postings_by_rid: HashMap::new(),
            regular_rids: HashSet::new(),
            false_positive_rids: HashSet::new(),
            approx_matchable_rids: HashSet::new(),
            licenses_by_key: HashMap::new(),
        }
    }

    /// Create a new empty license index with the specified legalese count.
    ///
    /// Convenience method that creates a new TokenDictionary and LicenseIndex
    /// in one call.
    ///
    /// # Arguments
    /// * `legalese_count` - Number of reserved legalese token IDs
    ///
    /// # Returns
    /// A new LicenseIndex instance with a new TokenDictionary
    pub fn with_legalese_count(legalese_count: usize) -> Self {
        Self::new(TokenDictionary::new(legalese_count))
    }
}

impl Default for LicenseIndex {
    fn default() -> Self {
        Self::with_legalese_count(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_license_index_new() {
        let dict = TokenDictionary::new(10);
        let index = LicenseIndex::new(dict);

        assert_eq!(index.dictionary.legalese_count(), 10);
        assert!(index.rid_by_hash.is_empty());
        assert!(index.sets_by_rid.is_empty());
        assert!(index.msets_by_rid.is_empty());
        assert!(index.high_postings_by_rid.is_empty());
        assert!(index.regular_rids.is_empty());
        assert!(index.false_positive_rids.is_empty());
        assert!(index.approx_matchable_rids.is_empty());
        assert!(index.licenses_by_key.is_empty());
    }

    #[test]
    fn test_license_index_with_legalese_count() {
        let index = LicenseIndex::with_legalese_count(15);

        assert_eq!(index.dictionary.legalese_count(), 15);
        assert!(index.rid_by_hash.is_empty());
    }

    #[test]
    fn test_license_index_default() {
        let index = LicenseIndex::default();

        assert_eq!(index.dictionary.legalese_count(), 0);
        assert!(index.rid_by_hash.is_empty());
    }

    #[test]
    fn test_automaton_default() {
        let automaton =
            Automaton::new(std::iter::empty::<&[u8]>()).expect("Failed to create automaton");
        let _ = format!("{:?}", automaton);
    }

    #[test]
    fn test_license_index_clone() {
        let index = LicenseIndex::with_legalese_count(5);
        let cloned = index.clone();

        assert_eq!(cloned.dictionary.legalese_count(), 5);
        assert!(cloned.rid_by_hash.is_empty());
    }

    #[test]
    fn test_license_index_add_license() {
        let mut index = LicenseIndex::default();

        let license = License {
            key: "test-license".to_string(),
            name: "Test License".to_string(),
            spdx_license_key: Some("TEST".to_string()),
            category: Some("Permissive".to_string()),
            text: "Test license text".to_string(),
            reference_urls: vec![],
            notes: None,
        };

        index.add_license(license);

        assert_eq!(index.license_count(), 1);
        assert!(index.get_license("test-license").is_some());
    }

    #[test]
    fn test_license_index_add_licenses() {
        let mut index = LicenseIndex::default();

        let licenses = vec![
            License {
                key: "license-1".to_string(),
                name: "License 1".to_string(),
                spdx_license_key: Some("LIC1".to_string()),
                category: Some("Permissive".to_string()),
                text: "License 1 text".to_string(),
                reference_urls: vec![],
                notes: None,
            },
            License {
                key: "license-2".to_string(),
                name: "License 2".to_string(),
                spdx_license_key: Some("LIC2".to_string()),
                category: Some("Copyleft".to_string()),
                text: "License 2 text".to_string(),
                reference_urls: vec![],
                notes: None,
            },
        ];

        index.add_licenses(&licenses);

        assert_eq!(index.license_count(), 2);
        assert!(index.get_license("license-1").is_some());
        assert!(index.get_license("license-2").is_some());
    }

    #[test]
    fn test_license_index_get_license() {
        let mut index = LicenseIndex::default();

        let license = License {
            key: "mit".to_string(),
            name: "MIT License".to_string(),
            spdx_license_key: Some("MIT".to_string()),
            category: Some("Permissive".to_string()),
            text: "MIT License text".to_string(),
            reference_urls: vec![],
            notes: None,
        };

        index.add_license(license);

        let retrieved = index.get_license("mit");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "MIT License");

        assert!(index.get_license("unknown").is_none());
    }

    #[test]
    fn test_license_index_license_count() {
        let mut index = LicenseIndex::default();

        assert_eq!(index.license_count(), 0);

        let license = License {
            key: "test".to_string(),
            name: "Test".to_string(),
            spdx_license_key: Some("TEST".to_string()),
            category: Some("Permissive".to_string()),
            text: "Text".to_string(),
            reference_urls: vec![],
            notes: None,
        };

        index.add_license(license);

        assert_eq!(index.license_count(), 1);
    }
}
