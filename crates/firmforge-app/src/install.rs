//! Preparing an install: resolve → download → verify.
//!
//! Nothing here touches a device. The whole point is that every byte is fetched
//! and checked *before* the flashing step begins, so a network failure or a
//! corrupted download can never leave a half-written device. That ordering is
//! requirement R-SEC-1, and none of the competitors studied do it.

use firmforge_core::{
    manifest::Build,
    source::{self, Origin},
    verify::verify_sha256,
    Error, Result,
};
use serde::{Deserialize, Serialize};

/// One part, downloaded and checked, ready to be written.
#[derive(Debug, Clone)]
pub struct StagedPart {
    pub url: String,
    pub offset: u32,
    pub bytes: Vec<u8>,
    pub verified: bool,
}

/// What the user is shown before confirming, and what the flasher consumes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartSummary {
    pub url: String,
    /// `0x10000` rather than `65536`: offsets are conventionally read in hex.
    pub offset: String,
    pub size_bytes: usize,
    pub verified: bool,
    /// True when the bytes came from the upstream publisher rather than a
    /// rehosted copy. Surfaced so provenance is visible, per the licensing
    /// analysis in `plan/spec/legal-and-licensing.md`.
    pub upstream: bool,
}

/// A fully prepared, verified install.
pub struct Prepared {
    pub parts: Vec<StagedPart>,
    pub summaries: Vec<PartSummary>,
}

impl Prepared {
    pub fn total_bytes(&self) -> usize {
        self.parts.iter().map(|p| p.bytes.len()).sum()
    }
}

/// Download and verify every part of `build`.
///
/// `manifest_url` is the URL the manifest came from; relative part paths are
/// resolved against it exactly as ESP Web Tools does.
pub async fn prepare(build: &Build, manifest_url: &str) -> Result<Prepared> {
    let resolved = source::resolve_all(&build.parts, manifest_url)?;
    let http = crate::github::shared_client()?;

    let mut parts = Vec::with_capacity(resolved.len());
    let mut summaries = Vec::with_capacity(resolved.len());

    for part in resolved {
        let bytes = crate::github::get_bytes(&http, &part.url).await?;

        // A declared digest that does not match is fatal: refuse, never warn.
        let verified = match &part.sha256 {
            Some(expected) => {
                verify_sha256(&part.url, &bytes, expected)?;
                true
            }
            None => false,
        };

        if bytes.is_empty() {
            return Err(Error::Io(format!("{} is empty", part.url)));
        }

        summaries.push(PartSummary {
            url: part.url.clone(),
            offset: format!("0x{:X}", part.offset),
            size_bytes: bytes.len(),
            verified,
            upstream: part.origin == Origin::Upstream,
        });
        parts.push(StagedPart {
            url: part.url,
            offset: part.offset,
            bytes,
            verified,
        });
    }

    Ok(Prepared { parts, summaries })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offsets_are_presented_in_hex() {
        let s = PartSummary {
            url: "https://example.invalid/app.bin".into(),
            offset: format!("0x{:X}", 0x10000),
            size_bytes: 12,
            verified: true,
            upstream: true,
        };
        assert_eq!(s.offset, "0x10000");
    }
}
