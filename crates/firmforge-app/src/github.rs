//! Reading firmware out of a GitHub repository.
//!
//! This implements the original requirement: *"a desktop application that can
//! access firmware from my github repository's firmware folder"*.
//!
//! Two discovery routes, tried in order:
//!
//! 1. **`firmware/manifest.json` on the default branch** — the repository
//!    convention documented in `firmware/README.md`. This is the primary path
//!    and the one the user's own repo uses.
//! 2. **A `manifest.json` asset attached to a GitHub release** — how ESPHome
//!    and the wider ESP Web Tools ecosystem publish, so third-party firmware
//!    works without the publisher changing anything.
//!
//! Unauthenticated GitHub API access is 60 requests/hour, which is ample here
//! (one or two calls per source refresh) but is why results are cached.

use firmforge_core::{Error, Manifest, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

const USER_AGENT: &str = concat!("firmforge/", env!("CARGO_PKG_VERSION"));

/// A firmware repository the user has added.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    pub owner: String,
    pub repo: String,
}

impl Source {
    /// Parse `owner/repo`, a full GitHub URL, or `github.com/owner/repo`.
    pub fn parse(input: &str) -> Result<Self> {
        let cleaned = input
            .trim()
            .trim_end_matches('/')
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .trim_start_matches("www.")
            .trim_start_matches("github.com/");

        let mut parts = cleaned.split('/').filter(|s| !s.is_empty());
        match (parts.next(), parts.next()) {
            (Some(owner), Some(repo)) => Ok(Source {
                owner: owner.to_string(),
                repo: repo.trim_end_matches(".git").to_string(),
            }),
            _ => Err(Error::InvalidUrl(format!(
                "expected owner/repo, got '{input}'"
            ))),
        }
    }

    pub fn slug(&self) -> String {
        format!("{}/{}", self.owner, self.repo)
    }
}

/// A manifest found in a repository, with the URL it came from so that relative
/// part paths can be resolved correctly.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredManifest {
    pub source: String,
    /// Where the manifest itself was fetched from; the base for part paths.
    pub manifest_url: String,
    /// Human-readable provenance, e.g. `main branch` or `release v1.16`.
    pub provenance: String,
    pub manifest: Manifest,
}

fn client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| Error::Network(e.to_string()))
}

#[derive(Deserialize)]
struct RepoInfo {
    default_branch: String,
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    assets: Vec<ReleaseAsset>,
}

#[derive(Deserialize)]
struct ReleaseAsset {
    name: String,
    browser_download_url: String,
}

/// Discover every firmware manifest a repository publishes.
pub async fn discover(source: &Source) -> Result<Vec<DiscoveredManifest>> {
    let http = client()?;
    let mut found = Vec::new();

    if let Ok(m) = manifest_from_default_branch(&http, source).await {
        found.push(m);
    }
    found.extend(manifests_from_releases(&http, source).await.unwrap_or_default());

    if found.is_empty() {
        return Err(Error::NoManifest(source.slug()));
    }
    Ok(found)
}

/// Route 1: `firmware/manifest.json` on the default branch.
async fn manifest_from_default_branch(
    http: &reqwest::Client,
    source: &Source,
) -> Result<DiscoveredManifest> {
    let info: RepoInfo = get_json(
        http,
        &format!(
            "https://api.github.com/repos/{}/{}",
            source.owner, source.repo
        ),
    )
    .await?;

    let url = format!(
        "https://raw.githubusercontent.com/{}/{}/{}/firmware/manifest.json",
        source.owner, source.repo, info.default_branch
    );
    let bytes = get_bytes(http, &url).await?;

    Ok(DiscoveredManifest {
        source: source.slug(),
        manifest_url: url,
        provenance: format!("{} branch", info.default_branch),
        manifest: Manifest::from_json(&bytes)?,
    })
}

/// Route 2: a `manifest.json` attached to any of the ten most recent releases.
async fn manifests_from_releases(
    http: &reqwest::Client,
    source: &Source,
) -> Result<Vec<DiscoveredManifest>> {
    let releases: Vec<Release> = get_json(
        http,
        &format!(
            "https://api.github.com/repos/{}/{}/releases?per_page=10",
            source.owner, source.repo
        ),
    )
    .await?;

    let mut out = Vec::new();
    for release in releases {
        let Some(asset) = release
            .assets
            .iter()
            .find(|a| a.name.eq_ignore_ascii_case("manifest.json"))
        else {
            continue;
        };
        let Ok(bytes) = get_bytes(http, &asset.browser_download_url).await else {
            continue;
        };
        let Ok(manifest) = Manifest::from_json(&bytes) else {
            continue;
        };

        out.push(DiscoveredManifest {
            source: source.slug(),
            manifest_url: asset.browser_download_url.clone(),
            provenance: if release.prerelease {
                format!("release {} (pre-release)", release.tag_name)
            } else {
                format!("release {}", release.tag_name)
            },
            manifest,
        });
    }
    Ok(out)
}

async fn get_json<T: serde::de::DeserializeOwned>(
    http: &reqwest::Client,
    url: &str,
) -> Result<T> {
    let response = http
        .get(url)
        .send()
        .await
        .map_err(|e| Error::Network(e.to_string()))?;
    check_status(&response, url)?;
    response
        .json::<T>()
        .await
        .map_err(|e| Error::Network(e.to_string()))
}

/// Fetch raw bytes. Used for manifests here and for firmware parts in
/// `download`, so the rate-limit and error messages stay consistent.
pub async fn get_bytes(http: &reqwest::Client, url: &str) -> Result<Vec<u8>> {
    let response = http
        .get(url)
        .send()
        .await
        .map_err(|e| Error::Network(e.to_string()))?;
    check_status(&response, url)?;
    Ok(response
        .bytes()
        .await
        .map_err(|e| Error::Network(e.to_string()))?
        .to_vec())
}

fn check_status(response: &reqwest::Response, url: &str) -> Result<()> {
    let status = response.status();
    if status.is_success() {
        return Ok(());
    }
    // 403 with a zero remaining budget is the rate limit, not a permissions
    // problem; saying so saves the user a confusing detour.
    let hint = if status.as_u16() == 403
        && response
            .headers()
            .get("x-ratelimit-remaining")
            .and_then(|v| v.to_str().ok())
            == Some("0")
    {
        " (GitHub API rate limit reached — 60 requests/hour without sign-in)"
    } else if status.as_u16() == 404 {
        " (not found — check the repository name, and that it is public)"
    } else {
        ""
    };
    Err(Error::Network(format!("{status} fetching {url}{hint}")))
}

pub fn shared_client() -> Result<reqwest::Client> {
    client()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_shapes_users_actually_paste() {
        for input in [
            "apurv123/firmforge",
            "https://github.com/apurv123/firmforge",
            "github.com/apurv123/firmforge/",
            "https://github.com/apurv123/firmforge.git",
        ] {
            let s = Source::parse(input).unwrap_or_else(|e| panic!("{input}: {e}"));
            assert_eq!(s.slug(), "apurv123/firmforge", "input was {input}");
        }
    }

    #[test]
    fn rejects_input_that_is_not_a_repository() {
        assert!(Source::parse("firmforge").is_err());
        assert!(Source::parse("").is_err());
    }
}
