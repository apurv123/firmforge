//! Artifact verification.
//!
//! R-SEC-1: every downloaded part is hashed and compared against the manifest
//! *before* a single byte is written to a device. No competitor in the
//! product research does this.

use crate::error::{Error, Result};
use sha2::{Digest, Sha256};

/// Lowercase hex SHA-256 of `bytes`.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

/// Verify `bytes` against an expected lowercase-hex SHA-256 digest.
pub fn verify_sha256(path: &str, bytes: &[u8], expected: &str) -> Result<()> {
    let actual = sha256_hex(bytes);
    if actual.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err(Error::ChecksumMismatch {
            path: path.to_string(),
            expected: expected.to_string(),
            actual,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_known_vector() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn accepts_a_matching_digest_case_insensitively() {
        let d = sha256_hex(b"firmware").to_uppercase();
        assert!(verify_sha256("app.bin", b"firmware", &d).is_ok());
    }

    #[test]
    fn rejects_tampered_bytes() {
        let d = sha256_hex(b"firmware");
        let err = verify_sha256("app.bin", b"f1rmware", &d).unwrap_err();
        assert!(matches!(err, Error::ChecksumMismatch { .. }));
    }
}
