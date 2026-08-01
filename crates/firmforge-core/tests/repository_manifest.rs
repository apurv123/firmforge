//! Integration test: the repository's own `firmware/manifest.json` must be a
//! valid firmforge manifest, and must exercise the features the convention
//! documents. This keeps the worked example and the parser honest about each
//! other.
//!
//! The example deliberately describes **real, published firmware** (ESP32
//! Marauder v1.14.1) using that project's own authoritative offsets and
//! digests, so these assertions are checked against reality rather than
//! against numbers invented to make a test pass.

use firmforge_core::device::{DeviceIdentity, SerialType};
use firmforge_core::manifest::Manifest;
use firmforge_core::matching;
use firmforge_core::source::{resolve_all, Origin};

const MANIFEST_URL: &str =
    "https://raw.githubusercontent.com/apurv123/firmforge/main/firmware/manifest.json";

fn load() -> Manifest {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../firmware/manifest.json");
    let bytes = std::fs::read(path).expect("firmware/manifest.json must exist");
    Manifest::from_json(&bytes).expect("firmware/manifest.json must parse")
}

#[test]
fn repository_manifest_parses_and_uses_the_convention() {
    let m = load();
    assert_eq!(m.version, "v1.14.1");
    assert_eq!(m.builds.len(), 2);

    for build in &m.builds {
        assert_eq!(build.parts.len(), 4, "bootloader, table, ota-data, app");
        assert!(
            build.parts.iter().all(|p| p.sha256.is_some()),
            "every part must carry a digest so it can be verified before writing"
        );
    }
}

/// The licensing invariant from `plan/spec/legal-and-licensing.md` §2: the
/// example must *reference* upstream release assets, never rehost them. If
/// someone later adds a mirrored binary, this fails loudly.
#[test]
fn every_part_is_referenced_upstream_not_rehosted() {
    let m = load();
    for build in &m.builds {
        for part in resolve_all(&build.parts, MANIFEST_URL).expect("parts must resolve") {
            assert_eq!(
                part.origin,
                Origin::Upstream,
                "{} is served from our own repository — that is redistribution",
                part.url
            );
        }
    }
}

/// Bootloader offset genuinely differs by chip: 0x0 on ESP32-S3, 0x1000 on the
/// original ESP32. Getting this wrong is the classic hand-rolled esptool bug,
/// so the worked example pins both.
#[test]
fn bootloader_offsets_match_the_chip() {
    let m = load();

    let s3 = m
        .builds
        .iter()
        .find(|b| b.chip_family == "ESP32-S3")
        .unwrap();
    assert_eq!(s3.parts[0].offset, 0x0);

    let esp32 = m.builds.iter().find(|b| b.chip_family == "ESP32").unwrap();
    assert_eq!(esp32.parts[0].offset, 0x1000);

    // The application offset is the same on both.
    assert_eq!(s3.parts[3].offset, 0x10000);
    assert_eq!(esp32.parts[3].offset, 0x10000);
}

#[test]
fn a_4mb_s3_board_is_told_why_the_first_build_does_not_fit() {
    let m = load();
    let device = DeviceIdentity {
        chip_family: "ESP32-S3".into(),
        chip_revision: Some(0),
        flash_size: Some(4 * 1024 * 1024),
        has_psram: false,
        serial_type: SerialType::Cdc,
        mac: None,
    };

    let verdict = matching::evaluate(&m.builds[0], &device);
    assert!(!verdict.compatible);
    assert_eq!(
        verdict.reasons[0].message(),
        "needs 8 MB flash, yours has 4 MB"
    );
}

#[test]
fn an_esp32_board_selects_the_esp32_build_not_the_s3_one() {
    let m = load();
    let device = DeviceIdentity {
        chip_family: "ESP32".into(),
        chip_revision: Some(1),
        flash_size: Some(4 * 1024 * 1024),
        has_psram: false,
        serial_type: SerialType::Uart,
        mac: None,
    };

    let chosen =
        matching::select_best(&m.builds, &device).expect("an ESP32 build must be selectable");
    assert_eq!(chosen.chip_family, "ESP32");
}

/// The demo device the app ships with must be able to install the example, or
/// the out-of-the-box walkthrough is broken.
#[test]
fn the_demo_device_can_install_the_example() {
    let m = load();
    let demo = DeviceIdentity {
        chip_family: "ESP32-S3".into(),
        chip_revision: Some(0),
        flash_size: Some(8 * 1024 * 1024),
        has_psram: true,
        serial_type: SerialType::Cdc,
        mac: None,
    };

    let chosen =
        matching::select_best(&m.builds, &demo).expect("the demo device must have a build");
    assert_eq!(chosen.chip_family, "ESP32-S3");
}
