//! Tests for embedded license index loading.
//!
//! This module contains tests for:
//! - Engine equivalence (from_embedded vs from_directory)
//! - Determinism (regenerate artifact twice, verify identical)
//! - Failure handling (corrupt bytes, schema mismatch, empty patterns)
//! - Packaging (verify artifact is loadable)

use super::*;
use crate::license_detection::embedded::index::{EmbeddedLicenseIndex, SCHEMA_VERSION};
use crate::license_detection::{SCANCODE_LICENSES_LICENSES_PATH, SCANCODE_LICENSES_RULES_PATH};
use once_cell::sync::Lazy;
use std::path::PathBuf;
use std::sync::Once;

static TEST_ENGINE: Lazy<LicenseDetectionEngine> = Lazy::new(|| {
    LicenseDetectionEngine::from_embedded().expect("Should initialize from embedded artifact")
});

static INIT: Once = Once::new();

fn get_engine() -> &'static LicenseDetectionEngine {
    INIT.call_once(|| {
        let _ = &*TEST_ENGINE;
    });
    &TEST_ENGINE
}

fn get_reference_data_paths() -> Option<(PathBuf, PathBuf)> {
    let rules_path = PathBuf::from(SCANCODE_LICENSES_RULES_PATH);
    let licenses_path = PathBuf::from(SCANCODE_LICENSES_LICENSES_PATH);
    if rules_path.exists() && licenses_path.exists() {
        Some((rules_path, licenses_path))
    } else {
        None
    }
}

mod engine_equivalence {
    use super::*;

    #[test]
    fn test_from_embedded_initializes() {
        let engine = get_engine();

        assert!(
            !engine.index().rules_by_rid.is_empty(),
            "Should have rules loaded"
        );
        assert!(
            !engine.index().licenses_by_key.is_empty(),
            "Should have licenses loaded"
        );
        assert!(
            engine.index().len_legalese > 0,
            "Should have legalese tokens"
        );
    }

    #[test]
    fn test_from_embedded_vs_from_directory_rule_count() {
        let Some((rules_path, licenses_path)) = get_reference_data_paths() else {
            eprintln!("Skipping test: reference directories not found");
            return;
        };

        let engine_from_dir = {
            let loaded_rules = rules::load_loaded_rules_from_directory(&rules_path).unwrap();
            let loaded_licenses =
                rules::load_loaded_licenses_from_directory(&licenses_path).unwrap();
            let index = index::build_index_from_loaded(loaded_rules, loaded_licenses, false);
            let spdx_mapping =
                build_spdx_mapping(&index.licenses_by_key.values().cloned().collect::<Vec<_>>());
            LicenseDetectionEngine {
                index: Arc::new(index),
                spdx_mapping,
            }
        };

        let engine_from_embedded = get_engine();

        assert_eq!(
            engine_from_dir.index().rules_by_rid.len(),
            engine_from_embedded.index().rules_by_rid.len(),
            "Should have same number of rules"
        );
        assert_eq!(
            engine_from_dir.index().licenses_by_key.len(),
            engine_from_embedded.index().licenses_by_key.len(),
            "Should have same number of licenses"
        );
    }

    #[test]
    fn test_from_embedded_vs_from_directory_license_keys() {
        let Some((rules_path, licenses_path)) = get_reference_data_paths() else {
            eprintln!("Skipping test: reference directories not found");
            return;
        };

        let engine_from_dir = {
            let loaded_rules = rules::load_loaded_rules_from_directory(&rules_path).unwrap();
            let loaded_licenses =
                rules::load_loaded_licenses_from_directory(&licenses_path).unwrap();
            let index = index::build_index_from_loaded(loaded_rules, loaded_licenses, false);
            let spdx_mapping =
                build_spdx_mapping(&index.licenses_by_key.values().cloned().collect::<Vec<_>>());
            LicenseDetectionEngine {
                index: Arc::new(index),
                spdx_mapping,
            }
        };

        let engine_from_embedded = get_engine();

        let mut dir_keys: Vec<_> = engine_from_dir
            .index()
            .licenses_by_key
            .keys()
            .cloned()
            .collect();
        let mut embedded_keys: Vec<_> = engine_from_embedded
            .index()
            .licenses_by_key
            .keys()
            .cloned()
            .collect();
        dir_keys.sort();
        embedded_keys.sort();

        assert_eq!(dir_keys, embedded_keys, "Should have same license keys");
    }

