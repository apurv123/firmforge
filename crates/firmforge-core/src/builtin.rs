//! The built-in source list, which exists to solve cold start.
//!
//! A fresh install with an empty catalogue is useless: the user has no idea
//! which repositories to type in, and the app cannot demonstrate anything. So
//! firmforge ships with a curated list.
//!
//! Two tiers, and the split is deliberate:
//!
//! * **Bundled** sources are added automatically on first run. They are all
//!   mainstream, benign firmware — home automation and LED control. Nothing
//!   here creates a licensing or app-store problem.
//!
//! * **Suggested** sources are shown but *not* added. The user must add them
//!   explicitly. These are offensive-security tools; keeping them opt-in is
//!   what preserves the "the user supplies the catalogue" position that keeps
//!   the mobile app shippable and firmforge neutral about what firmware does.
//!   See `plan/spec/legal-and-licensing.md` §3.
//!
//! Every entry is removable. Removals are remembered, so a bundled source the
//! user deletes does not silently come back on the next launch.
//!
//! **On staleness:** these URLs are a snapshot, and some are version-pinned by
//! the publisher (WLED's manifest path contains a release number). They will rot.
//! firmforge therefore treats a built-in that fails to load as *stale*, says so
//! plainly, and lets the user remove it — rather than showing a broken source
//! forever or, worse, silently hiding it.

use serde::{Deserialize, Serialize};

/// Whether a built-in is added automatically or only offered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Tier {
    /// Added on first run.
    Bundled,
    /// Offered, but the user must add it deliberately.
    Suggested,
}

/// How to reach a source's manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind", content = "value")]
pub enum Locator {
    /// A GitHub repository, discovered via `firmware/manifest.json` and releases.
    Repo(String),
    /// A direct manifest URL. Most established projects publish their ESP Web
    /// Tools manifest on GitHub Pages rather than inside the repository, so
    /// supporting this is what makes the built-in list possible at all.
    ManifestUrl(String),
}

/// Whether firmforge can actually install from this source today.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Availability {
    /// Publishes a machine-readable manifest firmforge can read.
    Ready,
    /// A real, popular project that ships bare `.bin` files with no manifest,
    /// so offsets and chip targets cannot be determined safely. Listed anyway,
    /// because pretending it does not exist helps nobody — but it is shown as
    /// a link, not as something that can be added and will then fail.
    NoManifestYet,
}

/// One curated firmware source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuiltinSource {
    /// Stable key used to remember removals across upgrades.
    pub id: String,
    pub name: String,
    pub summary: String,
    pub locator: Locator,
    pub tier: Tier,
    pub availability: Availability,
    pub license: String,
    /// Chip families this source is known to publish builds for.
    pub families: Vec<String>,
    pub homepage: String,
    /// Shown before a suggested source is added.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caution: Option<String>,
}

/// The date this list was last checked against the live URLs, so the app can
/// tell the user how old its defaults are instead of pretending they are fresh.
pub const CURATED_ON: &str = "2026-08-01";

/// The curated list.
///
/// Every bundled URL below was fetched and confirmed to parse, and to contain
/// an ESP32-S3 build, on `CURATED_ON`.
pub fn all() -> Vec<BuiltinSource> {
    vec![
        BuiltinSource {
            id: "esphome-web".into(),
            name: "ESPHome Web".into(),
            summary: "Adopt a board into ESPHome and Home Assistant. The reference \
                      ESP Web Tools publisher."
                .into(),
            locator: Locator::ManifestUrl(
                "https://esphome.github.io/firmware/esphome-web/manifest.json".into(),
            ),
            tier: Tier::Bundled,
            availability: Availability::Ready,
            license: "GPL-3.0 / Apache-2.0".into(),
            families: vec![
                "ESP32".into(),
                "ESP32-C3".into(),
                "ESP32-C6".into(),
                "ESP32-S2".into(),
                "ESP32-S3".into(),
                "ESP8266".into(),
            ],
            homepage: "https://esphome.io".into(),
            caution: None,
        },
        BuiltinSource {
            id: "wled".into(),
            name: "WLED".into(),
            summary: "Addressable LED control over WiFi. One of the most widely \
                      installed ESP32 firmwares there is."
                .into(),
            locator: Locator::ManifestUrl(
                "https://install.wled.me/bin/Release/release_0_15_3/manifest.json".into(),
            ),
            tier: Tier::Bundled,
            availability: Availability::Ready,
            license: "EUPL-1.2".into(),
            families: vec![
                "ESP32".into(),
                "ESP32-C3".into(),
                "ESP32-S2".into(),
                "ESP32-S3".into(),
                "ESP8266".into(),
            ],
            homepage: "https://kno.wled.ge".into(),
            caution: None,
        },
        BuiltinSource {
            id: "tasmota".into(),
            name: "Tasmota".into(),
            summary: "Control smart plugs, switches and sensors locally, without \
                      the vendor's cloud."
                .into(),
            locator: Locator::ManifestUrl(
                "https://tasmota.github.io/install/manifest_ext/release.tasmota.manifest.json"
                    .into(),
            ),
            tier: Tier::Bundled,
            availability: Availability::Ready,
            license: "GPL-3.0".into(),
            families: vec![
                "ESP32".into(),
                "ESP32-C3".into(),
                "ESP32-C6".into(),
                "ESP32-S2".into(),
                "ESP32-S3".into(),
                "ESP8266".into(),
            ],
            homepage: "https://tasmota.github.io".into(),
            caution: None,
        },
        BuiltinSource {
            id: "marauder".into(),
            name: "ESP32 Marauder".into(),
            summary: "WiFi and Bluetooth analysis toolkit for M5Stack and CYD boards. \
                      Upstream publishes its own installer format rather than a \
                      standard manifest, so firmforge maintains one that points at \
                      Marauder's official release downloads."
                .into(),
            locator: Locator::Repo("apurv123/firmforge".into()),
            tier: Tier::Suggested,
            availability: Availability::Ready,
            license: "MIT".into(),
            families: vec!["ESP32".into(), "ESP32-S3".into()],
            homepage: "https://github.com/justcallmekoko/ESP32Marauder".into(),
            caution: Some(
                "Offensive-security tooling. Only use it on networks and devices you \
                 own or are authorised to test; transmitting on these bands is \
                 regulated in most countries."
                    .into(),
            ),
        },
        BuiltinSource {
            id: "bruce".into(),
            name: "Bruce".into(),
            summary: "Multi-purpose offensive-security firmware for M5Stack and \
                      similar handhelds. Its releases are bare .bin files with no \
                      manifest, so the flash offsets and chip target cannot be \
                      determined safely — firmforge will not guess them."
                .into(),
            locator: Locator::Repo("BruceDevices/firmware".into()),
            tier: Tier::Suggested,
            availability: Availability::NoManifestYet,
            license: "AGPL-3.0".into(),
            families: vec!["ESP32".into(), "ESP32-S3".into()],
            homepage: "https://github.com/BruceDevices/firmware".into(),
            caution: Some(
                "Offensive-security tooling, and AGPL-3.0: if you ever rehost its \
                 binaries you inherit a source obligation. Only use it on networks \
                 and devices you own or are authorised to test."
                    .into(),
            ),
        },
    ]
}

