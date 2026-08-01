use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("failed to parse manifest: {0}")]
    ManifestParse(#[from] serde_json::Error),

    #[error("checksum mismatch for {path}: expected {expected}, computed {actual}")]
    ChecksumMismatch {
        path: String,
        expected: String,
        actual: String,
    },

    #[error("no build in this release is compatible with the connected device")]
    NoCompatibleBuild,

    #[error("io error: {0}")]
    Io(String),
}

pub type Result<T> = std::result::Result<T, Error>;
