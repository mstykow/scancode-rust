//! Span - efficient integer range sets.
//!
//! Spans are used to track matched positions in license detection.

use std::ops::Range;

/// A span represents an efficient integer range set.
///
/// Spans are used to track ranges of text that have been matched,
/// allowing for merge, overlap detection, and other operations.
#[derive(Debug, Clone)]
pub struct Span {
    /// The ranges in this span
    #[allow(dead_code)]
    ranges: Vec<Range<usize>>,
}

impl Span {
    /// Create a new empty span.
    pub fn new() -> Self {
        Self { ranges: Vec::new() }
    }

    /// Create a span from a single range.
    pub fn from_range(range: Range<usize>) -> Self {
        Self {
            ranges: vec![range],
        }
    }

    /// Create a span from an iterator of positions.
    ///
    /// This converts individual positions into contiguous ranges.
    ///
    /// # Arguments
    /// * `positions` - Iterator over positions to include in the span
    pub fn from_iterator(positions: impl IntoIterator<Item = usize>) -> Self {
        let mut sorted: Vec<usize> = positions.into_iter().collect();
        sorted.sort_unstable();
        sorted.dedup();

        let mut ranges = Vec::new();
        let mut iter = sorted.into_iter().peekable();

        while let Some(start) = iter.next() {
            let mut end = start + 1;

            while let Some(&next) = iter.peek() {
                if next == end {
                    end += 1;
                    iter.next();
                } else {
                    break;
                }
            }

            ranges.push(start..end);
        }

        Self { ranges }
    }

    /// Add a range to this span, merging with existing ranges if needed.
    ///
    /// # Arguments
    /// * `range` - The range to add
    #[allow(dead_code)]
    pub fn add(&mut self, range: Range<usize>) {
        let mut new_range = range.clone();
        let mut to_remove = Vec::new();
        let mut has_overlap = false;

        for (i, existing) in self.ranges.iter().enumerate() {
            if self.ranges_overlap(&new_range, existing) {
                new_range = self.merge_ranges(&new_range, existing);
                to_remove.push(i);
                has_overlap = true;
            }
        }

        if has_overlap {
            to_remove.sort_by(|a, b| b.cmp(a));
            for i in to_remove {
                self.ranges.remove(i);
            }
        }
        self.ranges.push(new_range);
    }

    /// Check if two ranges overlap.
    #[allow(dead_code)]
    fn ranges_overlap(&self, r1: &Range<usize>, r2: &Range<usize>) -> bool {
        r1.start < r2.end && r2.start < r1.end
    }

    /// Merge two overlapping ranges.
    #[allow(dead_code)]
    fn merge_ranges(&self, r1: &Range<usize>, r2: &Range<usize>) -> Range<usize> {
        r1.start.min(r2.start)..r1.end.max(r2.end)
    }

    /// Check if this span is empty.
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.ranges.is_empty()
    }

    /// Get the number of ranges in this span.
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.ranges.len()
    }

    /// Get the total length covered by all ranges.
    #[allow(dead_code)]
    pub fn total_length(&self) -> usize {
        self.ranges.iter().map(|r| r.end - r.start).sum()
    }

    #[allow(dead_code)]
    pub fn overlap(&self, other: &Span) -> usize {
        let mut count = 0;
        for self_range in &self.ranges {
            for other_range in &other.ranges {
                let overlap_start = self_range.start.max(other_range.start);
                let overlap_end = self_range.end.min(other_range.end);
                if overlap_start < overlap_end {
                    count += overlap_end - overlap_start;
                }
            }
        }
        count
    }

    #[allow(dead_code)]
    pub fn overlap_ratio(&self, other: &Span) -> f64 {
        let overlap = self.overlap(other);
        let max_len = self.total_length().max(other.total_length());
        if max_len == 0 {
            0.0
        } else {
            overlap as f64 / max_len as f64
        }
    }

    pub fn union_span(&self, other: &Span) -> Span {
        let mut result = self.clone();
        for range in &other.ranges {
            result.add(range.clone());
        }
        result
    }

    pub fn intersects(&self, other: &Span) -> bool {
        for self_range in &self.ranges {
            for other_range in &other.ranges {
                if self_range.start < other_range.end && other_range.start < self_range.end {
                    return true;
                }
            }
        }
        false
    }
}

