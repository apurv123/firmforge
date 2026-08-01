//! The shared application surface.
//!
//! Everything the UI can ask for lives here, so the desktop and mobile shells
//! stay thin: each simply re-exports these functions as Tauri commands. Keeping
//! this crate free of `tauri` also means it can back a headless CLI (spec
//! F-32) without change.

pub mod flash;
pub mod github;
pub mod install;

use firmforge_core::{
    device::DeviceIdentity,
    manifest::{Build, Manifest},
    matching::{self, Compatibility},
};
use serde::{Deserialize, Serialize};

pub use flash::{demo_target, FlashEvent, Target};
pub use github::{discover, DiscoveredManifest, Source};
pub use install::{prepare, PartSummary, Prepared};

/// A catalogue entry as rendered on a firmforge card (spec screen D3).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogueEntry {
    pub name: String,
    pub version: String,
    pub channel: String,
    pub variant: Option<String>,
    pub capabilities: Vec<String>,
    pub compatibility: Compatibility,
    /// Plain-language reasons shown on a dimmed card. Never hide the card.
    pub reasons: Vec<String>,
    pub verified: bool,

    // ---- provenance and addressing, so a card can be acted on ----
    /// `owner/repo` this came from, empty for a hand-pasted manifest.
    #[serde(default)]
    pub source: String,
    /// Where the manifest was fetched from; the base for relative part paths.
    #[serde(default)]
    pub manifest_url: String,
    /// e.g. `main branch` or `release v1.16`.
    #[serde(default)]
    pub provenance: String,
    /// Index into `manifest.builds`, so the shell can name this exact build.
    pub build_index: usize,
    pub chip_family: String,
    pub part_count: usize,
}

/// Parse a manifest and evaluate every build against a detected device.
///
/// Incompatible builds are returned too, each carrying a readable reason —
/// spec §9.3 principle 6.
pub fn build_catalogue(
    manifest: &Manifest,
    device: Option<&DeviceIdentity>,
) -> Vec<CatalogueEntry> {
    manifest
        .builds
        .iter()
        .enumerate()
        .map(|(index, build)| entry_for(manifest, build, index, device))
        .collect()
}

/// Build catalogue entries for everything discovered in a repository, carrying
/// the provenance needed to download and flash each one.
pub fn catalogue_from_sources(
    discovered: &[DiscoveredManifest],
    device: Option<&DeviceIdentity>,
) -> Vec<CatalogueEntry> {
    discovered
        .iter()
        .flat_map(|found| {
            build_catalogue(&found.manifest, device)
                .into_iter()
                .map(move |mut entry| {
                    entry.source = found.source.clone();
                    entry.manifest_url = found.manifest_url.clone();
                    entry.provenance = found.provenance.clone();
                    entry
                })
        })
        .collect()
}

fn entry_for(
    manifest: &Manifest,
    build: &Build,
    build_index: usize,
    device: Option<&DeviceIdentity>,
) -> CatalogueEntry {
    let compatibility = match device {
        Some(d) => matching::evaluate(build, d),
        None => Compatibility::ok(),
    };
    let reasons = compatibility.reasons.iter().map(|r| r.message()).collect();

    CatalogueEntry {
        name: manifest.name.clone(),
        version: manifest.version.clone(),
        channel: serde_json::to_value(manifest.channel)
            .ok()
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_else(|| "stable".into()),
        variant: build.variant.clone(),
        capabilities: build.capabilities.clone(),
        compatibility,
        reasons,
        verified: manifest.signature.is_some(),
        source: String::new(),
        manifest_url: String::new(),
        provenance: String::new(),
        build_index,
        chip_family: build.chip_family.clone(),
        part_count: build.parts.len(),
    }
}

/// Parse manifest JSON coming from a GitHub source.
pub fn parse_manifest(json: &[u8]) -> firmforge_core::Result<Manifest> {
    Manifest::from_json(json)
}

/// Enumerate connectable serial ports (desktop only; returns empty on mobile,
/// where USB access goes through the platform plugin instead).
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub fn list_serial_ports() -> Vec<firmforge_flash::serial::SerialPortInfo> {
    firmforge_flash::serial::list_ports().unwrap_or_default()
}

#[cfg(any(target_os = "android", target_os = "ios"))]
pub fn list_serial_ports() -> Vec<serde_json::Value> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use firmforge_core::device::SerialType;

    const MANIFEST: &str = r#"{
      "name": "Bruce",
      "version": "1.16",
      "channel": "beta",
      "builds": [
        { "chipFamily": "ESP32-S3", "serialType": "cdc",
          "parts": [{ "path": "app.bin", "offset": 65536 }],
          "capabilities": ["wifi", "ble", "subghz"] },
        { "chipFamily": "ESP32-C3",
          "parts": [{ "path": "app.bin", "offset": 0 }] }
      ]
    }"#;

    fn s3_device() -> DeviceIdentity {
        DeviceIdentity {
            chip_family: "ESP32-S3".into(),
            chip_revision: Some(0),
            flash_size: Some(8 * 1024 * 1024),
            has_psram: false,
            serial_type: SerialType::Cdc,
            mac: None,
        }
    }

    #[test]
    fn catalogue_marks_incompatible_builds_without_hiding_them() {
        let m = parse_manifest(MANIFEST.as_bytes()).unwrap();
        let entries = build_catalogue(&m, Some(&s3_device()));

        assert_eq!(entries.len(), 2, "incompatible builds must still be listed");
        assert!(entries[0].compatibility.compatible);
        assert_eq!(entries[0].channel, "beta");
        assert!(entries[0].capabilities.contains(&"subghz".to_string()));

        assert!(!entries[1].compatibility.compatible);
        assert!(entries[1].reasons[0].contains("ESP32-C3"));
    }

    #[test]
    fn unsigned_manifests_are_not_marked_verified() {
        let m = parse_manifest(MANIFEST.as_bytes()).unwrap();
        assert!(!build_catalogue(&m, None)[0].verified);
    }

    #[test]
    fn port_enumeration_is_safe_to_call() {
        let _ = list_serial_ports();
    }
}
