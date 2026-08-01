//! Running an install against a device, with progress reporting.
//!
//! Two backends share one event stream so the UI is identical either way:
//!
//! * **Demo** — writes nothing. Exists because verifying the app should not
//!   require owning an ESP32, and because rehearsing a flash is the safest way
//!   to check a manifest is correct before touching real hardware.
//! * **Serial** — the real thing, still to be wired to `espflash`.
//!
//! Events are emitted through a callback rather than returned, so the desktop
//! shell can forward them to the webview as they happen and the progress bar
//! moves during the write rather than after it.

use crate::install::Prepared;
use firmforge_core::device::{DeviceIdentity, SerialType};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Progress and log events, mirrored into the UI console.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum FlashEvent {
    Started {
        parts: usize,
        total_bytes: usize,
        demo: bool,
    },
    PartStarted {
        index: usize,
        parts: usize,
        offset: String,
        size_bytes: usize,
    },
    Progress {
        written_bytes: usize,
        total_bytes: usize,
        percent: u8,
    },
    Log {
        line: String,
    },
    Finished {
        total_bytes: usize,
        seconds: f32,
        demo: bool,
    },
    Failed {
        message: String,
    },
}

/// The device a build is being installed onto.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Target {
    pub port: String,
    pub identity: DeviceIdentity,
    /// True when this is the simulated device rather than real hardware.
    pub demo: bool,
}

/// The simulated device: an ESP32-S3 with 8 MB flash and native USB.
///
/// A real-ish identity matters — it means compatibility matching, incompatible
/// build reasons and part selection are all exercised for real in demo mode.
pub fn demo_target() -> Target {
    Target {
        port: "DEMO".to_string(),
        identity: DeviceIdentity {
            chip_family: "ESP32-S3".into(),
            chip_revision: Some(0),
            flash_size: Some(8 * 1024 * 1024),
            has_psram: true,
            serial_type: SerialType::Cdc,
            mac: Some("24:6F:28:00:00:01".into()),
        },
        demo: true,
    }
}

/// Simulate an install, emitting the same events a real flash would.
///
/// Paced at roughly 400 kB/s, which is what a 921600-baud write with
/// compression actually achieves, so the progress bar behaves realistically.
pub async fn run_demo<F>(prepared: &Prepared, emit: F)
where
    F: Fn(FlashEvent),
{
    let total = prepared.total_bytes();
    let started = std::time::Instant::now();

    emit(FlashEvent::Started {
        parts: prepared.parts.len(),
        total_bytes: total,
        demo: true,
    });
    emit(FlashEvent::Log {
        line: "Demo device — nothing will be written to any hardware.".into(),
    });
    emit(FlashEvent::Log {
        line: "Connecting....... Chip is ESP32-S3 (revision v0.0)".into(),
    });
    emit(FlashEvent::Log {
        line: "Features: WiFi, BLE, 8MB flash, PSRAM".into(),
    });

    let mut written = 0usize;

    for (index, part) in prepared.parts.iter().enumerate() {
        emit(FlashEvent::PartStarted {
            index: index + 1,
            parts: prepared.parts.len(),
            offset: format!("0x{:X}", part.offset),
            size_bytes: part.bytes.len(),
        });
        emit(FlashEvent::Log {
            line: format!(
                "Writing {} bytes at 0x{:X}{}",
                part.bytes.len(),
                part.offset,
                if part.verified {
                    " (SHA-256 verified)"
                } else {
                    " (no checksum in manifest)"
                }
            ),
        });

        // ~64 kB chunks, the erase block granularity esptool uses.
        let chunk = 65_536usize;
        let mut done_in_part = 0usize;
        while done_in_part < part.bytes.len() {
            let step = chunk.min(part.bytes.len() - done_in_part);
            done_in_part += step;
            written += step;

            emit(FlashEvent::Progress {
                written_bytes: written,
                total_bytes: total,
                percent: percent(written, total),
            });
            tokio::time::sleep(Duration::from_millis(160)).await;
        }
    }

    emit(FlashEvent::Log {
        line: "Hash of data verified.".into(),
    });
    emit(FlashEvent::Log {
        line: "Leaving... Hard resetting via RTS pin...".into(),
    });
    emit(FlashEvent::Finished {
        total_bytes: total,
        seconds: started.elapsed().as_secs_f32(),
        demo: true,
    });
}

fn percent(written: usize, total: usize) -> u8 {
    if total == 0 {
        return 100;
    }
    ((written as f64 / total as f64) * 100.0).round().min(100.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_demo_device_is_a_realistic_target() {
        let t = demo_target();
        assert!(t.demo);
        assert_eq!(t.identity.chip_family, "ESP32-S3");
        assert_eq!(t.identity.flash_size, Some(8 * 1024 * 1024));
    }

    #[test]
    fn percentages_are_bounded() {
        assert_eq!(percent(0, 100), 0);
        assert_eq!(percent(50, 100), 50);
        assert_eq!(percent(100, 100), 100);
        assert_eq!(percent(10, 0), 100, "an empty install is complete");
        assert_eq!(percent(200, 100), 100, "never exceeds 100");
    }
}
