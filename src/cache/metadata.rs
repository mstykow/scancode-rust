use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheSnapshotMetadata {
    pub cache_schema_version: u32,
    pub engine_version: String,
    pub rules_fingerprint: String,
    pub build_options_fingerprint: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheInvalidationKey<'a> {
    pub cache_schema_version: u32,
    pub engine_version: &'a str,
    pub rules_fingerprint: &'a str,
    pub build_options_fingerprint: &'a str,
}

impl CacheSnapshotMetadata {
    pub fn is_compatible_with(&self, key: &CacheInvalidationKey<'_>) -> bool {
        self.cache_schema_version == key.cache_schema_version
            && self.engine_version == key.engine_version
            && self.rules_fingerprint == key.rules_fingerprint
            && self.build_options_fingerprint == key.build_options_fingerprint
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_metadata() -> CacheSnapshotMetadata {
        CacheSnapshotMetadata {
            cache_schema_version: 1,
            engine_version: "engine-v1".to_string(),
            rules_fingerprint: "rules-abc".to_string(),
            build_options_fingerprint: "opts-123".to_string(),
            created_at: "2026-03-02T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn test_metadata_compatibility_all_fields_match() {
        let metadata = fixture_metadata();
        let key = CacheInvalidationKey {
            cache_schema_version: 1,
            engine_version: "engine-v1",
            rules_fingerprint: "rules-abc",
            build_options_fingerprint: "opts-123",
        };

        assert!(metadata.is_compatible_with(&key));
    }

    #[test]
    fn test_metadata_compatibility_detects_engine_version_mismatch() {
        let metadata = fixture_metadata();
        let key = CacheInvalidationKey {
            cache_schema_version: 1,
            engine_version: "engine-v2",
            rules_fingerprint: "rules-abc",
            build_options_fingerprint: "opts-123",
        };

        assert!(!metadata.is_compatible_with(&key));
    }

    #[test]
    fn test_metadata_compatibility_detects_rules_fingerprint_mismatch() {
        let metadata = fixture_metadata();
        let key = CacheInvalidationKey {
            cache_schema_version: 1,
            engine_version: "engine-v1",
            rules_fingerprint: "rules-def",
            build_options_fingerprint: "opts-123",
        };

        assert!(!metadata.is_compatible_with(&key));
    }
}
