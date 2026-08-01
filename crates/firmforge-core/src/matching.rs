//! Compatibility matching: given a detected device, decide which builds are
//! installable — and, when they are not, say exactly why.
//!
//! Never hide an incompatible build. A dimmed card that says "needs 16 MB
//! flash, yours is 4 MB" teaches the user something; a missing card confuses
//! them. See spec §9.3 principle 6.

use crate::device::DeviceIdentity;
use crate::manifest::Build;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind", content = "detail")]
pub enum Incompatibility {
    ChipFamily { required: String, found: String },
    SerialType { required: String, found: String },
    FlashTooSmall { required: u64, found: u64 },
    PsramRequired,
    ChipRevisionTooOld { required: u32, found: u32 },
}

impl Incompatibility {
    /// Plain-language reason, rendered directly on the catalogue card.
    pub fn message(&self) -> String {
        match self {
            Incompatibility::ChipFamily { required, found } => {
                format!("built for {required}, your device is {found}")
            }
            Incompatibility::SerialType { required, found } => {
                format!("needs a {required} connection, yours is {found}")
            }
            Incompatibility::FlashTooSmall { required, found } => format!(
                "needs {} MB flash, yours has {} MB",
                required / 1_048_576,
                found / 1_048_576
            ),
            Incompatibility::PsramRequired => "requires PSRAM, which this device lacks".into(),
            Incompatibility::ChipRevisionTooOld { required, found } => {
                format!("needs chip revision {required} or newer, yours is {found}")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Compatibility {
    pub compatible: bool,
    pub reasons: Vec<Incompatibility>,
}

impl Compatibility {
    pub fn ok() -> Self {
        Self {
            compatible: true,
            reasons: Vec::new(),
        }
    }
}

/// Evaluate one build against one detected device.
pub fn evaluate(build: &Build, device: &DeviceIdentity) -> Compatibility {
    let mut reasons = Vec::new();

    if !build.chip_family.eq_ignore_ascii_case(&device.chip_family) {
        reasons.push(Incompatibility::ChipFamily {
            required: build.chip_family.clone(),
            found: device.chip_family.clone(),
        });
    }

    // An absent serialType is a deliberate fallback for any connection type.
    if let Some(required) = build.serial_type.as_deref() {
        let found = device.serial_type.as_str();
        if !required.eq_ignore_ascii_case(found) {
            reasons.push(Incompatibility::SerialType {
                required: required.to_string(),
                found: found.to_string(),
            });
        }
    }

    if let (Some(required), Some(found)) = (build.constraints.min_flash_size, device.flash_size) {
        if found < required {
            reasons.push(Incompatibility::FlashTooSmall { required, found });
        }
    }

    if build.constraints.psram_required.unwrap_or(false) && !device.has_psram {
        reasons.push(Incompatibility::PsramRequired);
    }

    if let (Some(required), Some(found)) =
        (build.constraints.min_chip_revision, device.chip_revision)
    {
        if found < required {
            reasons.push(Incompatibility::ChipRevisionTooOld { required, found });
        }
    }

    Compatibility {
        compatible: reasons.is_empty(),
        reasons,
    }
}

/// Pick the best build for a device, preferring an exact `serialType` match
/// over the any-connection fallback.
pub fn select_best<'a>(builds: &'a [Build], device: &DeviceIdentity) -> Option<&'a Build> {
    let mut candidates: Vec<&Build> = builds
        .iter()
        .filter(|b| evaluate(b, device).compatible)
        .collect();

    candidates.sort_by_key(|b| if b.serial_type.is_some() { 0 } else { 1 });
    candidates.into_iter().next()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::SerialType;
    use crate::manifest::{Constraints, Part};

    fn device() -> DeviceIdentity {
        DeviceIdentity {
            chip_family: "ESP32-S3".into(),
            chip_revision: Some(1),
            flash_size: Some(8 * 1024 * 1024),
            has_psram: false,
            serial_type: SerialType::Cdc,
            mac: None,
        }
    }

    fn build(chip: &str, serial: Option<&str>) -> Build {
        Build {
            chip_family: chip.into(),
            serial_type: serial.map(str::to_string),
            parts: vec![Part {
                path: "app.bin".into(),
                offset: 0x10000,
                sha256: None,
            }],
            variant: None,
            variant_omits: vec![],
            capabilities: vec![],
            assets: vec![],
            board_ids: vec![],
            constraints: Constraints::default(),
            ota_protocol: None,
        }
    }

    #[test]
    fn rejects_wrong_chip_family_with_a_readable_reason() {
        let c = evaluate(&build("ESP32-C3", None), &device());
        assert!(!c.compatible);
        assert!(c.reasons[0].message().contains("ESP32-C3"));
    }

    #[test]
    fn absent_serial_type_is_a_fallback_for_any_connection() {
        assert!(evaluate(&build("ESP32-S3", None), &device()).compatible);
    }

    #[test]
    fn rejects_mismatched_serial_type() {
        assert!(!evaluate(&build("ESP32-S3", Some("uart")), &device()).compatible);
    }

    #[test]
    fn prefers_an_exact_serial_type_match_over_the_fallback() {
        let builds = vec![build("ESP32-S3", None), build("ESP32-S3", Some("cdc"))];
        let chosen = select_best(&builds, &device()).unwrap();
        assert_eq!(chosen.serial_type.as_deref(), Some("cdc"));
    }

    #[test]
    fn reports_insufficient_flash_in_megabytes() {
        let mut b = build("ESP32-S3", None);
        b.constraints.min_flash_size = Some(16 * 1024 * 1024);
        let c = evaluate(&b, &device());
        assert!(!c.compatible);
        assert_eq!(c.reasons[0].message(), "needs 16 MB flash, yours has 8 MB");
    }
}
