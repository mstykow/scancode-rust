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
}
