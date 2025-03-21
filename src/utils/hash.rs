use md5::{Digest as Md5Digest, Md5};
use sha1::Sha1;
use sha2::Sha256;

/// Calculate SHA1 hash of content and return it as a hex string
pub fn calculate_sha1(content: &[u8]) -> String {
    let digest = Sha1::digest(content);
    format!("{:x}", digest)
}

/// Calculate MD5 hash of content and return it as a hex string
pub fn calculate_md5(content: &[u8]) -> String {
    let digest = Md5::digest(content);
    format!("{:x}", digest)
}

/// Calculate SHA256 hash of content and return it as a hex string
pub fn calculate_sha256(content: &[u8]) -> String {
    let digest = Sha256::digest(content);
    format!("{:x}", digest)
}