impl Default for Span {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_new() {
        let span = Span::new();
        assert!(span.is_empty());
        assert_eq!(span.len(), 0);
        assert_eq!(span.total_length(), 0);
    }

    #[test]
    fn test_span_from_range() {
        let span = Span::from_range(5..10);
        assert!(!span.is_empty());
        assert_eq!(span.len(), 1);
        assert_eq!(span.total_length(), 5);
    }

    #[test]
    fn test_span_add_non_overlapping() {
        let mut span = Span::new();
        span.add(5..10);
        span.add(20..25);

        assert_eq!(span.len(), 2);
        assert_eq!(span.total_length(), 10);
    }

    #[test]
    fn test_span_add_overlapping() {
        let mut span = Span::new();
        span.add(5..10);
        span.add(8..15);

        // Should merge into a single range
        assert_eq!(span.len(), 1);
        assert_eq!(span.total_length(), 10);
    }

    #[test]
    fn test_span_add_adjacent() {
        let mut span = Span::new();
        span.add(5..10);
        span.add(10..15);

        // Adjacent ranges don't overlap, so should remain separate
        assert_eq!(span.len(), 2);
    }

    #[test]
    fn test_span_add_multiple_overlapping() {
        let mut span = Span::new();
        span.add(5..10);
        span.add(8..15);
        span.add(12..20);

        // Should all merge into one range
        assert_eq!(span.len(), 1);
        assert_eq!(span.total_length(), 15);
    }

    #[test]
    fn test_span_default() {
        let span = Span::default();
        assert!(span.is_empty());
    }

    #[test]
    fn test_span_from_iterator_contiguous() {
        let span = Span::from_iterator(vec![1, 2, 3, 4, 5]);
        assert_eq!(span.len(), 1);
        assert_eq!(span.total_length(), 5);
    }

    #[test]
    fn test_span_from_iterator_non_contiguous() {
        let span = Span::from_iterator(vec![1, 2, 3, 10, 11, 12]);
        assert_eq!(span.len(), 2);
        assert_eq!(span.total_length(), 6);
    }

    #[test]
    fn test_span_from_iterator_unsorted() {
        let span = Span::from_iterator(vec![5, 1, 3, 2, 4]);
        assert_eq!(span.len(), 1);
        assert_eq!(span.total_length(), 5);
    }

    #[test]
    fn test_span_from_iterator_with_duplicates() {
        let span = Span::from_iterator(vec![1, 2, 2, 3, 3, 3, 4]);
        assert_eq!(span.len(), 1);
        assert_eq!(span.total_length(), 4);
    }

    #[test]
    fn test_span_from_iterator_empty() {
        let span: Span = Span::from_iterator(vec![]);
        assert!(span.is_empty());
        assert_eq!(span.total_length(), 0);
    }

    #[test]
    fn test_span_from_iterator_single_element() {
        let span = Span::from_iterator(vec![42]);
        assert_eq!(span.len(), 1);
        assert_eq!(span.total_length(), 1);
    }

    #[test]
    fn test_span_add_to_empty() {
        let mut span = Span::new();
        span.add(5..10);

        assert_eq!(span.len(), 1);
        assert_eq!(span.total_length(), 5);
    }

    #[test]
    fn test_span_add_contained_within_existing() {
        let mut span = Span::new();
        span.add(1..20);
        span.add(5..10);

        assert_eq!(span.len(), 1);
        assert_eq!(span.total_length(), 19);
    }

    #[test]
    fn test_span_add_existing_contained_in_new() {
        let mut span = Span::new();
        span.add(5..10);
        span.add(1..20);

        assert_eq!(span.len(), 1);
        assert_eq!(span.total_length(), 19);
    }

