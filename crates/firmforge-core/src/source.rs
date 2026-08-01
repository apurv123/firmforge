//! Resolving manifest parts to absolute download URLs.
//!
//! This module exists for a licensing reason as much as a technical one.
//!
//! Bruce is AGPL-3.0 and Meshtastic is GPL-3.0. Under §6 of those licences,
//! *conveying a binary* obliges the conveyor to offer the corresponding source
//! for that exact build. Mirroring `Bruce.bin` into your own `firmware/` folder
//! therefore makes you a distributor with real obligations; publishing a
//! manifest that *points at* the upstream release asset does not, because the
//! bytes travel from the original publisher to the user.
//!
//! So firmforge treats "reference upstream" as the default and rehosting as the
//! deliberate exception. See `plan/spec/legal-and-licensing.md` §2.

use crate::error::{Error, Result};
use crate::manifest::Part;
use url::Url;

/// Where a part's bytes will actually come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// Served from somewhere other than the manifest's own host: the bytes are
    /// the upstream publisher's, and no redistribution is taking place.
    Upstream,
    /// Served from the same host and path prefix as the manifest itself.
    Rehosted,
}

/// A part resolved to something that can be fetched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedPart {
    pub url: String,
    pub offset: u32,
    pub sha256: Option<String>,
    pub origin: Origin,
}

/// Resolve one part against the URL the manifest was fetched from.
///
/// Precedence: explicit `url`, then an absolute `path`, then `path` resolved
/// relative to `manifest_url` (the ESP Web Tools rule).
pub fn resolve(part: &Part, manifest_url: &str) -> Result<ResolvedPart> {
    let base =
        Url::parse(manifest_url).map_err(|e| Error::InvalidUrl(format!("{manifest_url}: {e}")))?;

    let raw = part.url.as_deref().unwrap_or(&part.path);
    let resolved = base
        .join(raw)
        .map_err(|e| Error::InvalidUrl(format!("{raw}: {e}")))?;

    let origin = if same_location(&base, &resolved) {
        Origin::Rehosted
    } else {
        Origin::Upstream
    };

    Ok(ResolvedPart {
        url: resolved.to_string(),
        offset: part.offset,
        sha256: part.sha256.clone(),
        origin,
    })
}

/// Resolve every part of a build, preserving flash order.
pub fn resolve_all(parts: &[Part], manifest_url: &str) -> Result<Vec<ResolvedPart>> {
    parts.iter().map(|p| resolve(p, manifest_url)).collect()
}

/// Same host, and sharing the manifest's directory prefix.
fn same_location(base: &Url, candidate: &Url) -> bool {
    if base.host_str() != candidate.host_str() {
        return false;
    }
    let dir = base
        .path()
        .rsplit_once('/')
        .map(|(head, _)| head)
        .unwrap_or("");
    candidate.path().starts_with(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn part(path: &str) -> Part {
        Part {
            path: path.into(),
            offset: 0x10000,
            sha256: None,
            url: None,
        }
    }

    const MANIFEST_URL: &str =
        "https://raw.githubusercontent.com/apurv123/firmforge/main/firmware/manifest.json";

    #[test]
    fn relative_paths_resolve_against_the_manifest() {
        let r = resolve(&part("app.bin"), MANIFEST_URL).unwrap();
        assert_eq!(
            r.url,
            "https://raw.githubusercontent.com/apurv123/firmforge/main/firmware/app.bin"
        );
        assert_eq!(r.origin, Origin::Rehosted, "a sibling file is our own copy");
    }

    #[test]
    fn absolute_upstream_paths_are_not_redistribution() {
        let r = resolve(
            &part("https://github.com/BruceDevices/firmware/releases/download/1.16/Bruce.bin"),
            MANIFEST_URL,
        )
        .unwrap();
        assert_eq!(r.origin, Origin::Upstream);
    }

    #[test]
    fn explicit_url_wins_over_path() {
        let mut p = part("app.bin");
        p.url = Some("https://github.com/o/r/releases/download/v1/app.bin".into());
        let r = resolve(&p, MANIFEST_URL).unwrap();
        assert!(r.url.starts_with("https://github.com/o/r"));
        assert_eq!(r.origin, Origin::Upstream);
    }

    #[test]
    fn order_and_offsets_survive_resolution() {
        let parts = vec![
            Part {
                path: "boot.bin".into(),
                offset: 0,
                sha256: None,
                url: None,
            },
            Part {
                path: "app.bin".into(),
                offset: 0x10000,
                sha256: None,
                url: None,
            },
        ];
        let r = resolve_all(&parts, MANIFEST_URL).unwrap();
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].offset, 0);
        assert_eq!(r[1].offset, 0x10000);
    }

    #[test]
    fn a_bad_manifest_url_is_an_error_not_a_panic() {
        assert!(resolve(&part("app.bin"), "not a url").is_err());
    }
}
