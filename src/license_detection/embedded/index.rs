//! Embedded license index serialization and deserialization.
//!
//! This module provides the `EmbeddedLicenseIndex` struct which is a serializable
//! representation of the `LicenseIndex`. It uses sorted vectors for HashMaps to
//! ensure deterministic serialization, and byte vectors for automatons.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::license_detection::automaton::Automaton;
use crate::license_detection::index::LicenseIndex;
use crate::license_detection::index::dictionary::{TokenDictionary, TokenId, TokenKind};
use crate::license_detection::models::{License, Rule};

pub const SCHEMA_VERSION: u32 = 1;

pub type HighPostingsEntry = (u16, Vec<usize>);
pub type HighPostingsByRidEntry = (usize, Vec<HighPostingsEntry>);

// This struct and its methods will be used by the xtask build process
// and runtime index loading in upcoming phases.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedLicenseIndex {
    pub schema_version: u32,
    pub data: Vec<u8>,
}

// These methods will be used by the xtask build process and runtime index loading.
#[allow(dead_code)]
impl SerializedLicenseIndex {
    pub fn to_bytes(&self) -> Result<Vec<u8>, SerializationError> {
        let serialized =
            bincode::serde::encode_to_vec(self, bincode::config::standard()).map_err(|e| {
                SerializationError(format!("Failed to serialize SerializedLicenseIndex: {}", e))
            })?;
        Ok(serialized)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SerializationError> {
        let (index, _): (Self, _) =
            bincode::serde::decode_from_slice(bytes, bincode::config::standard()).map_err(|e| {
                SerializationError(format!(
                    "Failed to deserialize SerializedLicenseIndex: {}",
                    e
                ))
            })?;
        Ok(index)
    }

    pub fn decompress(&self) -> Result<EmbeddedLicenseIndex, SerializationError> {
        let decompressed = zstd::decode_all(&self.data[..]).map_err(|e| {
            SerializationError(format!("Failed to decompress license index: {}", e))
        })?;
        let (index, _): (EmbeddedLicenseIndex, _) =
            bincode::serde::decode_from_slice(&decompressed, bincode::config::standard()).map_err(
                |e| {
                    SerializationError(format!("Failed to deserialize EmbeddedLicenseIndex: {}", e))
                },
            )?;
        Ok(index)
    }

    pub fn decompress_from_bytes(bytes: &[u8]) -> Result<EmbeddedLicenseIndex, SerializationError> {
        let serialized: SerializedLicenseIndex =
            bincode::serde::decode_from_slice(bytes, bincode::config::standard())
                .map_err(|e| {
                    SerializationError(format!(
                        "Failed to deserialize SerializedLicenseIndex: {}",
                        e
                    ))
                })?
                .0;

        if serialized.schema_version != SCHEMA_VERSION {
            return Err(SerializationError(format!(
                "Schema version mismatch: expected {}, got {}",
                SCHEMA_VERSION, serialized.schema_version
            )));
        }

        let decompressed = zstd::decode_all(&serialized.data[..]).map_err(|e| {
            SerializationError(format!("Failed to decompress license index: {}", e))
        })?;
        let (index, _): (EmbeddedLicenseIndex, _) =
            bincode::serde::decode_from_slice(&decompressed, bincode::config::standard()).map_err(
                |e| {
                    SerializationError(format!("Failed to deserialize EmbeddedLicenseIndex: {}", e))
                },
            )?;
        Ok(index)
    }
}

#[derive(Debug, Clone)]
pub struct SerializationError(pub String);

impl std::fmt::Display for SerializationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "License index serialization error: {}", self.0)
    }
}

