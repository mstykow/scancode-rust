use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use rmp_serde::Serializer;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::cache::metadata::{CacheInvalidationKey, CacheSnapshotMetadata};

const SNAPSHOT_FILE_MAGIC: &[u8] = b"scancode-cache-v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheMissReason {
    NotFound,
    InvalidHeader,
    CorruptedData,
    IncompatibleMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotReadStatus {
    Hit(Vec<u8>),
    Miss(CacheMissReason),
}

#[derive(Debug)]
pub enum CacheIoError {
    Io(io::Error),
    Encode(rmp_serde::encode::Error),
}

impl fmt::Display for CacheIoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "I/O error: {err}"),
            Self::Encode(err) => write!(f, "cache encode error: {err}"),
        }
    }
}

impl std::error::Error for CacheIoError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::Encode(err) => Some(err),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheSnapshotEnvelope {
    metadata: CacheSnapshotMetadata,
    payload: Vec<u8>,
}

pub fn write_snapshot_payload(
    path: &Path,
    metadata: &CacheSnapshotMetadata,
    payload: &[u8],
) -> Result<(), CacheIoError> {
    let parent = path.parent().ok_or_else(|| {
        CacheIoError::Io(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Cache snapshot path has no parent: {:?}", path),
        ))
    })?;

    fs::create_dir_all(parent).map_err(CacheIoError::Io)?;

    let temp_path = temp_snapshot_path(path);
    let result = write_snapshot_payload_to_temp(&temp_path, metadata, payload)
        .and_then(|_| fs::rename(&temp_path, path).map_err(CacheIoError::Io));

    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }

    result
}

pub fn read_snapshot_payload(
    path: &Path,
    key: &CacheInvalidationKey<'_>,
) -> Result<SnapshotReadStatus, CacheIoError> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            return Ok(SnapshotReadStatus::Miss(CacheMissReason::NotFound));
        }
        Err(err) => return Err(CacheIoError::Io(err)),
    };

    let mut header = vec![0_u8; SNAPSHOT_FILE_MAGIC.len()];
    if let Err(err) = file.read_exact(&mut header) {
        if err.kind() == io::ErrorKind::UnexpectedEof {
            return Ok(SnapshotReadStatus::Miss(CacheMissReason::InvalidHeader));
        }
        return Err(CacheIoError::Io(err));
    }

    if header.as_slice() != SNAPSHOT_FILE_MAGIC {
        return Ok(SnapshotReadStatus::Miss(CacheMissReason::InvalidHeader));
    }

    let decoder = match zstd::Decoder::new(file) {
        Ok(decoder) => decoder,
        Err(_) => return Ok(SnapshotReadStatus::Miss(CacheMissReason::CorruptedData)),
    };

    let envelope: CacheSnapshotEnvelope = match rmp_serde::decode::from_read(decoder) {
        Ok(envelope) => envelope,
        Err(_) => return Ok(SnapshotReadStatus::Miss(CacheMissReason::CorruptedData)),
    };

    if !envelope.metadata.is_compatible_with(key) {
        return Ok(SnapshotReadStatus::Miss(
            CacheMissReason::IncompatibleMetadata,
        ));
    }

    Ok(SnapshotReadStatus::Hit(envelope.payload))
}

pub fn load_snapshot_payload(
    path: &Path,
    key: &CacheInvalidationKey<'_>,
) -> Result<Option<Vec<u8>>, CacheIoError> {
    match read_snapshot_payload(path, key)? {
        SnapshotReadStatus::Hit(payload) => Ok(Some(payload)),
        SnapshotReadStatus::Miss(_) => Ok(None),
    }
}

fn write_snapshot_payload_to_temp(
    temp_path: &Path,
    metadata: &CacheSnapshotMetadata,
    payload: &[u8],
) -> Result<(), CacheIoError> {
    let envelope = CacheSnapshotEnvelope {
        metadata: metadata.clone(),
        payload: payload.to_vec(),
    };

    let mut encoded_envelope = Vec::new();
    envelope
        .serialize(&mut Serializer::new(&mut encoded_envelope))
        .map_err(CacheIoError::Encode)?;

    let mut temp_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(temp_path)
        .map_err(CacheIoError::Io)?;

    temp_file
        .write_all(SNAPSHOT_FILE_MAGIC)
        .map_err(CacheIoError::Io)?;

    {
        let mut encoder = zstd::Encoder::new(&mut temp_file, 3).map_err(CacheIoError::Io)?;
        encoder
            .write_all(&encoded_envelope)
            .map_err(CacheIoError::Io)?;
        encoder.finish().map_err(CacheIoError::Io)?;
    }

    temp_file.sync_all().map_err(CacheIoError::Io)?;
    Ok(())
}

