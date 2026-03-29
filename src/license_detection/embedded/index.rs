//! Embedded license index serialization and deserialization.
//!
//! This module provides the `EmbeddedLicenseIndex` struct which is a serializable
//! representation of the `LicenseIndex`. It uses sorted vectors for HashMaps to
//! ensure deterministic serialization, and byte vectors for automatons.

use std::collections::{HashMap, HashSet};
use std::ops::Range;

use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize, Serialize};

use crate::license_detection::automaton::Automaton;
use crate::license_detection::index::LicenseIndex;
use crate::license_detection::index::dictionary::{TokenDictionary, TokenId, TokenKind};
use crate::license_detection::models::{License, Rule, RuleKind};

pub const SCHEMA_VERSION: u32 = 3;
pub const EMBEDDED_LICENSE_INDEX_LFS_POINTER_PREFIX: &[u8] =
    b"version https://git-lfs.github.com/spec/v1\n";

pub type HighPostingsEntry = (u16, Vec<usize>);
pub type HighPostingsByRidEntry = (usize, Vec<HighPostingsEntry>);

#[derive(Debug, Clone)]
pub struct SerializationError(pub String);

impl std::fmt::Display for SerializationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "License index serialization error: {}", self.0)
    }
}

impl std::error::Error for SerializationError {}

pub fn embedded_index_artifact_setup_hint() -> &'static str {
    "Run ./setup.sh or cargo run --manifest-path xtask/Cargo.toml --bin generate-index-artifact"
}

fn validate_embedded_artifact_bytes(bytes: &[u8]) -> Result<(), SerializationError> {
    if bytes.is_empty() {
        return Err(SerializationError(format!(
            "Embedded license index artifact is empty. {}.",
            embedded_index_artifact_setup_hint()
        )));
    }

    if bytes.starts_with(EMBEDDED_LICENSE_INDEX_LFS_POINTER_PREFIX) {
        return Err(SerializationError(format!(
            "Embedded license index artifact is a Git LFS pointer, not the real generated artifact. {}.",
            embedded_index_artifact_setup_hint()
        )));
    }

    Ok(())
}

#[derive(Debug, Clone, Copy, Archive, RkyvSerialize, RkyvDeserialize, Serialize, Deserialize)]
pub struct EmbeddedRange {
    pub start: usize,
    pub end: usize,
}

impl From<Range<usize>> for EmbeddedRange {
    fn from(range: Range<usize>) -> Self {
        Self {
            start: range.start,
            end: range.end,
        }
    }
}

impl From<EmbeddedRange> for Range<usize> {
    fn from(embedded: EmbeddedRange) -> Self {
        embedded.start..embedded.end
    }
}

impl From<ArchivedEmbeddedRange> for EmbeddedRange {
    fn from(archived: ArchivedEmbeddedRange) -> Self {
        Self {
            start: u32::from(archived.start) as usize,
            end: u32::from(archived.end) as usize,
        }
    }
}

#[derive(Debug, Clone, Archive, RkyvSerialize, RkyvDeserialize, Serialize, Deserialize)]
pub struct EmbeddedTokenMetadata {
    pub kind: TokenKind,
    pub is_digit_only: bool,
    pub is_short_or_digit: bool,
}

#[derive(Debug, Clone, Archive, RkyvSerialize, RkyvDeserialize, Serialize, Deserialize)]
pub struct EmbeddedTokenDictionary {
    pub tokens_to_ids: Vec<(String, u16)>,
    pub token_metadata: Vec<Option<EmbeddedTokenMetadata>>,
    pub len_legalese: usize,
    pub next_id: u16,
}

