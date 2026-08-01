//! Chip families, and which ones firmforge actually supports.
//!
//! The chip family is the top-level choice in the app, made before any firmware
//! is shown. That ordering is deliberate: everything downstream — which sources
//! are worth showing, which builds are installable, which flash offsets apply,
//! which transports exist — is a function of the chip. Asking first means the
//! rest of the app never has to hedge.
//!
//! Support is staged. ESP32-S3 is the family firmforge is built and tuned for
//! today; the others are listed so the roadmap is visible rather than hidden,
//! and so a user with an ESP32-C3 learns that immediately instead of after a
//! failed install.

use serde::{Deserialize, Serialize};

/// How far along support for a family is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SupportLevel {
    /// Built, tested and optimised for.
    Supported,
    /// Recognised and browsable, but installs are not enabled yet.
    Planned,
}

/// One selectable chip family.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChipFamily {
    /// The exact `chipFamily` string used in ESP Web Tools manifests. This is
    /// the join key against every manifest in the wild, so it must match
    /// byte-for-byte.
    pub id: String,
    pub display_name: String,
    /// Boards a user is likely to recognise, to make the choice concrete.
    pub example_boards: Vec<String>,
    /// Bootloader offset for this chip. 0x0 on the newer parts, 0x1000 on the
    /// original ESP32 and the S2 — the classic cause of bricked flashes.
    pub bootloader_offset: u32,
    pub support: SupportLevel,
    pub note: String,
}

impl ChipFamily {
    pub fn is_supported(&self) -> bool {
        self.support == SupportLevel::Supported
    }
}

/// Every family firmforge knows about, in the order they should be offered.
///
/// ESP32-S3 leads because it is the family that is actually supported. The rest
/// are ordered by how commonly they turn up in this ecosystem.
pub fn all() -> Vec<ChipFamily> {
    vec![
        family(
            "ESP32-S3",
            "ESP32-S3",
            &["M5Stack Cardputer", "LilyGO T-Deck", "M5StickC Plus 2"],
            0x0,
            SupportLevel::Supported,
            "Native USB, dual core, PSRAM. The family firmforge is built for.",
        ),
        family(
            "ESP32",
            "ESP32 (original)",
            &["ESP32 DevKit v1", "CYD 2432S028", "TTGO T-Display"],
            0x1000,
            SupportLevel::Planned,
            "Needs a USB-to-UART bridge, and its bootloader sits at 0x1000.",
        ),
        family(
            "ESP32-C3",
            "ESP32-C3",
            &["ESP32-C3 SuperMini", "XIAO ESP32C3"],
            0x0,
            SupportLevel::Planned,
            "RISC-V, single core, native USB.",
        ),
        family(
            "ESP32-C6",
            "ESP32-C6",
            &["M5NanoC6", "XIAO ESP32C6"],
            0x0,
            SupportLevel::Planned,
            "Adds 802.15.4 for Thread and Zigbee.",
        ),
        family(
            "ESP32-S2",
            "ESP32-S2",
            &["LOLIN S2 Mini"],
            0x1000,
            SupportLevel::Planned,
            "Single core, no Bluetooth.",
        ),
        family(
            "ESP32-H2",
            "ESP32-H2",
            &["ESP32-H2-DevKitM-1"],
            0x0,
            SupportLevel::Planned,
            "802.15.4 and Bluetooth LE, no WiFi.",
        ),
        family(
            "ESP8266",
            "ESP8266",
            &["Wemos D1 Mini", "NodeMCU"],
            0x0,
            SupportLevel::Planned,
            "The original. Still everywhere in home automation.",
        ),
    ]
}

/// Look up a family by its manifest `chipFamily` string.
pub fn find(id: &str) -> Option<ChipFamily> {
    all().into_iter().find(|f| f.id.eq_ignore_ascii_case(id))
}

/// The family selected by default on a fresh install.
pub fn default_family() -> ChipFamily {
    all()
        .into_iter()
        .find(ChipFamily::is_supported)
        .expect("at least one family must be supported")
}

fn family(
    id: &str,
    display_name: &str,
    boards: &[&str],
    bootloader_offset: u32,
    support: SupportLevel,
    note: &str,
) -> ChipFamily {
    ChipFamily {
        id: id.to_string(),
        display_name: display_name.to_string(),
        example_boards: boards.iter().map(|s| s.to_string()).collect(),
        bootloader_offset,
        support,
        note: note.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn esp32_s3_is_the_supported_default() {
        let d = default_family();
        assert_eq!(d.id, "ESP32-S3");
        assert!(d.is_supported());
    }

    #[test]
    fn exactly_one_family_is_supported_today() {
        let supported: Vec<_> = all().into_iter().filter(|f| f.is_supported()).collect();
        assert_eq!(
            supported.len(),
            1,
            "adding support is a deliberate act; update this test with it"
        );
    }

    /// These offsets are load-bearing. ESP32 and S2 boot at 0x1000; everything
    /// newer boots at 0x0. Verified against ESP32 Marauder's published
    /// installer manifest for both an S3 and an ESP32 target.
    #[test]
    fn bootloader_offsets_are_right_per_chip() {
        assert_eq!(find("ESP32-S3").unwrap().bootloader_offset, 0x0);
        assert_eq!(find("ESP32-C3").unwrap().bootloader_offset, 0x0);
        assert_eq!(find("ESP32").unwrap().bootloader_offset, 0x1000);
        assert_eq!(find("ESP32-S2").unwrap().bootloader_offset, 0x1000);
    }

    #[test]
    fn lookup_matches_manifest_spelling_case_insensitively() {
        assert!(find("esp32-s3").is_some());
        assert!(find("ESP32-S3").is_some());
        assert!(find("nrf52").is_none());
    }

    #[test]
    fn every_family_id_is_unique() {
        let mut ids: Vec<String> = all().into_iter().map(|f| f.id).collect();
        let total = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), total);
    }
}
