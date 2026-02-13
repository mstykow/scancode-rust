//! Unit tests for license detection components.
//!
//! This module contains strategy-specific unit tests for:
//! - Hash matching (`1-hash` matcher)
//! - SPDX-License-Identifier matching (`1-spdx-id` matcher)
//! - Aho-Corasick exact matching (`2-aho` matcher)
//! - Sequence alignment matching (`3-seq` matcher)
//! - Unknown license detection (`5-unknown` matcher)
//! - Detection grouping and expression combination

#[cfg(test)]
mod tests {
    use crate::license_detection::LicenseDetectionEngine;
    use std::path::PathBuf;

    fn get_reference_data_path() -> Option<PathBuf> {
        let data_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data");
        if data_path.exists() {
            Some(data_path)
        } else {
            None
        }
    }

    fn create_engine_from_reference() -> Option<LicenseDetectionEngine> {
        let data_path = get_reference_data_path()?;
        LicenseDetectionEngine::new(&data_path).ok()
    }

    #[test]
    fn test_spdx_simple() {
        let Some(engine) = create_engine_from_reference() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let text = "SPDX-License-Identifier: MIT\nSome code here";
        let detections = engine.detect(text).expect("Detection should succeed");

        assert!(
            !detections.is_empty(),
            "Should detect license from SPDX identifier"
        );

        let has_mit = detections.iter().any(|d| {
            d.license_expression
                .as_ref()
                .map(|e| e.contains("mit"))
                .unwrap_or(false)
        });
        assert!(has_mit, "Should detect MIT license");
    }

    #[test]
    fn test_spdx_with_or() {
        let Some(engine) = create_engine_from_reference() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let text = "SPDX-License-Identifier: MIT OR Apache-2.0";
        let detections = engine.detect(text).expect("Detection should succeed");

        assert!(
            !detections.is_empty(),
            "Should detect license from SPDX identifier with OR"
        );
    }

    #[test]
    fn test_spdx_with_plus() {
        let Some(engine) = create_engine_from_reference() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let text = "SPDX-License-Identifier: GPL-2.0+";
        let detections = engine.detect(text).expect("Detection should succeed");

        assert!(
            !detections.is_empty(),
            "Should detect license from SPDX identifier with plus"
        );
    }

    #[test]
    fn test_spdx_in_comment() {
        let Some(engine) = create_engine_from_reference() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let text = "// SPDX-License-Identifier: MIT\n/* some code */";
        let detections = engine.detect(text).expect("Detection should succeed");

        assert!(
            !detections.is_empty(),
            "Should detect SPDX identifier in comment"
        );
    }

    #[test]
    fn test_hash_exact_mit() {
        let Some(engine) = create_engine_from_reference() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let mit_text = r#"Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE."#;

        let detections = engine.detect(mit_text).expect("Detection should succeed");

        assert!(!detections.is_empty(), "Should detect MIT license");

        let has_mit = detections.iter().any(|d| {
            d.license_expression
                .as_ref()
                .map(|e| e.contains("mit") || e.contains("unknown"))
                .unwrap_or(false)
        });
        assert!(has_mit, "Should detect MIT or unknown license");
    }

    #[test]
    fn test_aho_single_rule() {
        let Some(engine) = create_engine_from_reference() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let text = "Licensed under the MIT License";
        let detections = engine.detect(text).expect("Detection should succeed");

        assert!(!detections.is_empty(), "Should detect license notice");
    }

    #[test]
    fn test_aho_apache_notice() {
        let Some(engine) = create_engine_from_reference() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let text = "Licensed under the Apache License, Version 2.0";
        let detections = engine.detect(text).expect("Detection should succeed");

        assert!(!detections.is_empty(), "Should detect Apache notice");
    }

    #[test]
    fn test_seq_partial_license() {
        let Some(engine) = create_engine_from_reference() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let partial_mit = r#"Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software."#;

        let detections = engine
            .detect(partial_mit)
            .expect("Detection should succeed");

        assert!(!detections.is_empty(), "Should detect partial MIT license");
    }

    #[test]
    fn test_unknown_proprietary() {
        let Some(engine) = create_engine_from_reference() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let text = "This software is proprietary and confidential. All rights reserved.";
        let detections = engine.detect(text).expect("Detection should succeed");

        assert!(
            !detections.is_empty(),
            "Should detect unknown license or return empty"
        );
    }

    #[test]
    fn test_empty_text() {
        let Some(engine) = create_engine_from_reference() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let detections = engine.detect("").expect("Detection should succeed");
        assert!(
            detections.is_empty()
                || detections
                    .iter()
                    .any(|d| d.license_expression.as_deref() == Some("proprietary-license")),
            "Empty text should have no detections or only proprietary-license"
        );

        let detections = engine
            .detect("   \n\n   ")
            .expect("Detection should succeed");
        assert!(
            detections.is_empty() || !detections.is_empty(),
            "Whitespace-only should complete"
        );
    }

    #[test]
    fn test_no_license_text() {
        let Some(engine) = create_engine_from_reference() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let text = "This is just some random text without any license information.";
        let detections = engine.detect(text).expect("Detection should succeed");

        assert!(
            detections.is_empty() || !detections.is_empty(),
            "Detection should complete without error"
        );
    }

    #[test]
    fn test_gpl_notice() {
        let Some(engine) = create_engine_from_reference() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let text = "This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation.";
        let detections = engine.detect(text).expect("Detection should succeed");

        assert!(!detections.is_empty(), "Should detect GPL notice");
    }
}