#[derive(Debug, Clone, Archive, RkyvSerialize, RkyvDeserialize, Serialize, Deserialize)]
pub struct EmbeddedRule {
    pub identifier: String,
    pub license_expression: String,
    pub text: String,
    pub tokens: Vec<u16>,
    pub rule_kind: RuleKind,
    pub is_false_positive: bool,
    pub is_required_phrase: bool,
    pub is_from_license: bool,
    pub relevance: u8,
    pub minimum_coverage: Option<u8>,
    pub has_stored_minimum_coverage: bool,
    pub is_continuous: bool,
    pub required_phrase_spans: Vec<EmbeddedRange>,
    pub stopwords_by_pos: Vec<(usize, usize)>,
    pub referenced_filenames: Option<Vec<String>>,
    pub ignorable_urls: Option<Vec<String>>,
    pub ignorable_emails: Option<Vec<String>>,
    pub ignorable_copyrights: Option<Vec<String>>,
    pub ignorable_holders: Option<Vec<String>>,
    pub ignorable_authors: Option<Vec<String>>,
    pub language: Option<String>,
    pub notes: Option<String>,
    pub length_unique: usize,
    pub high_length_unique: usize,
    pub high_length: usize,
    pub min_matched_length: usize,
    pub min_high_matched_length: usize,
    pub min_matched_length_unique: usize,
    pub min_high_matched_length_unique: usize,
    pub is_small: bool,
    pub is_tiny: bool,
    pub starts_with_license: bool,
    pub ends_with_license: bool,
    pub is_deprecated: bool,
    pub spdx_license_key: Option<String>,
    pub other_spdx_license_keys: Vec<String>,
}

impl From<&Rule> for EmbeddedRule {
    fn from(rule: &Rule) -> Self {
        let mut stopwords_by_pos: Vec<(usize, usize)> = rule
            .stopwords_by_pos
            .iter()
            .map(|(k, v)| (*k, *v))
            .collect();
        stopwords_by_pos.sort_by_key(|(k, _)| *k);

        Self {
            identifier: rule.identifier.clone(),
            license_expression: rule.license_expression.clone(),
            text: rule.text.clone(),
            tokens: rule.tokens.iter().map(|t| t.raw()).collect(),
            rule_kind: rule.rule_kind,
            is_false_positive: rule.is_false_positive,
            is_required_phrase: rule.is_required_phrase,
            is_from_license: rule.is_from_license,
            relevance: rule.relevance,
            minimum_coverage: rule.minimum_coverage,
            has_stored_minimum_coverage: rule.has_stored_minimum_coverage,
            is_continuous: rule.is_continuous,
            required_phrase_spans: rule
                .required_phrase_spans
                .iter()
                .map(|r| EmbeddedRange::from(r.clone()))
                .collect(),
            stopwords_by_pos,
            referenced_filenames: rule.referenced_filenames.clone(),
            ignorable_urls: rule.ignorable_urls.clone(),
            ignorable_emails: rule.ignorable_emails.clone(),
            ignorable_copyrights: rule.ignorable_copyrights.clone(),
            ignorable_holders: rule.ignorable_holders.clone(),
            ignorable_authors: rule.ignorable_authors.clone(),
            language: rule.language.clone(),
            notes: rule.notes.clone(),
            length_unique: rule.length_unique,
            high_length_unique: rule.high_length_unique,
            high_length: rule.high_length,
            min_matched_length: rule.min_matched_length,
            min_high_matched_length: rule.min_high_matched_length,
            min_matched_length_unique: rule.min_matched_length_unique,
            min_high_matched_length_unique: rule.min_high_matched_length_unique,
            is_small: rule.is_small,
            is_tiny: rule.is_tiny,
            starts_with_license: rule.starts_with_license,
            ends_with_license: rule.ends_with_license,
            is_deprecated: rule.is_deprecated,
            spdx_license_key: rule.spdx_license_key.clone(),
            other_spdx_license_keys: rule.other_spdx_license_keys.clone(),
        }
    }
}