impl std::error::Error for SerializationError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedTokenMetadata {
    pub kind: TokenKind,
    pub is_digit_only: bool,
    pub is_short_or_digit: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedTokenDictionary {
    pub tokens_to_ids: Vec<(String, u16)>,
    pub token_metadata: Vec<Option<EmbeddedTokenMetadata>>,
    pub len_legalese: usize,
    pub next_id: u16,
}

impl From<&TokenDictionary> for EmbeddedTokenDictionary {
    fn from(dict: &TokenDictionary) -> Self {
        let mut tokens_to_ids: Vec<(String, u16)> = dict
            .tokens_to_ids_iter()
            .map(|(s, tid)| (s.clone(), tid.raw()))
            .collect();
        tokens_to_ids.sort_by(|a, b| a.0.cmp(&b.0));

        let token_metadata = dict
            .metadata_slice()
            .iter()
            .map(|opt_meta| {
                opt_meta.map(|meta| EmbeddedTokenMetadata {
                    kind: meta.kind,
                    is_digit_only: meta.is_digit_only,
                    is_short_or_digit: meta.is_short_or_digit,
                })
            })
            .collect();

        Self {
            tokens_to_ids,
            token_metadata,
            len_legalese: dict.legalese_count(),
            next_id: dict.next_id_raw(),
        }
    }
}

impl From<EmbeddedTokenDictionary> for TokenDictionary {
    fn from(embedded: EmbeddedTokenDictionary) -> Self {
        let mut tokens_to_ids = HashMap::new();
        for (token, id) in embedded.tokens_to_ids {
            tokens_to_ids.insert(token, TokenId::new(id));
        }

        let token_metadata = embedded
            .token_metadata
            .into_iter()
            .map(|opt_meta| {
                opt_meta.map(
                    |meta| crate::license_detection::index::dictionary::TokenMetadata {
                        kind: meta.kind,
                        is_digit_only: meta.is_digit_only,
                        is_short_or_digit: meta.is_short_or_digit,
                    },
                )
            })
            .collect();

        Self::from_parts(
            tokens_to_ids,
            token_metadata,
            embedded.len_legalese,
            TokenId::new(embedded.next_id),
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedLicenseIndex {
    pub schema_version: u32,
    pub dictionary: EmbeddedTokenDictionary,
    pub len_legalese: usize,
    pub rid_by_hash: Vec<([u8; 20], usize)>,
    pub rules_by_rid: Vec<Rule>,
    pub tids_by_rid: Vec<Vec<u16>>,
    pub rules_automaton: Vec<u8>,
    pub unknown_automaton: Vec<u8>,
    pub sets_by_rid: Vec<(usize, Vec<u16>)>,
    pub msets_by_rid: Vec<(usize, Vec<(u16, u16)>)>,
    pub high_sets_by_rid: Vec<(usize, Vec<u16>)>,
    pub high_postings_by_rid: Vec<HighPostingsByRidEntry>,
    pub false_positive_rids: Vec<usize>,
    pub approx_matchable_rids: Vec<usize>,
    pub licenses_by_key: Vec<(String, License)>,
    pub pattern_id_to_rid: Vec<Vec<usize>>,
    pub rid_by_spdx_key: Vec<(String, usize)>,
    pub unknown_spdx_rid: Option<usize>,
    pub rids_by_high_tid: Vec<(u16, Vec<usize>)>,
}

impl From<&LicenseIndex> for EmbeddedLicenseIndex {
    fn from(index: &LicenseIndex) -> Self {
        let dictionary = EmbeddedTokenDictionary::from(&index.dictionary);

        let mut rid_by_hash: Vec<([u8; 20], usize)> = index
            .rid_by_hash
            .iter()
            .map(|(hash, rid)| (*hash, *rid))
            .collect();
        rid_by_hash.sort_by_key(|(_, rid)| *rid);

        let tids_by_rid = index
            .tids_by_rid
            .iter()
            .map(|tids| tids.iter().map(|tid| tid.raw()).collect())
            .collect();

        let rules_automaton = index.rules_automaton.serialize();
        let unknown_automaton = index.unknown_automaton.serialize();

        let mut sets_by_rid: Vec<(usize, Vec<u16>)> = index
            .sets_by_rid
            .iter()
            .map(|(rid, set)| {
                let mut tokens: Vec<u16> = set.iter().map(|tid| tid.raw()).collect();
                tokens.sort();
                (*rid, tokens)
            })
            .collect();
        sets_by_rid.sort_by_key(|(rid, _)| *rid);

        let mut msets_by_rid: Vec<(usize, Vec<(u16, u16)>)> = index
            .msets_by_rid
            .iter()
            .map(|(rid, mset)| {
                let mut entries: Vec<(u16, u16)> = mset
                    .iter()
                    .map(|(tid, count)| (tid.raw(), *count as u16))
                    .collect();
                entries.sort_by_key(|(tid, _)| *tid);
                (*rid, entries)
            })
            .collect();
        msets_by_rid.sort_by_key(|(rid, _)| *rid);

        let mut high_sets_by_rid: Vec<(usize, Vec<u16>)> = index
            .high_sets_by_rid
            .iter()
            .map(|(rid, set)| {
                let mut tokens: Vec<u16> = set.iter().map(|tid| tid.raw()).collect();
                tokens.sort();
                (*rid, tokens)
            })
            .collect();
        high_sets_by_rid.sort_by_key(|(rid, _)| *rid);

        let mut high_postings_by_rid: Vec<HighPostingsByRidEntry> = index
            .high_postings_by_rid
            .iter()
            .map(|(rid, postings)| {
                let mut entries: Vec<(u16, Vec<usize>)> = postings
                    .iter()
                    .map(|(tid, positions)| (tid.raw(), positions.clone()))
                    .collect();
                entries.sort_by_key(|(tid, _)| *tid);
                (*rid, entries)
            })
            .collect();
        high_postings_by_rid.sort_by_key(|(rid, _)| *rid);

        let mut false_positive_rids: Vec<usize> =
            index.false_positive_rids.iter().copied().collect();
        false_positive_rids.sort();

        let mut approx_matchable_rids: Vec<usize> =
            index.approx_matchable_rids.iter().copied().collect();
        approx_matchable_rids.sort();

        let mut licenses_by_key: Vec<(String, License)> = index
            .licenses_by_key
            .iter()
            .map(|(key, license)| (key.clone(), license.clone()))
            .collect();
        licenses_by_key.sort_by(|a, b| a.0.cmp(&b.0));

        let mut rid_by_spdx_key: Vec<(String, usize)> = index
            .rid_by_spdx_key
            .iter()
            .map(|(key, rid)| (key.clone(), *rid))
            .collect();
        rid_by_spdx_key.sort_by(|a, b| a.0.cmp(&b.0));

        let mut rids_by_high_tid: Vec<(u16, Vec<usize>)> = index
            .rids_by_high_tid
            .iter()
            .map(|(tid, rids)| {
                let mut rids_vec: Vec<usize> = rids.iter().copied().collect();
                rids_vec.sort();
                (tid.raw(), rids_vec)
            })
            .collect();
        rids_by_high_tid.sort_by_key(|(tid, _)| *tid);

        Self {
            schema_version: SCHEMA_VERSION,
            dictionary,
            len_legalese: index.len_legalese,
            rid_by_hash,
            rules_by_rid: index.rules_by_rid.clone(),
            tids_by_rid,
            rules_automaton,
            unknown_automaton,
            sets_by_rid,
            msets_by_rid,
            high_sets_by_rid,
            high_postings_by_rid,
            false_positive_rids,
            approx_matchable_rids,
            licenses_by_key,
            pattern_id_to_rid: index.pattern_id_to_rid.clone(),
            rid_by_spdx_key,
            unknown_spdx_rid: index.unknown_spdx_rid,
            rids_by_high_tid,
        }
    }
}

impl TryFrom<EmbeddedLicenseIndex> for LicenseIndex {
    type Error = SerializationError;

    fn try_from(embedded: EmbeddedLicenseIndex) -> Result<Self, Self::Error> {
        use std::time::Instant;
        let t0 = Instant::now();

        if embedded.schema_version != SCHEMA_VERSION {
            return Err(SerializationError(format!(
                "Schema version mismatch: expected {}, got {}",
                SCHEMA_VERSION, embedded.schema_version
            )));
        }

        let dictionary = TokenDictionary::from(embedded.dictionary);
        let t1 = Instant::now();
        eprintln!(
            "    [try_from] TokenDictionary::from took {:?}",
            t1.duration_since(t0)
        );

        let rid_by_hash: HashMap<[u8; 20], usize> = embedded.rid_by_hash.into_iter().collect();
        let t2 = Instant::now();
        eprintln!(
            "    [try_from] rid_by_hash.collect took {:?}",
            t2.duration_since(t1)
        );

        let tids_by_rid: Vec<Vec<TokenId>> = embedded
            .tids_by_rid
            .into_iter()
            .map(|tids| tids.into_iter().map(TokenId::new).collect())
            .collect();
        let t3 = Instant::now();
        eprintln!(
            "    [try_from] tids_by_rid took {:?}",
            t3.duration_since(t2)
        );

        let rules_automaton = Automaton::deserialize_unchecked(&embedded.rules_automaton);
        let t4 = Instant::now();
        eprintln!(
            "    [try_from] rules_automaton.deserialize took {:?}",
            t4.duration_since(t3)
        );

        let unknown_automaton = Automaton::deserialize_unchecked(&embedded.unknown_automaton);
        let t5 = Instant::now();
        eprintln!(
            "    [try_from] unknown_automaton.deserialize took {:?}",
            t5.duration_since(t4)
        );

        let sets_by_rid: HashMap<usize, HashSet<TokenId>> = embedded
            .sets_by_rid
            .into_iter()
            .map(|(rid, tokens)| (rid, tokens.into_iter().map(TokenId::new).collect()))
            .collect();
        let t6 = Instant::now();
        eprintln!(
            "    [try_from] sets_by_rid took {:?}",
            t6.duration_since(t5)
        );

        let msets_by_rid: HashMap<usize, HashMap<TokenId, usize>> = embedded
            .msets_by_rid
            .into_iter()
            .map(|(rid, entries)| {
                (
                    rid,
                    entries
                        .into_iter()
                        .map(|(tid, count)| (TokenId::new(tid), count as usize))
                        .collect(),
                )
            })
            .collect();
        let t7 = Instant::now();
        eprintln!(
            "    [try_from] msets_by_rid took {:?}",
            t7.duration_since(t6)
        );

        let high_sets_by_rid: HashMap<usize, HashSet<TokenId>> = embedded
            .high_sets_by_rid
            .into_iter()
            .map(|(rid, tokens)| (rid, tokens.into_iter().map(TokenId::new).collect()))
            .collect();
        let t8 = Instant::now();
        eprintln!(
            "    [try_from] high_sets_by_rid took {:?}",
            t8.duration_since(t7)
        );

        let high_postings_by_rid: HashMap<usize, HashMap<TokenId, Vec<usize>>> = embedded
            .high_postings_by_rid
            .into_iter()
            .map(|(rid, entries)| {
                (
                    rid,
                    entries
                        .into_iter()
                        .map(|(tid, positions)| (TokenId::new(tid), positions))
                        .collect(),
                )
            })
            .collect();
        let t9 = Instant::now();
        eprintln!(
            "    [try_from] high_postings_by_rid took {:?}",
            t9.duration_since(t8)
        );

        let false_positive_rids: HashSet<usize> =
            embedded.false_positive_rids.into_iter().collect();

        let approx_matchable_rids: HashSet<usize> =
            embedded.approx_matchable_rids.into_iter().collect();

        let licenses_by_key: HashMap<String, License> =
            embedded.licenses_by_key.into_iter().collect();
        let t10 = Instant::now();
        eprintln!(
            "    [try_from] licenses_by_key took {:?}",
            t10.duration_since(t9)
        );

        let rid_by_spdx_key: HashMap<String, usize> =
            embedded.rid_by_spdx_key.into_iter().collect();

        let rids_by_high_tid: HashMap<TokenId, HashSet<usize>> = embedded
            .rids_by_high_tid
            .into_iter()
            .map(|(tid, rids)| (TokenId::new(tid), rids.into_iter().collect()))
            .collect();
        let t11 = Instant::now();
        eprintln!(
            "    [try_from] rids_by_high_tid took {:?}",
            t11.duration_since(t10)
        );
        eprintln!("    [try_from] TOTAL took {:?}", t11.duration_since(t0));

        Ok(LicenseIndex {
            dictionary,
            len_legalese: embedded.len_legalese,
            rid_by_hash,
            rules_by_rid: embedded.rules_by_rid,
            tids_by_rid,
            rules_automaton,
            unknown_automaton,
            sets_by_rid,
            msets_by_rid,
            high_sets_by_rid,
            high_postings_by_rid,
            false_positive_rids,
            approx_matchable_rids,
            licenses_by_key,
            pattern_id_to_rid: embedded.pattern_id_to_rid,
            rid_by_spdx_key,
            unknown_spdx_rid: embedded.unknown_spdx_rid,
            rids_by_high_tid,
        })
    }
}

// This method will be used by the xtask build process in upcoming phases.
#[allow(dead_code)]
impl EmbeddedLicenseIndex {
    pub fn serialize(&self) -> Result<SerializedLicenseIndex, SerializationError> {
        let bincode_data = bincode::serde::encode_to_vec(self, bincode::config::standard())
            .map_err(|e| {
                SerializationError(format!("Failed to serialize EmbeddedLicenseIndex: {}", e))
            })?;

        let compressed = zstd::encode_all(&bincode_data[..], 0)
            .map_err(|e| SerializationError(format!("Failed to compress license index: {}", e)))?;

        Ok(SerializedLicenseIndex {
            schema_version: SCHEMA_VERSION,
            data: compressed,
        })
    }

    pub fn serialize_to_bytes(&self) -> Result<Vec<u8>, SerializationError> {
        let bincode_data = bincode::serde::encode_to_vec(self, bincode::config::standard())
            .map_err(|e| {
                SerializationError(format!("Failed to serialize EmbeddedLicenseIndex: {}", e))
            })?;

        zstd::encode_all(&bincode_data[..], 0)
            .map_err(|e| SerializationError(format!("Failed to compress license index: {}", e)))
    }

    pub fn deserialize_from_bytes(bytes: &[u8]) -> Result<Self, SerializationError> {
        use std::time::Instant;
        let t0 = Instant::now();

        let decompressed = zstd::decode_all(bytes).map_err(|e| {
            SerializationError(format!("Failed to decompress license index: {}", e))
        })?;
        let t1 = Instant::now();
        eprintln!(
            "  [deserialize_from_bytes] zstd::decode_all took {:?}",
            t1.duration_since(t0)
        );
        eprintln!(
            "  [deserialize_from_bytes] decompressed size: {} MB",
            decompressed.len() / 1_000_000
        );

        let (index, _): (Self, _) =
            bincode::serde::decode_from_slice(&decompressed, bincode::config::standard()).map_err(
                |e| {
                    SerializationError(format!("Failed to deserialize EmbeddedLicenseIndex: {}", e))
                },
            )?;
        let t2 = Instant::now();
        eprintln!(
            "  [deserialize_from_bytes] bincode::decode took {:?}",
            t2.duration_since(t1)
        );
        eprintln!(
            "  [deserialize_from_bytes] TOTAL took {:?}",
            t2.duration_since(t0)
        );

        Ok(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_license_index() -> LicenseIndex {
        let mut index = LicenseIndex::with_legalese_count(3);

        index.rid_by_hash.insert([1u8; 20], 0);
        index.rid_by_hash.insert([2u8; 20], 1);

        index
            .rules_by_rid
            .push(crate::license_detection::models::Rule {
                identifier: "test-rule-1".to_string(),
                license_expression: "MIT".to_string(),
                text: "MIT License".to_string(),
                tokens: vec![TokenId::new(0), TokenId::new(1)],
                rule_kind: crate::license_detection::models::RuleKind::Text,
                is_false_positive: false,
                is_required_phrase: false,
                is_from_license: true,
                relevance: 100,
                minimum_coverage: None,
                has_stored_minimum_coverage: false,
                is_continuous: false,
                required_phrase_spans: vec![],
                stopwords_by_pos: HashMap::new(),
                referenced_filenames: None,
                ignorable_urls: None,
                ignorable_emails: None,
                ignorable_copyrights: None,
                ignorable_holders: None,
                ignorable_authors: None,
                language: None,
                notes: None,
                length_unique: 2,
                high_length_unique: 1,
                high_length: 1,
                min_matched_length: 1,
                min_high_matched_length: 1,
                min_matched_length_unique: 1,
                min_high_matched_length_unique: 1,
                is_small: false,
                is_tiny: false,
                starts_with_license: false,
                ends_with_license: false,
                is_deprecated: false,
                spdx_license_key: Some("MIT".to_string()),
                other_spdx_license_keys: vec![],
            });

        index
            .tids_by_rid
            .push(vec![TokenId::new(0), TokenId::new(1)]);
        index
            .tids_by_rid
            .push(vec![TokenId::new(2), TokenId::new(3)]);

        index.sets_by_rid.insert(
            0,
            vec![TokenId::new(0), TokenId::new(1)].into_iter().collect(),
        );
        index.sets_by_rid.insert(
            1,
            vec![TokenId::new(2), TokenId::new(3)].into_iter().collect(),
        );

        index.msets_by_rid.insert(
            0,
            vec![(TokenId::new(0), 2), (TokenId::new(1), 1)]
                .into_iter()
                .collect(),
        );

        index
            .high_sets_by_rid
            .insert(0, vec![TokenId::new(0)].into_iter().collect());

        index.high_postings_by_rid.insert(
            0,
            vec![(TokenId::new(0), vec![0, 5, 10])]
                .into_iter()
                .collect(),
        );

        index.false_positive_rids.insert(42);
        index.false_positive_rids.insert(43);

        index.approx_matchable_rids.insert(0);
        index.approx_matchable_rids.insert(1);

        index.licenses_by_key.insert(
            "mit".to_string(),
            License {
                key: "mit".to_string(),
                name: "MIT License".to_string(),
                spdx_license_key: Some("MIT".to_string()),
                other_spdx_license_keys: vec![],
                category: Some("Permissive".to_string()),
                text: "MIT License text".to_string(),
                reference_urls: vec![],
                notes: None,
                is_deprecated: false,
                replaced_by: vec![],
                minimum_coverage: None,
                ignorable_copyrights: None,
                ignorable_holders: None,
                ignorable_authors: None,
                ignorable_urls: None,
                ignorable_emails: None,
            },
        );

        index.pattern_id_to_rid = vec![vec![0], vec![1], vec![2]];

        index.rid_by_spdx_key.insert("MIT".to_lowercase(), 0);
        index.rid_by_spdx_key.insert("Apache-2.0".to_lowercase(), 1);

        index.unknown_spdx_rid = Some(999);

        index
            .rids_by_high_tid
            .insert(TokenId::new(0), vec![0, 1].into_iter().collect());
        index
            .rids_by_high_tid
            .insert(TokenId::new(1), vec![2, 3].into_iter().collect());

        index
    }

    #[test]
    fn test_embedded_license_index_roundtrip() {
        let original = create_test_license_index();

        let embedded = EmbeddedLicenseIndex::from(&original);

        let restored = LicenseIndex::try_from(embedded).expect("Should deserialize");

        assert_eq!(restored.len_legalese, original.len_legalese);
        assert_eq!(restored.rid_by_hash.len(), original.rid_by_hash.len());
        assert_eq!(restored.rules_by_rid.len(), original.rules_by_rid.len());
        assert_eq!(restored.tids_by_rid.len(), original.tids_by_rid.len());
        assert_eq!(restored.sets_by_rid.len(), original.sets_by_rid.len());
        assert_eq!(restored.msets_by_rid.len(), original.msets_by_rid.len());
        assert_eq!(
            restored.high_sets_by_rid.len(),
            original.high_sets_by_rid.len()
        );
        assert_eq!(
            restored.high_postings_by_rid.len(),
            original.high_postings_by_rid.len()
        );
        assert_eq!(
            restored.false_positive_rids.len(),
            original.false_positive_rids.len()
        );
        assert_eq!(
            restored.approx_matchable_rids.len(),
            original.approx_matchable_rids.len()
        );
        assert_eq!(
            restored.licenses_by_key.len(),
            original.licenses_by_key.len()
        );
        assert_eq!(restored.pattern_id_to_rid, original.pattern_id_to_rid);
        assert_eq!(
            restored.rid_by_spdx_key.len(),
            original.rid_by_spdx_key.len()
        );
        assert_eq!(restored.unknown_spdx_rid, original.unknown_spdx_rid);
        assert_eq!(
            restored.rids_by_high_tid.len(),
            original.rids_by_high_tid.len()
        );
    }

    #[test]
    fn test_serialized_license_index_roundtrip() {
        let original = create_test_license_index();

        let embedded = EmbeddedLicenseIndex::from(&original);
        let serialized = embedded.serialize().expect("Should serialize");

        let bytes = serialized.to_bytes().expect("Should convert to bytes");
        let restored_serialized =
            SerializedLicenseIndex::from_bytes(&bytes).expect("Should parse bytes");

        let restored_embedded = restored_serialized.decompress().expect("Should decompress");
        let restored = LicenseIndex::try_from(restored_embedded).expect("Should deserialize");

        assert_eq!(restored.len_legalese, original.len_legalese);
        assert_eq!(restored.rid_by_hash.len(), original.rid_by_hash.len());
        assert_eq!(restored.rules_by_rid.len(), original.rules_by_rid.len());
        assert_eq!(
            restored.licenses_by_key.len(),
            original.licenses_by_key.len()
        );
    }

    #[test]
    fn test_schema_version_check() {
        let mut embedded = EmbeddedLicenseIndex::from(&create_test_license_index());
        embedded.schema_version = 999;

        let result = LicenseIndex::try_from(embedded);
        assert!(result.is_err());
        assert!(result.unwrap_err().0.contains("Schema version mismatch"));
    }

    #[test]
    fn test_token_dictionary_roundtrip() {
        let original = create_test_license_index();

        let embedded = EmbeddedTokenDictionary::from(&original.dictionary);
        let restored = TokenDictionary::from(embedded);

        assert_eq!(
            restored.legalese_count(),
            original.dictionary.legalese_count()
        );
        assert_eq!(
            restored.tokens_to_ids_len(),
            original.dictionary.tokens_to_ids_len()
        );
    }

    #[test]
    fn test_sorted_vectors_for_determinism() {
        let mut index = LicenseIndex::with_legalese_count(2);

        index.rid_by_hash.insert([3u8; 20], 2);
        index.rid_by_hash.insert([1u8; 20], 0);
        index.rid_by_hash.insert([2u8; 20], 1);

        index.licenses_by_key.insert(
            "zebra".to_string(),
            License {
                key: "zebra".to_string(),
                name: "Zebra".to_string(),
                spdx_license_key: None,
                other_spdx_license_keys: vec![],
                category: None,
                text: "".to_string(),
                reference_urls: vec![],
                notes: None,
                is_deprecated: false,
                replaced_by: vec![],
                minimum_coverage: None,
                ignorable_copyrights: None,
                ignorable_holders: None,
                ignorable_authors: None,
                ignorable_urls: None,
                ignorable_emails: None,
            },
        );
        index.licenses_by_key.insert(
            "alpha".to_string(),
            License {
                key: "alpha".to_string(),
                name: "Alpha".to_string(),
                spdx_license_key: None,
                other_spdx_license_keys: vec![],
                category: None,
                text: "".to_string(),
                reference_urls: vec![],
                notes: None,
                is_deprecated: false,
                replaced_by: vec![],
                minimum_coverage: None,
                ignorable_copyrights: None,
                ignorable_holders: None,
                ignorable_authors: None,
                ignorable_urls: None,
                ignorable_emails: None,
            },
        );

        let embedded1 = EmbeddedLicenseIndex::from(&index);
        let embedded2 = EmbeddedLicenseIndex::from(&index);

        let bytes1 =
            bincode::serde::encode_to_vec(&embedded1, bincode::config::standard()).unwrap();
        let bytes2 =
            bincode::serde::encode_to_vec(&embedded2, bincode::config::standard()).unwrap();

        assert_eq!(bytes1, bytes2, "Serialization should be deterministic");

        assert_eq!(embedded1.licenses_by_key[0].0, "alpha");
        assert_eq!(embedded1.licenses_by_key[1].0, "zebra");
    }

    #[test]
    fn test_load_embedded_license_index_artifact() {
        let artifact_bytes =
            include_bytes!("../../../resources/license_detection/license_index.bincode.zst");

        let embedded = EmbeddedLicenseIndex::deserialize_from_bytes(artifact_bytes)
            .expect("Should deserialize EmbeddedLicenseIndex");

        assert_eq!(
            embedded.schema_version, SCHEMA_VERSION,
            "Embedded schema version should match"
        );

        let license_index =
            LicenseIndex::try_from(embedded).expect("Should convert to LicenseIndex");

        assert!(
            !license_index.rules_by_rid.is_empty(),
            "Should have rules loaded"
        );
        assert!(
            !license_index.licenses_by_key.is_empty(),
            "Should have licenses loaded"
        );
        assert!(
            license_index.len_legalese > 0,
            "Should have legalese tokens"
        );
        assert!(
            !license_index.pattern_id_to_rid.is_empty(),
            "Should have pattern_id_to_rid mapping"
        );
    }
}