fn temp_snapshot_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("snapshot");
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    parent.join(format!(".tmp-{file_name}-{}", Uuid::new_v4()))
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::TempDir;

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

    fn fixture_key() -> CacheInvalidationKey<'static> {
        CacheInvalidationKey {
            cache_schema_version: 1,
            engine_version: "engine-v1",
            rules_fingerprint: "rules-abc",
            build_options_fingerprint: "opts-123",
        }
    }

    #[test]
    fn test_write_and_read_snapshot_round_trip() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let snapshot_path = temp_dir.path().join("index").join("snapshot.bin.zst");
        let payload = b"snapshot-payload";

        write_snapshot_payload(&snapshot_path, &fixture_metadata(), payload)
            .expect("Failed to write snapshot payload");

        let status = read_snapshot_payload(&snapshot_path, &fixture_key())
            .expect("Failed to read snapshot payload");

        assert_eq!(status, SnapshotReadStatus::Hit(payload.to_vec()));
        assert_eq!(
            load_snapshot_payload(&snapshot_path, &fixture_key()).expect("load should succeed"),
            Some(payload.to_vec())
        );
    }

    #[test]
    fn test_write_snapshot_creates_parent_directories() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let snapshot_path = temp_dir
            .path()
            .join("nested")
            .join("cache")
            .join("snapshot.bin.zst");

        write_snapshot_payload(&snapshot_path, &fixture_metadata(), b"payload")
            .expect("Failed to write snapshot payload");

        assert!(snapshot_path.exists());
    }

    #[test]
    fn test_read_snapshot_missing_file_is_not_found_miss() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let snapshot_path = temp_dir.path().join("missing.bin.zst");

        let status = read_snapshot_payload(&snapshot_path, &fixture_key())
            .expect("missing file should not error");

        assert_eq!(status, SnapshotReadStatus::Miss(CacheMissReason::NotFound));
        assert_eq!(
            load_snapshot_payload(&snapshot_path, &fixture_key()).expect("load should succeed"),
            None
        );
    }

    #[test]
    fn test_read_snapshot_incompatible_metadata_is_miss() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let snapshot_path = temp_dir.path().join("snapshot.bin.zst");

        write_snapshot_payload(&snapshot_path, &fixture_metadata(), b"payload")
            .expect("Failed to write snapshot payload");

        let key = CacheInvalidationKey {
            cache_schema_version: 1,
            engine_version: "engine-v2",
            rules_fingerprint: "rules-abc",
            build_options_fingerprint: "opts-123",
        };

        let status = read_snapshot_payload(&snapshot_path, &key).expect("read should succeed");
        assert_eq!(
            status,
            SnapshotReadStatus::Miss(CacheMissReason::IncompatibleMetadata)
        );
    }

    #[test]
    fn test_read_snapshot_invalid_header_is_miss() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let snapshot_path = temp_dir.path().join("snapshot.bin.zst");

        fs::write(&snapshot_path, b"invalid-header")
            .expect("Failed to write invalid snapshot file");

        let status =
            read_snapshot_payload(&snapshot_path, &fixture_key()).expect("read should succeed");
        assert_eq!(
            status,
            SnapshotReadStatus::Miss(CacheMissReason::InvalidHeader)
        );
    }

    #[test]
    fn test_read_snapshot_corrupted_payload_is_miss() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let snapshot_path = temp_dir.path().join("snapshot.bin.zst");

        let mut file = File::create(&snapshot_path).expect("Failed to create snapshot file");
        file.write_all(SNAPSHOT_FILE_MAGIC)
            .expect("Failed to write snapshot magic header");
        file.write_all(b"not-zstd-data")
            .expect("Failed to write corrupted payload");

        let status =
            read_snapshot_payload(&snapshot_path, &fixture_key()).expect("read should succeed");
        assert_eq!(
            status,
            SnapshotReadStatus::Miss(CacheMissReason::CorruptedData)
        );
    }
}
