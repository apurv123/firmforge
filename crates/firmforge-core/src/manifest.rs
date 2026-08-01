//! The firmforge firmware manifest.
//!
//! Deliberately a **superset of the ESP Web Tools `manifest.json`** so that:
//!   * firmforge can install any firmware that already ships an ESP Web Tools
//!     manifest (Bruce, ESPHome, and the vendor long tail), and
//!   * any firmware published for firmforge also works in a browser flasher.
//!
//! All firmforge extensions are optional and are ignored by ESP Web Tools.
//! See `plan/spec/product-research/esp-web-tools/README.md` for the analysis
//! that motivated this decision.

use serde::{Deserialize, Serialize};

/// Release channel. Mirrors Bruce's `Latest / Beta / Other` selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    #[default]
    Stable,
    Beta,
    Nightly,
}

/// One flashable part: a binary written at an absolute flash offset.
///
/// Firmware is explicitly multi-part. Note that the bootloader offset differs
/// by chip family (`0x0` on C3/C6/H2 versus `0x1000` on ESP32/S2/S3), which is
/// the single most common cause of hand-rolled `esptool` failures.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Part {
    pub path: String,
    pub offset: u32,
    /// firmforge extension: lowercase hex SHA-256 of the part.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
}

/// A non-binary payload shipped with a build (SD card / LittleFS assets).
///
/// Exists because Bruce's App Store currently requires users to physically
/// remove the SD card and copy files into `/Bruce/` by hand.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Asset {
    pub path: String,
    /// Destination on the device, e.g. `/Bruce/themes/dark.bin`.
    pub target: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
}

/// Constraints a device must satisfy for a build to be installable.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Constraints {
    /// Minimum flash size in bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_flash_size: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub psram_required: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_chip_revision: Option<u32>,
}

/// One installable variant of a release.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Build {
    /// ESP Web Tools field, e.g. `ESP32`, `ESP32-C3`, `ESP32-S3`, `ESP8266`.
    pub chip_family: String,

    /// ESP Web Tools field: `cdc` (native USB) or `uart` (USB-to-UART bridge).
    /// Absent means "fallback for any connection type" — copy those semantics
    /// exactly, they are load-bearing for ESP32-S3 boards.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub serial_type: Option<String>,

    pub parts: Vec<Part>,

    // ---- firmforge extensions ----
    /// Named variant, e.g. Bruce's `LITE_VERSION`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,

    /// What this variant gives up, shown to the user before they choose it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variant_omits: Vec<String>,

    /// Capability tags used as catalogue facets: `wifi`, `ble`, `subghz`,
    /// `nfc`, `ir`, `lora`, ...
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub capabilities: Vec<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assets: Vec<Asset>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub board_ids: Vec<String>,

    #[serde(default)]
    pub constraints: Constraints,

    /// Whether this build can subsequently be updated over the air, and how.
    /// Determines which transports the app may offer (the only path on iOS).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ota_protocol: Option<String>,
}

/// A parsed manifest describing one firmware release.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub name: String,
    pub version: String,
    pub builds: Vec<Build>,

    // ---- ESP Web Tools optional metadata ----
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub home_assistant_domain: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub funding_url: Option<String>,

    // ---- firmforge extensions ----
    #[serde(default)]
    pub channel: Channel,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_notes_url: Option<String>,
    /// Detached Ed25519 signature over the canonical manifest bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

impl Manifest {
    pub fn from_json(bytes: &[u8]) -> crate::Result<Self> {
        Ok(serde_json::from_slice(bytes)?)
    }

    pub fn to_json(&self) -> crate::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The canonical ESP Web Tools manifest from esphome/esp-web-tools must
    /// parse unchanged — that compatibility is the whole interop bet.
    const ESP_WEB_TOOLS: &str = r#"{
      "name": "ESPHome",
      "version": "2021.10.3",
      "home_assistant_domain": "esphome",
      "funding_url": "https://esphome.io/guides/supporters.html",
      "builds": [
        {
          "chipFamily": "ESP32",
          "parts": [
            { "path": "bootloader_dout_40m.bin", "offset": 4096 },
            { "path": "partitions.bin", "offset": 32768 },
            { "path": "boot_app0.bin", "offset": 57344 },
            { "path": "esp32.bin", "offset": 65536 }
          ]
        },
        {
          "chipFamily": "ESP32-S3",
          "serialType": "cdc",
          "parts": [ { "path": "esp32-s3-cdc.bin", "offset": 65536 } ]
        },
        {
          "chipFamily": "ESP8266",
          "parts": [ { "path": "esp8266.bin", "offset": 0 } ]
        }
      ]
    }"#;

    #[test]
    fn parses_upstream_esp_web_tools_manifest() {
        let m = Manifest::from_json(ESP_WEB_TOOLS.as_bytes()).expect("must parse");
        assert_eq!(m.name, "ESPHome");
        assert_eq!(m.builds.len(), 3);
        assert_eq!(m.builds[0].chip_family, "ESP32");
        assert_eq!(m.builds[0].parts[0].offset, 4096);
        assert_eq!(m.builds[1].serial_type.as_deref(), Some("cdc"));
        // No serialType on the ESP8266 build => any-connection fallback.
        assert!(m.builds[2].serial_type.is_none());
        // Extensions default cleanly when absent.
        assert_eq!(m.channel, Channel::Stable);
    }

    #[test]
    fn roundtrips_with_extensions() {
        let m = Manifest {
            name: "Bruce".into(),
            version: "1.16".into(),
            channel: Channel::Beta,
            release_notes_url: Some("https://example.invalid/notes".into()),
            signature: None,
            home_assistant_domain: None,
            funding_url: None,
            builds: vec![Build {
                chip_family: "ESP32-S3".into(),
                serial_type: Some("uart".into()),
                parts: vec![Part {
                    path: "app.bin".into(),
                    offset: 0x10000,
                    sha256: Some("ab".repeat(32)),
                }],
                variant: Some("LITE_VERSION".into()),
                variant_omits: vec!["ssh".into(), "wireguard".into(), "interpreter".into()],
                capabilities: vec!["wifi".into(), "ble".into()],
                assets: vec![],
                board_ids: vec!["m5stack-cardputer".into()],
                constraints: Constraints {
                    min_flash_size: Some(8 * 1024 * 1024),
                    ..Default::default()
                },
                ota_protocol: None,
            }],
        };
        let json = m.to_json().unwrap();
        let back = Manifest::from_json(json.as_bytes()).unwrap();
        assert_eq!(m, back);
    }
}
