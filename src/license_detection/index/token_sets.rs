//! Token set and multiset utilities for license detection.

use std::collections::{HashMap, HashSet};

/// Build a token ID set and multiset from a sequence of token IDs.
///
/// The set contains unique token IDs, while the multiset (bag) contains
/// all token IDs with their occurrence counts.
///
/// Corresponds to Python: `build_set_and_tids_mset()` in match_set.py
///
/// # Arguments
///
/// * `token_ids` - Sequence of token IDs
///
/// # Returns
///
/// A tuple of (set of unique token IDs, multiset as HashMap of token ID -> count)
pub fn build_set_and_mset(token_ids: &[u16]) -> (HashSet<u16>, HashMap<u16, usize>) {
    let mut tids_mset = HashMap::new();

    for &tid in token_ids {
        *tids_mset.entry(tid).or_insert(0) += 1;
    }

    let tids_set: HashSet<u16> = tids_mset.keys().copied().collect();

    (tids_set, tids_mset)
}

/// Count unique tokens in a set (equivalent to Python's `len()` for intbitset).
///
/// Corresponds to Python: `tids_set_counter = len` (line 116 in match_set.py)
///
/// # Arguments
///
/// * `tids_set` - Set of unique token IDs
///
/// # Returns
///
/// Number of unique tokens in the set
pub fn tids_set_counter(tids_set: &HashSet<u16>) -> usize {
    tids_set.len()
}

/// Count total occurrences of tokens in a multiset.
///
/// Corresponds to Python: `multiset_counter()` in match_set.py (line 140)
///
/// # Arguments
///
/// * `mset` - Multiset as HashMap of token ID -> count
///
/// # Returns
///
/// Sum of all occurrence counts in the multiset
pub fn multiset_counter(mset: &HashMap<u16, usize>) -> usize {
    mset.values().sum()
}

/// Get subset of a token set containing only high-value (legalese) tokens.
///
/// High-value tokens are those with IDs less than len_legalese.
///
/// Corresponds to Python: `high_tids_set_subset()` in match_set.py (line 148)
///
/// # Arguments
///
/// * `tids_set` - Set of token IDs
/// * `len_legalese` - Number of legalese tokens (IDs < this are high-value)
///
/// # Returns
///
/// Subset of tids_set containing only token IDs < len_legalese
pub fn high_tids_set_subset(tids_set: &HashSet<u16>, len_legalese: usize) -> HashSet<u16> {
    tids_set
        .iter()
        .filter(|&&tid| (tid as usize) < len_legalese)
        .copied()
        .collect()
}

/// Get subset of a multiset containing only high-value (legalese) tokens.
///
/// Corresponds to Python: `high_tids_multiset_subset()` in match_set.py (line 155)
///
/// # Arguments
///
/// * `mset` - Multiset as HashMap of token ID -> count
/// * `len_legalese` - Number of legalese tokens (IDs < this are high-value)
///
/// # Returns
///
/// Subset of mset containing only token IDs < len_legalese
pub fn high_multiset_subset(
    mset: &HashMap<u16, usize>,
    len_legalese: usize,
) -> HashMap<u16, usize> {
    mset.iter()
        .filter(|(tid, _)| (**tid as usize) < len_legalese)
        .map(|(&tid, &count)| (tid, count))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_set_and_mset() {
        let token_ids = vec![1, 2, 3, 2, 4, 1, 1];
        let (tids_set, tids_mset) = build_set_and_mset(&token_ids);

        assert_eq!(tids_set.len(), 4);
        assert!(tids_set.contains(&1));
        assert!(tids_set.contains(&2));
        assert!(tids_set.contains(&3));
        assert!(tids_set.contains(&4));

        assert_eq!(tids_mset.get(&1), Some(&3));
        assert_eq!(tids_mset.get(&2), Some(&2));
        assert_eq!(tids_mset.get(&3), Some(&1));
        assert_eq!(tids_mset.get(&4), Some(&1));
    }

    #[test]
    fn test_build_set_and_mset_empty() {
        let token_ids: Vec<u16> = vec![];
        let (tids_set, tids_mset) = build_set_and_mset(&token_ids);

        assert_eq!(tids_set.len(), 0);
        assert_eq!(tids_mset.len(), 0);
    }

    #[test]
    fn test_tids_set_counter() {
        let mut set = HashSet::new();
        set.insert(1);
        set.insert(2);
        set.insert(3);
        assert_eq!(tids_set_counter(&set), 3);
    }

    #[test]
    fn test_multiset_counter() {
        let mut mset = HashMap::new();
        mset.insert(1, 3);
        mset.insert(2, 2);
        mset.insert(3, 1);
        assert_eq!(multiset_counter(&mset), 6);
    }

    #[test]
    fn test_high_tids_set_subset() {
        let mut set = HashSet::new();
        set.insert(1);
        set.insert(2);
        set.insert(5);
        set.insert(10);

        let high_set = high_tids_set_subset(&set, 5);
        assert_eq!(high_set.len(), 2);
        assert!(high_set.contains(&1));
        assert!(high_set.contains(&2));
        assert!(!high_set.contains(&5));
        assert!(!high_set.contains(&10));
    }

    #[test]
    fn test_high_multiset_subset() {
        let mut mset = HashMap::new();
        mset.insert(1, 3);
        mset.insert(2, 2);
        mset.insert(5, 1);
        mset.insert(10, 1);

        let high_mset = high_multiset_subset(&mset, 5);
        assert_eq!(high_mset.len(), 2);
        assert_eq!(high_mset.get(&1), Some(&3));
        assert_eq!(high_mset.get(&2), Some(&2));
        assert!(!high_mset.contains_key(&5));
        assert!(!high_mset.contains_key(&10));
    }
}