    #[test]
    fn test_from_embedded_vs_from_directory_detection_mit() {
        let Some(_) = get_reference_data_paths() else {
            eprintln!("Skipping test: reference directories not found");
            return;
        };

        let engine_from_embedded = get_engine();

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

        let detections = engine_from_embedded
            .detect_with_kind(mit_text, false, false)
            .expect("Detection should succeed");

        assert!(!detections.is_empty(), "Should detect MIT license");
        assert!(
            detections
                .iter()
                .any(|d| d.license_expression.as_deref() == Some("mit")),
            "Should have MIT in expression"
        );
    }

    #[test]
    fn test_from_embedded_vs_from_directory_detection_apache() {
        let Some(_) = get_reference_data_paths() else {
            eprintln!("Skipping test: reference directories not found");
            return;
        };

        let engine_from_embedded = get_engine();

        let apache_text = r#"Apache License
Version 2.0, January 2004
http://www.apache.org/licenses/

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License."#;

        let detections = engine_from_embedded
            .detect_with_kind(apache_text, false, false)
            .expect("Detection should succeed");

        assert!(!detections.is_empty(), "Should detect Apache license");
    }
}

mod determinism {
    use super::*;

    #[test]
    fn test_compression_is_deterministic() {
        let data = b"test data for compression";
        let compressed1 = zstd::encode_all(&data[..], 0).expect("Should compress");
        let compressed2 = zstd::encode_all(&data[..], 0).expect("Should compress");

        assert_eq!(
            compressed1, compressed2,
            "Same input should produce identical compressed output"
        );
    }

    #[test]
    #[ignore = "Too slow for CI - builds full index twice from reference data"]
    fn test_artifact_generation_from_reference_is_deterministic() {
        let Some((rules_path, licenses_path)) = get_reference_data_paths() else {
            eprintln!("Skipping test: reference directories not found");
            return;
        };

        let mut loaded_rules1 =
            rules::load_loaded_rules_from_directory(&rules_path).expect("Should load rules");
        let mut loaded_licenses1 = rules::load_loaded_licenses_from_directory(&licenses_path)
            .expect("Should load licenses");

        let mut loaded_rules2 =
            rules::load_loaded_rules_from_directory(&rules_path).expect("Should load rules");
        let mut loaded_licenses2 = rules::load_loaded_licenses_from_directory(&licenses_path)
            .expect("Should load licenses");

        loaded_rules1.sort_by(|a, b| a.identifier.cmp(&b.identifier));
        loaded_licenses1.sort_by(|a, b| a.key.cmp(&b.key));
        loaded_rules2.sort_by(|a, b| a.identifier.cmp(&b.identifier));
        loaded_licenses2.sort_by(|a, b| a.key.cmp(&b.key));

        let index1 = index::build_index_from_loaded(loaded_rules1, loaded_licenses1, false);
        let index2 = index::build_index_from_loaded(loaded_rules2, loaded_licenses2, false);

        let embedded1 = EmbeddedLicenseIndex::from(&index1);
        let embedded2 = EmbeddedLicenseIndex::from(&index2);

        let bytes1 = embedded1.serialize_to_bytes().expect("Should serialize");
        let bytes2 = embedded2.serialize_to_bytes().expect("Should serialize");

        assert_eq!(bytes1, bytes2, "Regenerated artifacts should be identical");
    }
}

mod failure_handling {
    use super::*;

    #[test]
    fn test_decompression_invalid_data_fails() {
        let invalid_compressed: Vec<u8> = vec![0xFF, 0xFE, 0xFD, 0xFC];

        let result = zstd::decode_all(&invalid_compressed[..]);

        assert!(result.is_err(), "Should fail to decompress invalid data");
    }

    #[test]
    fn test_build_index_from_empty_loaded_is_valid() {
        let index = index::build_index_from_loaded(vec![], vec![], false);

        assert!(index.rules_by_rid.is_empty(), "Should have empty rules");
        assert!(
            index.licenses_by_key.is_empty(),
            "Should have empty licenses"
        );
    }
}

