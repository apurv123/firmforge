//! Integration test: the repository's own `firmware/manifest.json` must be a
//! valid firmforge manifest, and must exercise the features the convention
//! documents. This keeps the worked example and the parser honest about each
//! other.

use firmforge_core::device::{DeviceIdentity, SerialType};
use firmforge_core::manifest::Manifest;
use firmforge_core::matching;

fn load() -> Manifest {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../firmware/manifest.json");
    let bytes = std::fs::read(path).expect("firmware/manifest.json must exist");
    Manifest::from_json(&bytes).expect("firmware/manifest.json must parse")
}

#[test]
fn repository_manifest_parses_and_uses_the_convention() {
    let m = load();
    assert_eq!(m.version, "0.1.0");
    assert_eq!(m.builds.len(), 3);

    let s3_cdc = &m.builds[0];
    assert_eq!(s3_cdc.serial_type.as_deref(), Some("cdc"));
    assert_eq!(s3_cdc.constraints.min_flash_size, Some(8 * 1024 * 1024));
    assert!(s3_cdc.parts.iter().all(|p| p.sha256.is_some()));
    assert_eq!(s3_cdc.assets.len(), 1);

    // Bruce-style capability-differentiated variant.
    let lite = &m.builds[1];
    assert_eq!(lite.variant.as_deref(), Some("LITE_VERSION"));
    assert!(lite.variant_omits.contains(&"interpreter".to_string()));

    // The C3 bootloader sits at offset 0, unlike the S3 builds at 0x1000.
    let c3 = &m.builds[2];
    assert_eq!(c3.parts[0].offset, 0);
    assert_eq!(m.builds[0].parts[0].offset, 0x1000);
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
fn a_c3_board_selects_the_any_connection_fallback_build() {
    let m = load();
    let device = DeviceIdentity {
        chip_family: "ESP32-C3".into(),
        chip_revision: Some(0),
        flash_size: Some(4 * 1024 * 1024),
        has_psram: false,
        serial_type: SerialType::Uart,
        mac: None,
    };

    let chosen = matching::select_best(&m.builds, &device).expect("a C3 build must be selectable");
    assert_eq!(chosen.chip_family, "ESP32-C3");
    assert!(chosen.serial_type.is_none());
}