    #[test]
    fn test_span_add_chain_of_overlaps() {
        let mut span = Span::new();
        span.add(1..5);
        span.add(4..8);
        span.add(7..12);
        span.add(11..15);

        assert_eq!(span.len(), 1);
        assert_eq!(span.total_length(), 14);
    }

    #[test]
    fn test_span_multiple_separate_ranges() {
        let mut span = Span::new();
        span.add(1..5);
        span.add(10..15);
        span.add(20..25);

        assert_eq!(span.len(), 3);
        assert_eq!(span.total_length(), 14);
    }

    #[test]
    fn test_span_ranges_overlap_touching() {
        let span = Span::new();
        assert!(span.ranges_overlap(&(1..5), &(4..8)));
        assert!(span.ranges_overlap(&(4..8), &(1..5)));
    }

    #[test]
    fn test_span_ranges_overlap_non_touching() {
        let span = Span::new();
        assert!(!span.ranges_overlap(&(1..5), &(6..10)));
        assert!(!span.ranges_overlap(&(1..5), &(10..15)));
    }

    #[test]
    fn test_span_merge_ranges_basic() {
        let span = Span::new();
        let merged = span.merge_ranges(&(1..5), &(3..8));
        assert_eq!(merged.start, 1);
        assert_eq!(merged.end, 8);
    }

    #[test]
    fn test_span_merge_ranges_disjoint() {
        let span = Span::new();
        let merged = span.merge_ranges(&(1..5), &(10..15));
        assert_eq!(merged.start, 1);
        assert_eq!(merged.end, 15);
    }

    #[test]
    fn test_overlap_identical_spans() {
        let span1 = Span::from_range(5..10);
        let span2 = Span::from_range(5..10);
        assert_eq!(span1.overlap(&span2), 5);
        assert_eq!(span2.overlap(&span1), 5);
    }

    #[test]
    fn test_overlap_no_overlap() {
        let span1 = Span::from_range(1..5);
        let span2 = Span::from_range(10..15);
        assert_eq!(span1.overlap(&span2), 0);
        assert_eq!(span2.overlap(&span1), 0);
    }

    #[test]
    fn test_overlap_adjacent_no_overlap() {
        let span1 = Span::from_range(1..5);
        let span2 = Span::from_range(5..10);
        assert_eq!(span1.overlap(&span2), 0);
        assert_eq!(span2.overlap(&span1), 0);
    }

    #[test]
    fn test_overlap_partial_overlap() {
        let span1 = Span::from_range(1..5);
        let span2 = Span::from_range(3..8);
        assert_eq!(span1.overlap(&span2), 2);
        assert_eq!(span2.overlap(&span1), 2);
    }

    #[test]
    fn test_overlap_contained() {
        let span1 = Span::from_range(1..20);
        let span2 = Span::from_range(5..10);
        assert_eq!(span1.overlap(&span2), 5);
        assert_eq!(span2.overlap(&span1), 5);
    }

    #[test]
    fn test_overlap_empty_spans() {
        let span1 = Span::new();
        let span2 = Span::from_range(5..10);
        assert_eq!(span1.overlap(&span2), 0);
        assert_eq!(span2.overlap(&span1), 0);
    }

    #[test]
    fn test_overlap_both_empty() {
        let span1 = Span::new();
        let span2 = Span::new();
        assert_eq!(span1.overlap(&span2), 0);
    }

    #[test]
    fn test_overlap_multi_range_spans() {
        let mut span1 = Span::new();
        span1.add(1..5);
        span1.add(10..15);
        let mut span2 = Span::new();
        span2.add(3..8);
        span2.add(12..20);
        assert_eq!(span1.overlap(&span2), 5);
    }

