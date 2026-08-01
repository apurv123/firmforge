//! Loading the built-in sources, and coping when they rot.
//!
//! The built-in list in `firmforge_core::builtin` is a snapshot taken when this
//! version of firmforge was released. Publishers move URLs, pin versions into
//! paths and retire releases, so some of those links *will* break — WLED's, for
//! instance, has a release number in it.
//!
//! The rule here: a stale built-in is a normal, expected state, not an error.
//! It gets loaded alongside the others, marked unavailable with the reason in
//! plain language, and the user can remove it. Nothing is silently dropped,
//! because a source vanishing without explanation is worse than one that is
//! visibly broken.

use crate::github::{self, DiscoveredManifest, Source};
use firmforge_core::builtin::{self, Availability, BuiltinSource, Tier};
use serde::{Deserialize, Serialize};

/// The result of trying to load one built-in source.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceStatus {
    /// Stable built-in id, or the slug for a user-added source.
    pub id: String,
    pub name: String,
    pub summary: String,
    pub tier: Tier,
    pub license: String,
    pub homepage: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caution: Option<String>,
    /// What the user would have typed to add this.
    pub target: String,
    /// Whether the manifest actually loaded.
    pub available: bool,
    /// Plain-language reason when it did not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub problem: Option<String>,
    pub manifests: Vec<DiscoveredManifest>,
    pub build_count: usize,
}

impl SourceStatus {
    fn from_builtin(source: &BuiltinSource) -> Self {
        SourceStatus {
            id: source.id.clone(),
            name: source.name.clone(),
            summary: source.summary.clone(),
            tier: source.tier,
            license: source.license.clone(),
            homepage: source.homepage.clone(),
            caution: source.caution.clone(),
            target: source.target().to_string(),
            available: false,
            problem: None,
            manifests: Vec::new(),
            build_count: 0,
        }
    }
}

/// Load one built-in source, turning any failure into a described state rather
/// than an error the caller has to decide how to render.
pub async fn load_builtin(source: &BuiltinSource) -> SourceStatus {
    let mut status = SourceStatus::from_builtin(source);

    if source.availability == Availability::NoManifestYet {
        status.problem = Some(format!(
            "{} does not publish a firmware manifest, so firmforge cannot tell \
             where each file belongs in flash. Guessing those offsets can brick \
             a board, so it will not.",
            source.name
        ));
        return status;
    }

    let parsed = match Source::parse(source.target()) {
        Ok(p) => p,
        Err(e) => {
            status.problem = Some(e.to_string());
            return status;
        }
    };

    match github::discover(&parsed).await {
        Ok(manifests) => {
            status.build_count = manifests.iter().map(|m| m.manifest.builds.len()).sum();
            status.available = true;
            status.manifests = manifests;
        }
        Err(e) => {
            status.problem = Some(stale_message(source, &e.to_string()));
        }
    }
    status
}

/// Load everything added on a fresh install.
///
/// Sequential on purpose: it is three requests, it keeps the crate free of an
/// async-combinator dependency, and a failing source must not affect the ones
/// after it.
pub async fn load_bundled() -> Vec<SourceStatus> {
    let mut out = Vec::new();
    for source in builtin::bundled() {
        out.push(load_builtin(&source).await);
    }
    out
}

/// Load a specific set of built-ins by id, so the app can respect the user's
/// removals instead of reloading the full default list every launch.
pub async fn load_builtins(ids: &[String]) -> Vec<SourceStatus> {
    let mut out = Vec::new();
    for id in ids {
        if let Some(source) = builtin::find(id) {
            out.push(load_builtin(&source).await);
        }
    }
    out
}

/// Explain a failed built-in in terms of what the user can do about it, rather
/// than echoing an HTTP status.
fn stale_message(source: &BuiltinSource, error: &str) -> String {
    let lower = error.to_lowercase();
    if lower.contains("404") || lower.contains("not found") {
        format!(
            "{} has moved or retired this download. The link shipped with \
             firmforge (checked {}) is out of date — remove this source and add \
             the current one from {}.",
            source.name,
            builtin::CURATED_ON,
            source.homepage
        )
    } else if lower.contains("rate limit") {
        "GitHub's hourly limit for anonymous requests was reached. This will \
         clear on its own within the hour."
            .to_string()
    } else if lower.contains("timed out") || lower.contains("timeout") {
        format!("{} did not respond in time. Worth retrying.", source.name)
    } else {
        format!("Could not load {}: {error}", source.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_source_with_no_manifest_reports_why_without_a_network_call() {
        let bruce = builtin::find("bruce").expect("bruce is in the built-in list");
        let status = load_builtin(&bruce).await;
        assert!(!status.available);
        assert!(status.problem.unwrap().contains("brick"));
    }

    #[test]
    fn stale_links_are_explained_as_staleness_not_as_http() {
        let wled = builtin::find("wled").unwrap();
        let message = stale_message(&wled, "404 Not Found fetching https://install.wled.me/...");
        assert!(message.contains("moved or retired"));
        assert!(message.contains(builtin::CURATED_ON));
        assert!(
            !message.contains("404"),
            "don't make the user read status codes"
        );
    }

    #[test]
    fn rate_limiting_is_described_as_temporary() {
        let esphome = builtin::find("esphome-web").unwrap();
        let message = stale_message(&esphome, "403 (GitHub API rate limit reached)");
        assert!(message.contains("clear on its own"));
    }
}