impl From<EmbeddedRule> for Rule {
    fn from(embedded: EmbeddedRule) -> Self {
        Self {
            identifier: embedded.identifier,
            license_expression: embedded.license_expression,
            text: embedded.text,
            tokens: embedded.tokens.into_iter().map(TokenId::new).collect(),
            rule_kind: embedded.rule_kind,
            is_false_positive: embedded.is_false_positive,
            is_required_phrase: embedded.is_required_phrase,
            is_from_license: embedded.is_from_license,
            relevance: embedded.relevance,
            minimum_coverage: embedded.minimum_coverage,
            has_stored_minimum_coverage: embedded.has_stored_minimum_coverage,
            is_continuous: embedded.is_continuous,
            required_phrase_spans: embedded
                .required_phrase_spans
                .into_iter()
                .map(Range::from)
                .collect(),
            stopwords_by_pos: embedded.stopwords_by_pos.into_iter().collect(),
            referenced_filenames: embedded.referenced_filenames,
            ignorable_urls: embedded.ignorable_urls,
            ignorable_emails: embedded.ignorable_emails,
            ignorable_copyrights: embedded.ignorable_copyrights,
            ignorable_holders: embedded.ignorable_holders,
            ignorable_authors: embedded.ignorable_authors,
            language: embedded.language,
            notes: embedded.notes,
            length_unique: embedded.length_unique,
            high_length_unique: embedded.high_length_unique,
            high_length: embedded.high_length,
            min_matched_length: embedded.min_matched_length,
            min_high_matched_length: embedded.min_high_matched_length,
            min_matched_length_unique: embedded.min_matched_length_unique,
            min_high_matched_length_unique: embedded.min_high_matched_length_unique,
            is_small: embedded.is_small,
            is_tiny: embedded.is_tiny,
            starts_with_license: embedded.starts_with_license,
            ends_with_license: embedded.ends_with_license,
            is_deprecated: embedded.is_deprecated,
            spdx_license_key: embedded.spdx_license_key,
            other_spdx_license_keys: embedded.other_spdx_license_keys,
        }
    }
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

impl From<&ArchivedEmbeddedTokenDictionary> for TokenDictionary {
    fn from(archived: &ArchivedEmbeddedTokenDictionary) -> Self {
        let embedded: EmbeddedTokenDictionary =
            rkyv::deserialize::<_, rkyv::rancor::Error>(archived).unwrap();
        embedded.into()
    }
}

impl From<&ArchivedEmbeddedRule> for Rule {
    fn from(archived: &ArchivedEmbeddedRule) -> Self {
        let embedded: EmbeddedRule = rkyv::deserialize::<_, rkyv::rancor::Error>(archived).unwrap();
        embedded.into()
    }
}

#[derive(Debug, Clone, Archive, RkyvSerialize, RkyvDeserialize, Serialize, Deserialize)]
pub struct EmbeddedLicenseIndex {
    pub schema_version: u32,
    pub dictionary: EmbeddedTokenDictionary,
    pub len_legalese: usize,
    pub rid_by_hash: Vec<([u8; 20], usize)>,
    pub rules_by_rid: Vec<EmbeddedRule>,
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
            rules_by_rid: index.rules_by_rid.iter().map(EmbeddedRule::from).collect(),
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

impl TryFrom<&ArchivedEmbeddedLicenseIndex> for LicenseIndex {
    type Error = SerializationError;