    #[test]
    fn test_overlap_ratio_identical() {
        let span1 = Span::from_range(5..10);
        let span2 = Span::from_range(5..10);
        let ratio = span1.overlap_ratio(&span2);
        assert!((ratio - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_overlap_ratio_no_overlap() {
        let span1 = Span::from_range(1..5);
        let span2 = Span::from_range(10..15);
        let ratio = span1.overlap_ratio(&span2);
        assert!((ratio - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_overlap_ratio_partial() {
        let span1 = Span::from_range(1..5);
        let span2 = Span::from_range(3..8);
        let ratio = span1.overlap_ratio(&span2);
        assert!((ratio - 2.0 / 5.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_overlap_ratio_different_lengths() {
        let span1 = Span::from_range(1..10);
        let span2 = Span::from_range(5..15);
        let ratio = span1.overlap_ratio(&span2);
        assert!((ratio - 5.0 / 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_overlap_ratio_empty() {
        let span1 = Span::new();
        let span2 = Span::from_range(5..10);
        let ratio = span1.overlap_ratio(&span2);
        assert!((ratio - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_union_span_non_overlapping() {
        let span1 = Span::from_range(1..5);
        let span2 = Span::from_range(10..15);
        let union = span1.union_span(&span2);
        assert_eq!(union.len(), 2);
        assert_eq!(union.total_length(), 9);
    }

    #[test]
    fn test_union_span_overlapping() {
        let span1 = Span::from_range(1..10);
        let span2 = Span::from_range(5..15);
        let union = span1.union_span(&span2);
        assert_eq!(union.len(), 1);
        assert_eq!(union.total_length(), 14);
    }

    #[test]
    fn test_union_span_identical() {
        let span1 = Span::from_range(5..10);
        let span2 = Span::from_range(5..10);
        let union = span1.union_span(&span2);
        assert_eq!(union.len(), 1);
        assert_eq!(union.total_length(), 5);
    }

    #[test]
    fn test_union_span_with_empty() {
        let span1 = Span::from_range(5..10);
        let span2 = Span::new();
        let union = span1.union_span(&span2);
        assert_eq!(union.len(), 1);
        assert_eq!(union.total_length(), 5);
    }

    #[test]
    fn test_union_span_adjacent() {
        let span1 = Span::from_range(1..5);
        let span2 = Span::from_range(5..10);
        let union = span1.union_span(&span2);
        assert_eq!(union.len(), 2);
        assert_eq!(union.total_length(), 9);
    }

    #[test]
    fn test_intersects_overlapping() {
        let span1 = Span::from_range(1..10);
        let span2 = Span::from_range(5..15);
        assert!(span1.intersects(&span2));
        assert!(span2.intersects(&span1));
    }

    #[test]
    fn test_intersects_no_overlap() {
        let span1 = Span::from_range(1..5);
        let span2 = Span::from_range(10..15);
        assert!(!span1.intersects(&span2));
        assert!(!span2.intersects(&span1));
    }

    #[test]
    fn test_intersects_adjacent() {
        let span1 = Span::from_range(1..5);
        let span2 = Span::from_range(5..10);
        assert!(!span1.intersects(&span2));
        assert!(!span2.intersects(&span1));
    }

    #[test]
    fn test_intersects_contained() {
        let span1 = Span::from_range(1..20);
        let span2 = Span::from_range(5..10);
        assert!(span1.intersects(&span2));
        assert!(span2.intersects(&span1));
    }

    #[test]
    fn test_intersects_identical() {
        let span1 = Span::from_range(5..10);
        let span2 = Span::from_range(5..10);
        assert!(span1.intersects(&span2));
    }

    #[test]
    fn test_intersects_empty() {
        let span1 = Span::new();
        let span2 = Span::from_range(5..10);
        assert!(!span1.intersects(&span2));
        assert!(!span2.intersects(&span1));
    }

    #[test]
    fn test_intersects_multi_range() {
        let mut span1 = Span::new();
        span1.add(1..5);
        span1.add(20..25);
        let mut span2 = Span::new();
        span2.add(10..15);
        span2.add(22..30);
        assert!(span1.intersects(&span2));
    }
}