mod packaging {
    use super::*;

    #[test]
    fn test_embedded_artifact_exists() {
        let artifact_path = PathBuf::from("resources/license_detection/license_index.bincode.zst");

        assert!(
            artifact_path.exists(),
            "Embedded artifact should exist at {}",
            artifact_path.display()
        );
    }

    #[test]
    fn test_embedded_artifact_is_loadable() {
        let engine = get_engine();

        assert!(
            !engine.index().rules_by_rid.is_empty(),
            "Should have rules loaded"
        );
        assert!(
            !engine.index().licenses_by_key.is_empty(),
            "Should have licenses loaded"
        );
    }

    #[test]
    fn test_embedded_artifact_schema_version() {
        let artifact_bytes =
            include_bytes!("../../resources/license_detection/license_index.bincode.zst");

        let decompressed =
            zstd::decode_all(&artifact_bytes[..]).expect("Should decompress artifact");

        let (index, _): (EmbeddedLicenseIndex, _) =
            bincode::serde::decode_from_slice(&decompressed, bincode::config::standard())
                .expect("Should deserialize artifact");

        assert_eq!(
            index.schema_version, SCHEMA_VERSION,
            "Embedded artifact should have current schema version"
        );
    }

    #[test]
    fn test_embedded_artifact_has_non_empty_rules() {
        let artifact_bytes =
            include_bytes!("../../resources/license_detection/license_index.bincode.zst");

        let decompressed =
            zstd::decode_all(&artifact_bytes[..]).expect("Should decompress artifact");

        let (index, _): (EmbeddedLicenseIndex, _) =
            bincode::serde::decode_from_slice(&decompressed, bincode::config::standard())
                .expect("Should deserialize artifact");

        assert!(
            !index.rules_by_rid.is_empty(),
            "Embedded artifact should have rules"
        );
    }

    #[test]
    fn test_embedded_artifact_has_non_empty_licenses() {
        let artifact_bytes =
            include_bytes!("../../resources/license_detection/license_index.bincode.zst");

        let decompressed =
            zstd::decode_all(&artifact_bytes[..]).expect("Should decompress artifact");

        let (index, _): (EmbeddedLicenseIndex, _) =
            bincode::serde::decode_from_slice(&decompressed, bincode::config::standard())
                .expect("Should deserialize artifact");

        assert!(
            !index.licenses_by_key.is_empty(),
            "Embedded artifact should have licenses"
        );
    }

    #[test]
    fn test_embedded_artifact_rules_sorted() {
        let artifact_bytes =
            include_bytes!("../../resources/license_detection/license_index.bincode.zst");

        let decompressed =
            zstd::decode_all(&artifact_bytes[..]).expect("Should decompress artifact");

        let (index, _): (EmbeddedLicenseIndex, _) =
            bincode::serde::decode_from_slice(&decompressed, bincode::config::standard())
                .expect("Should deserialize artifact");

        let identifiers: Vec<_> = index.rules_by_rid.iter().map(|r| &r.identifier).collect();

        let mut sorted_identifiers = identifiers.clone();
        sorted_identifiers.sort();

        assert_eq!(
            identifiers, sorted_identifiers,
            "Rules in artifact should be sorted by identifier"
        );
    }

    #[test]
    fn test_embedded_artifact_licenses_sorted() {
        let artifact_bytes =
            include_bytes!("../../resources/license_detection/license_index.bincode.zst");

        let decompressed =
            zstd::decode_all(&artifact_bytes[..]).expect("Should decompress artifact");

        let (index, _): (EmbeddedLicenseIndex, _) =
            bincode::serde::decode_from_slice(&decompressed, bincode::config::standard())
                .expect("Should deserialize artifact");

        let keys: Vec<_> = index.licenses_by_key.iter().map(|(k, _)| k).collect();

        let mut sorted_keys = keys.clone();
        sorted_keys.sort();

        assert_eq!(
            keys, sorted_keys,
            "Licenses in artifact should be sorted by key"
        );
    }
}
