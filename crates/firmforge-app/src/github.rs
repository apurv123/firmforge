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

/// A firmware source the user has added.
///
/// Two shapes, because the ecosystem has two shapes. Most projects do *not*
/// keep their manifest in the repository — ESPHome, WLED and Tasmota all
/// publish theirs to GitHub Pages or a CDN. Supporting only `owner/repo` would
/// have made the entire built-in catalogue impossible.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Source {
    /// A GitHub repository, searched for a manifest.
    Repo { owner: String, repo: String },
    /// A manifest fetched directly from a URL.
    Manifest { url: String },
}

impl Source {
    /// Parse whatever the user pasted.
    ///
    /// A URL ending in `.json` is taken as a direct manifest; a GitHub URL or
    /// `owner/repo` is taken as a repository. This ordering matters, because
    /// `https://github.com/o/r/releases/download/v1/manifest.json` is a URL to
    /// a manifest, not an instruction to search the repo.
    pub fn parse(input: &str) -> Result<Self> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err(Error::InvalidUrl("nothing to add".into()));
        }

        let is_url = trimmed.starts_with("http://") || trimmed.starts_with("https://");
        if is_url && looks_like_manifest(trimmed) {
            return Ok(Source::Manifest {
                url: trimmed.to_string(),
            });
        }

        let cleaned = trimmed
            .trim_end_matches('/')
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .trim_start_matches("www.")
            .trim_start_matches("github.com/");

        let mut parts = cleaned.split('/').filter(|s| !s.is_empty());
        match (parts.next(), parts.next()) {
            (Some(owner), Some(repo)) => Ok(Source::Repo {
                owner: owner.to_string(),
                repo: repo.trim_end_matches(".git").to_string(),
            }),
            _ => Err(Error::InvalidUrl(format!(
                "expected owner/repo or a link to a manifest.json, got '{input}'"
            ))),
        }
    }

    /// How this source is identified in the catalogue and in saved settings.
    pub fn slug(&self) -> String {
        match self {
            Source::Repo { owner, repo } => format!("{owner}/{repo}"),
            Source::Manifest { url } => url.clone(),
        }
    }
}

/// A URL points at a manifest if it names a `.json` file. Deliberately loose:
/// publishers use `manifest.json`, `release.tasmota.manifest.json`,
/// `esp32-s3.json` and worse.
fn looks_like_manifest(url: &str) -> bool {
    url.split(['?', '#'])
        .next()
        .unwrap_or(url)
        .to_ascii_lowercase()
        .ends_with(".json")
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

/// Discover every firmware manifest a source publishes.
pub async fn discover(source: &Source) -> Result<Vec<DiscoveredManifest>> {
    match source {
        Source::Manifest { url } => Ok(vec![manifest_from_url(url).await?]),
        Source::Repo { owner, repo } => discover_in_repo(source, owner, repo).await,
    }
}

/// Fetch a manifest straight from its published URL.
async fn manifest_from_url(url: &str) -> Result<DiscoveredManifest> {
    let http = client()?;
    let bytes = get_bytes(&http, url).await?;
    Ok(DiscoveredManifest {
        source: url.to_string(),
        manifest_url: url.to_string(),
        provenance: "published manifest".to_string(),
        manifest: Manifest::from_json(&bytes)?,
    })
}

async fn discover_in_repo(
    source: &Source,
    owner: &str,
    repo: &str,
) -> Result<Vec<DiscoveredManifest>> {
    let http = client()?;
    let mut found = Vec::new();

    if let Ok(m) = manifest_from_default_branch(&http, source, owner, repo).await {
        found.push(m);
    }
    found.extend(
        manifests_from_releases(&http, source, owner, repo)
            .await
            .unwrap_or_default(),
    );

    if found.is_empty() {
        return Err(Error::NoManifest(source.slug()));
    }
    Ok(found)
}

/// Route 1: `firmware/manifest.json` on the default branch.
async fn manifest_from_default_branch(
    http: &reqwest::Client,
    source: &Source,
    owner: &str,
    repo: &str,
) -> Result<DiscoveredManifest> {
    let info: RepoInfo = get_json(
        http,
        &format!("https://api.github.com/repos/{owner}/{repo}"),
    )
    .await?;

    let url = format!(
        "https://raw.githubusercontent.com/{}/{}/{}/firmware/manifest.json",
        owner, repo, info.default_branch
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
    owner: &str,
    repo: &str,
) -> Result<Vec<DiscoveredManifest>> {
    let releases: Vec<Release> = get_json(
        http,
        &format!("https://api.github.com/repos/{owner}/{repo}/releases?per_page=10"),
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

async fn get_json<T: serde::de::DeserializeOwned>(http: &reqwest::Client, url: &str) -> Result<T> {
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

    /// The built-in catalogue depends entirely on this: every bundled source
    /// is a published manifest URL, not a repository.
    #[test]
    fn parses_published_manifest_urls() {
        for input in [
            "https://esphome.github.io/firmware/esphome-web/manifest.json",
            "https://install.wled.me/bin/Release/release_0_15_3/manifest.json",
            "https://tasmota.github.io/install/manifest_ext/release.tasmota.manifest.json",
        ] {
            match Source::parse(input) {
                Ok(Source::Manifest { url }) => assert_eq!(url, input),
                other => panic!("{input} parsed as {other:?}"),
            }
        }
    }

    /// A release-asset link is a manifest, even though it is a github.com URL
    /// that would otherwise parse as a repository.
    #[test]
    fn a_manifest_url_on_github_is_not_treated_as_a_repo() {
        let url = "https://github.com/o/r/releases/download/v1/manifest.json";
        assert!(matches!(Source::parse(url), Ok(Source::Manifest { .. })));
    }

    #[test]
    fn query_strings_do_not_hide_the_json_extension() {
        assert!(matches!(
            Source::parse("https://example.com/manifest.json?v=2"),
            Ok(Source::Manifest { .. })
        ));
    }

    #[test]
    fn rejects_input_that_is_not_a_repository() {
        assert!(Source::parse("firmforge").is_err());
        assert!(Source::parse("").is_err());
        assert!(Source::parse("   ").is_err());
    }

    /// Every built-in must survive the parser, or the app breaks on first run
    /// for everybody.
    #[test]
    fn every_builtin_source_parses() {
        for builtin in firmforge_core::builtin::all() {
            let parsed =
                Source::parse(builtin.target()).unwrap_or_else(|e| panic!("{}: {e}", builtin.name));
            match (&builtin.locator, &parsed) {
                (firmforge_core::Locator::Repo(_), Source::Repo { .. }) => {}
                (firmforge_core::Locator::ManifestUrl(_), Source::Manifest { .. }) => {}
                _ => panic!("{} parsed into the wrong kind", builtin.name),
            }
        }
    }
}