    fn try_from(archived: &ArchivedEmbeddedLicenseIndex) -> Result<Self, Self::Error> {
        if u32::from(archived.schema_version) != SCHEMA_VERSION {
            return Err(SerializationError(format!(
                "Schema version mismatch: expected {}, got {}",
                SCHEMA_VERSION,
                u32::from(archived.schema_version)
            )));
        }

        let dictionary = TokenDictionary::from(&archived.dictionary);

        let rid_by_hash: HashMap<[u8; 20], usize> = archived
            .rid_by_hash
            .iter()
            .map(|entry| (entry.0, u32::from(entry.1) as usize))
            .collect();

        let tids_by_rid: Vec<Vec<TokenId>> = archived
            .tids_by_rid
            .iter()
            .map(|tids| {
                tids.iter()
                    .map(|tid| TokenId::new(u16::from(*tid)))
                    .collect()
            })
            .collect();

        let rules_automaton = Automaton::deserialize_unchecked(archived.rules_automaton.as_slice());

        let unknown_automaton =
            Automaton::deserialize_unchecked(archived.unknown_automaton.as_slice());

        let sets_by_rid: HashMap<usize, HashSet<TokenId>> = archived
            .sets_by_rid
            .iter()
            .map(|entry| {
                (
                    u32::from(entry.0) as usize,
                    entry
                        .1
                        .iter()
                        .map(|tid| TokenId::new(u16::from(*tid)))
                        .collect(),
                )
            })
            .collect();

        let msets_by_rid: HashMap<usize, HashMap<TokenId, usize>> = archived
            .msets_by_rid
            .iter()
            .map(|entry| {
                (
                    u32::from(entry.0) as usize,
                    entry
                        .1
                        .iter()
                        .map(|pair| (TokenId::new(u16::from(pair.0)), u16::from(pair.1) as usize))
                        .collect(),
                )
            })
            .collect();

        let high_sets_by_rid: HashMap<usize, HashSet<TokenId>> = archived
            .high_sets_by_rid
            .iter()
            .map(|entry| {
                (
                    u32::from(entry.0) as usize,
                    entry
                        .1
                        .iter()
                        .map(|tid| TokenId::new(u16::from(*tid)))
                        .collect(),
                )
            })
            .collect();

        let high_postings_by_rid: HashMap<usize, HashMap<TokenId, Vec<usize>>> = archived
            .high_postings_by_rid
            .iter()
            .map(|entry| {
                (
                    u32::from(entry.0) as usize,
                    entry
                        .1
                        .iter()
                        .map(|pair| {
                            (
                                TokenId::new(u16::from(pair.0)),
                                pair.1.iter().map(|p| u32::from(*p) as usize).collect(),
                            )
                        })
                        .collect(),
                )
            })
            .collect();

        let false_positive_rids: HashSet<usize> = archived
            .false_positive_rids
            .iter()
            .map(|v| u32::from(*v) as usize)
            .collect();

        let approx_matchable_rids: HashSet<usize> = archived
            .approx_matchable_rids
            .iter()
            .map(|v| u32::from(*v) as usize)
            .collect();

        let licenses_by_key: HashMap<String, License> = archived
            .licenses_by_key
            .iter()
            .map(|entry| {
                (
                    entry.0.as_str().to_string(),
                    rkyv::deserialize::<_, rkyv::rancor::Error>(&entry.1).unwrap(),
                )
            })
            .collect();

        let rid_by_spdx_key: HashMap<String, usize> = archived
            .rid_by_spdx_key
            .iter()
            .map(|entry| (entry.0.as_str().to_string(), u32::from(entry.1) as usize))
            .collect();

        let rids_by_high_tid: HashMap<TokenId, HashSet<usize>> = archived
            .rids_by_high_tid
            .iter()
            .map(|entry| {
                (
                    TokenId::new(u16::from(entry.0)),
                    entry.1.iter().map(|v| u32::from(*v) as usize).collect(),
                )
            })
            .collect();

        let rules_by_rid: Vec<Rule> = archived.rules_by_rid.iter().map(Rule::from).collect();

        Ok(LicenseIndex {
            dictionary,
            len_legalese: u32::from(archived.len_legalese) as usize,
            rid_by_hash,
            rules_by_rid,
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
            pattern_id_to_rid: archived
                .pattern_id_to_rid
                .iter()
                .map(|v| v.iter().map(|p| u32::from(*p) as usize).collect())
                .collect(),
            rid_by_spdx_key,
            unknown_spdx_rid: match &archived.unknown_spdx_rid {
                rkyv::option::ArchivedOption::Some(v) => Some(u32::from(*v) as usize),
                rkyv::option::ArchivedOption::None => None,
            },
            rids_by_high_tid,
        })
    }
}

// This method is used by the xtask build process.
impl EmbeddedLicenseIndex {
    pub fn serialize_to_bytes(&self) -> Result<Vec<u8>, SerializationError> {
        let rkyv_data = rkyv::to_bytes::<rkyv::rancor::Error>(self).map_err(|e| {
            SerializationError(format!("Failed to serialize EmbeddedLicenseIndex: {}", e))
        })?;

        zstd::encode_all(&rkyv_data[..], 0)
            .map_err(|e| SerializationError(format!("Failed to compress license index: {}", e)))
    }
}

pub fn load_license_index_from_bytes(bytes: &[u8]) -> Result<LicenseIndex, SerializationError> {
    validate_embedded_artifact_bytes(bytes)?;

    let decompressed = zstd::decode_all(bytes)
        .map_err(|e| SerializationError(format!("Failed to decompress license index: {}", e)))?;

    let archived: &ArchivedEmbeddedLicenseIndex =
        rkyv::access::<_, rkyv::rancor::Error>(&decompressed).map_err(|e| {
            SerializationError(format!(
                "Failed to access archived EmbeddedLicenseIndex: {}",
                e
            ))
        })?;

    LicenseIndex::try_from(archived)
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
                short_name: Some("MIT".to_string()),
                name: "MIT License".to_string(),
                language: Some("en".to_string()),
                spdx_license_key: Some("MIT".to_string()),
                other_spdx_license_keys: vec![],
                category: Some("Permissive".to_string()),
                owner: Some("Example Owner".to_string()),
                homepage_url: Some("https://example.com/license".to_string()),
                text: "MIT License text".to_string(),
                reference_urls: vec!["https://example.com/license".to_string()],
                osi_license_key: Some("MIT".to_string()),
                text_urls: vec!["https://example.com/text".to_string()],
                osi_url: Some("https://example.com/osi".to_string()),
                faq_url: None,
                other_urls: vec![],
                notes: None,
                is_deprecated: false,
                is_exception: false,
                is_unknown: false,
                is_generic: false,
                replaced_by: vec![],
                minimum_coverage: None,
                standard_notice: None,
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
        let bytes = embedded.serialize_to_bytes().expect("Should serialize");
        let restored = load_license_index_from_bytes(&bytes).expect("Should deserialize");

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
    fn test_embedded_license_index_bytes_roundtrip() {
        let original = create_test_license_index();

        let embedded = EmbeddedLicenseIndex::from(&original);
        let bytes = embedded.serialize_to_bytes().expect("Should serialize");

        let restored = load_license_index_from_bytes(&bytes).expect("Should deserialize");

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

        let rkyv_bytes =
            rkyv::to_bytes::<rkyv::rancor::Error>(&embedded).expect("Should serialize");
        let archived: &ArchivedEmbeddedLicenseIndex =
            rkyv::access::<_, rkyv::rancor::Error>(&rkyv_bytes).expect("Should access");

        let result = LicenseIndex::try_from(archived);
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
                short_name: Some("Zebra".to_string()),
                name: "Zebra".to_string(),
                language: Some("en".to_string()),
                spdx_license_key: None,
                other_spdx_license_keys: vec![],
                category: None,
                owner: None,
                homepage_url: None,
                text: "".to_string(),
                reference_urls: vec![],
                osi_license_key: None,
                text_urls: vec![],
                osi_url: None,
                faq_url: None,
                other_urls: vec![],
                notes: None,
                is_deprecated: false,
                is_exception: false,
                is_unknown: false,
                is_generic: false,
                replaced_by: vec![],
                minimum_coverage: None,
                standard_notice: None,
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
                short_name: Some("Alpha".to_string()),
                name: "Alpha".to_string(),
                language: Some("en".to_string()),
                spdx_license_key: None,
                other_spdx_license_keys: vec![],
                category: None,
                owner: None,
                homepage_url: None,
                text: "".to_string(),
                reference_urls: vec![],
                osi_license_key: None,
                text_urls: vec![],
                osi_url: None,
                faq_url: None,
                other_urls: vec![],
                notes: None,
                is_deprecated: false,
                is_exception: false,
                is_unknown: false,
                is_generic: false,
                replaced_by: vec![],
                minimum_coverage: None,
                standard_notice: None,
                ignorable_copyrights: None,
                ignorable_holders: None,
                ignorable_authors: None,
                ignorable_urls: None,
                ignorable_emails: None,
            },
        );

        let embedded1 = EmbeddedLicenseIndex::from(&index);
        let embedded2 = EmbeddedLicenseIndex::from(&index);

        let bytes1 = embedded1.serialize_to_bytes().unwrap();
        let bytes2 = embedded2.serialize_to_bytes().unwrap();

        assert_eq!(bytes1, bytes2, "Serialization should be deterministic");

        assert_eq!(embedded1.licenses_by_key[0].0, "alpha");
        assert_eq!(embedded1.licenses_by_key[1].0, "zebra");
    }

    #[test]
    fn test_load_embedded_license_index_artifact() {
        let artifact_bytes =
            include_bytes!("../../../resources/license_detection/license_index.zst");

        let license_index = load_license_index_from_bytes(artifact_bytes)
            .expect("Should load LicenseIndex from bytes");

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

    #[test]
    fn test_load_license_index_from_bytes_rejects_git_lfs_pointer() {
        let pointer =
            b"version https://git-lfs.github.com/spec/v1\noid sha256:deadbeef\nsize 123\n";

        let error = load_license_index_from_bytes(pointer).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("Git LFS pointer, not the real generated artifact")
        );
        assert!(error.to_string().contains("generate-index-artifact"));
    }
}
