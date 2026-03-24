//! Sorted Vec helpers for efficient merge and comparison operations.

/// Merge two sorted slices, removing duplicates.
/// O(n+m) time complexity, single allocation.
pub fn merge_sorted_dedup(a: &[usize], b: &[usize]) -> Vec<usize> {
    let mut result = Vec::with_capacity(a.len() + b.len());
    let (mut ai, mut bi) = (0, 0);

    while ai < a.len() && bi < b.len() {
        match a[ai].cmp(&b[bi]) {
            std::cmp::Ordering::Less => {
                result.push(a[ai]);
                ai += 1;
            }
            std::cmp::Ordering::Equal => {
                result.push(a[ai]);
                ai += 1;
                bi += 1;
            }
            std::cmp::Ordering::Greater => {
                result.push(b[bi]);
                bi += 1;
            }
        }
    }

    result.extend_from_slice(&a[ai..]);
    result.extend_from_slice(&b[bi..]);

    result
}

/// Compare two sorted slices for equality.
/// O(n) time complexity, no allocation.
pub fn sorted_eq(a: &[usize], b: &[usize]) -> bool {
    a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x == y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_empty_a() {
        assert_eq!(merge_sorted_dedup(&[], &[1, 2, 3]), vec![1, 2, 3]);
    }

    #[test]
    fn test_merge_empty_b() {
        assert_eq!(merge_sorted_dedup(&[1, 2, 3], &[]), vec![1, 2, 3]);
    }

    #[test]
    fn test_merge_both_empty() {
        assert_eq!(merge_sorted_dedup(&[], &[]), Vec::<usize>::new());
    }

    #[test]
    fn test_merge_duplicates() {
        assert_eq!(merge_sorted_dedup(&[1, 2, 3], &[2, 3, 4]), vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_merge_identical() {
        assert_eq!(merge_sorted_dedup(&[1, 2, 3], &[1, 2, 3]), vec![1, 2, 3]);
    }

    #[test]
    fn test_merge_disjoint() {
        assert_eq!(merge_sorted_dedup(&[1, 2], &[5, 6]), vec![1, 2, 5, 6]);
    }

    #[test]
    fn test_merge_single_element_duplicate() {
        assert_eq!(merge_sorted_dedup(&[5], &[5]), vec![5]);
    }

    #[test]
    fn test_merge_single_element_different() {
        assert_eq!(merge_sorted_dedup(&[3], &[7]), vec![3, 7]);
    }

    #[test]
    fn test_merge_a_contains_b() {
        assert_eq!(
            merge_sorted_dedup(&[1, 2, 3, 4, 5], &[2, 3, 4]),
            vec![1, 2, 3, 4, 5]
        );
    }

    #[test]
    fn test_merge_b_contains_a() {
        assert_eq!(
            merge_sorted_dedup(&[2, 3, 4], &[1, 2, 3, 4, 5]),
            vec![1, 2, 3, 4, 5]
        );
    }

    #[test]
    fn test_sorted_eq_identical() {
        assert!(sorted_eq(&[1, 2, 3], &[1, 2, 3]));
    }

    #[test]
    fn test_sorted_eq_different_lengths() {
        assert!(!sorted_eq(&[1, 2, 3], &[1, 2]));
        assert!(!sorted_eq(&[1, 2], &[1, 2, 3]));
    }

    #[test]
    fn test_sorted_eq_different_values() {
        assert!(!sorted_eq(&[1, 2, 3], &[1, 2, 4]));
    }

    #[test]
    fn test_sorted_eq_both_empty() {
        let empty: Vec<usize> = Vec::new();
        assert!(sorted_eq(&empty, &empty));
    }

    #[test]
    fn test_sorted_eq_one_empty() {
        assert!(!sorted_eq(&[1], &[]));
        assert!(!sorted_eq(&[], &[1]));
    }
}