/// The sources added automatically on first run.
pub fn bundled() -> Vec<BuiltinSource> {
    all()
        .into_iter()
        .filter(|s| s.tier == Tier::Bundled)
        .collect()
}

/// Sources offered but never added without the user asking.
pub fn suggested() -> Vec<BuiltinSource> {
    all()
        .into_iter()
        .filter(|s| s.tier == Tier::Suggested)
        .collect()
}

/// Look up a built-in by its stable id.
pub fn find(id: &str) -> Option<BuiltinSource> {
    all().into_iter().find(|s| s.id == id)
}

/// Built-ins relevant to a chip family, so the cold-start list is filtered by
/// the family the user picked rather than showing firmware that cannot fit.
pub fn for_family(family: &str) -> Vec<BuiltinSource> {
    all()
        .into_iter()
        .filter(|s| s.families.iter().any(|f| f.eq_ignore_ascii_case(family)))
        .collect()
}

impl BuiltinSource {
    /// What the user types, or what is fetched.
    pub fn target(&self) -> &str {
        match &self.locator {
            Locator::Repo(r) => r,
            Locator::ManifestUrl(u) => u,
        }
    }

    /// Whether the app should offer an "Add" button at all.
    pub fn can_add(&self) -> bool {
        self.availability == Availability::Ready
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cold_start_is_not_empty() {
        assert!(
            bundled().len() >= 3,
            "a fresh install must have something to show"
        );
    }

    /// The decision from `plan/spec/legal-and-licensing.md` §3, encoded so it
    /// cannot be undone by accident: nothing pre-added may be security tooling.
    #[test]
    fn no_offensive_security_firmware_is_pre_added() {
        for source in bundled() {
            assert!(
                source.caution.is_none(),
                "{} carries a caution, so it must be Suggested, not Bundled",
                source.name
            );
        }
    }

    #[test]
    fn every_suggested_source_explains_the_risk() {
        for source in all().into_iter().filter(|s| s.tier == Tier::Suggested) {
            assert!(
                source.caution.is_some(),
                "{} is suggested but says nothing about why it is opt-in",
                source.name
            );
        }
    }

    #[test]
    fn every_bundled_source_supports_the_default_family() {
        let default = crate::chip::default_family();
        for source in bundled() {
            assert!(
                source.families.contains(&default.id),
                "{} has no {} build, so it is useless on a fresh install",
                source.name,
                default.id
            );
        }
    }

    #[test]
    fn ids_are_unique_so_removals_can_be_remembered() {
        let mut ids: Vec<String> = all().into_iter().map(|s| s.id).collect();
        let total = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), total);
    }

    #[test]
    fn filtering_by_family_narrows_the_list() {
        assert!(!for_family("ESP32-S3").is_empty());
        assert!(for_family("nrf52").is_empty());
    }

    /// Everything added on first run must actually work on first run.
    #[test]
    fn nothing_bundled_is_known_to_be_uninstallable() {
        for source in bundled() {
            assert_eq!(source.availability, Availability::Ready, "{}", source.name);
            assert!(source.can_add());
        }
    }

    /// A source with no manifest has no trustworthy flash offsets, so it must
    /// never be presented as installable. Guessing offsets is how you brick a
    /// board.
    #[test]
    fn sources_without_a_manifest_cannot_be_added() {
        for source in all() {
            if source.availability == Availability::NoManifestYet {
                assert!(!source.can_add(), "{}", source.name);
                assert!(
                    !source.homepage.is_empty(),
                    "{} must at least link somewhere useful",
                    source.name
                );
            }
        }
    }
}
