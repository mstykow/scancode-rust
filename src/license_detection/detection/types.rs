//! Core detection data structures.

use crate::license_detection::models::LicenseMatch;

pub struct DetectionGroup {
    /// The matches in this group
    pub matches: Vec<LicenseMatch>,
    /// Start line of the group (1-indexed)
    pub start_line: usize,
    /// End line of the group (1-indexed)
    pub end_line: usize,
}

impl DetectionGroup {
    pub fn new(matches: Vec<LicenseMatch>) -> Self {
        if matches.is_empty() {
            return Self {
                matches,
                start_line: 0,
                end_line: 0,
            };
        }

        let start_line = matches.iter().map(|m| m.start_line).min().unwrap_or(0);
        let end_line = matches.iter().map(|m| m.end_line).max().unwrap_or(0);

        Self {
            matches,
            start_line,
            end_line,
        }
    }
}

/// A LicenseDetection combines one or more LicenseMatch objects using
/// various rules and heuristics.
#[derive(Debug, Clone)]
pub struct LicenseDetection {
    /// A license expression string using SPDX license expression syntax
    /// and ScanCode license keys - the effective license expression for this detection.
    pub license_expression: Option<String>,

    /// SPDX license expression string with SPDX ids only.
    pub license_expression_spdx: Option<String>,

    /// List of license matches combined in this detection.
    pub matches: Vec<LicenseMatch>,

    /// A list of detection log entries explaining how this detection was created.
    pub detection_log: Vec<String>,

    /// An identifier unique for a license detection, containing the license
    /// expression and a UUID crafted from the match contents.
    pub identifier: Option<String>,

    /// File path and start/end lines to locate the detection.
    pub file_region: Option<FileRegion>,
}

/// A file has one or more file-regions, which are separate regions of the file
/// containing some license information.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct FileRegion {
    /// File path
    pub path: String,
    /// Start line number (1-indexed)
    pub start_line: usize,
    /// End line number (1-indexed)
    pub end_line: usize,
}


#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_match(start_line: usize, end_line: usize) -> LicenseMatch {
        LicenseMatch {
            rid: 0,
            license_expression: "mit".to_string(),
            license_expression_spdx: "MIT".to_string(),
            from_file: Some("test.txt".to_string()),
            start_line,
            end_line,
            start_token: 0,
            end_token: 0,
            matcher: "1-hash".to_string(),
            score: 95.0,
            matched_length: 100,
            match_coverage: 95.0,
            rule_relevance: 100,
            rule_identifier: "mit.LICENSE".to_string(),
            rule_url: "https://example.com".to_string(),
            matched_text: Some("MIT License".to_string()),
            referenced_filenames: None,
            is_license_intro: false,
            is_license_clue: false,
            is_license_reference: false,
            is_license_tag: false,
            is_license_text: false,
            is_from_license: false,
            rule_length: 100,
            matched_token_positions: None,
            hilen: 50,
            rule_start_token: 0,
            qspan_positions: None,
            ispan_positions: None,
            hispan_positions: None,
            candidate_resemblance: 0.0,
            candidate_containment: 0.0,
        }
    }

    #[test]
    fn test_detection_group_new_empty() {
        let group = DetectionGroup::new(Vec::new());
        assert_eq!(group.matches.len(), 0);
        assert_eq!(group.start_line, 0);
        assert_eq!(group.end_line, 0);
    }

    #[test]
    fn test_detection_group_new_with_matches() {
        let match1 = create_test_match(1, 5);
        let match2 = create_test_match(10, 15);
        let group = DetectionGroup::new(vec![match1, match2]);

        assert_eq!(group.matches.len(), 2);
        assert_eq!(group.start_line, 1);
        assert_eq!(group.end_line, 15);
    }
}
