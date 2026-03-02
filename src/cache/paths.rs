use std::path::{Path, PathBuf};

pub fn validate_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub fn scan_result_cache_path(scan_results_dir: &Path, sha256: &str) -> Option<PathBuf> {
    if !validate_sha256_hex(sha256) {
        return None;
    }

    let shard_one = &sha256[0..2];
    let shard_two = &sha256[2..4];
    Some(
        scan_results_dir
            .join(shard_one)
            .join(shard_two)
            .join(format!("{sha256}.msgpack.zst")),
    )
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    const VALID_SHA256: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    #[test]
    fn test_validate_sha256_hex_accepts_lowercase_hex() {
        assert!(validate_sha256_hex(VALID_SHA256));
    }

    #[test]
    fn test_validate_sha256_hex_accepts_uppercase_hex() {
        let value = VALID_SHA256.to_uppercase();
        assert!(validate_sha256_hex(&value));
    }

    #[test]
    fn test_validate_sha256_hex_rejects_wrong_length() {
        assert!(!validate_sha256_hex("abcd"));
    }

    #[test]
    fn test_validate_sha256_hex_rejects_non_hex() {
        assert!(!validate_sha256_hex(
            "g123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        ));
    }

    #[test]
    fn test_scan_result_cache_path_uses_two_level_sharding() {
        let base = PathBuf::from("/tmp/cache/scan-results");
        let path = scan_result_cache_path(&base, VALID_SHA256).expect("expected valid cache path");

        assert_eq!(
            path,
            base.join("01")
                .join("23")
                .join(format!("{VALID_SHA256}.msgpack.zst"))
        );
    }

    #[test]
    fn test_scan_result_cache_path_rejects_invalid_hash() {
        let base = PathBuf::from("/tmp/cache/scan-results");
        assert!(scan_result_cache_path(&base, "invalid").is_none());
    }
}
