//! Running an install against a device, with progress reporting.
//!
//! Two backends share one event stream so the UI is identical either way:
//!
//! * **Demo** — writes nothing. Exists because verifying the app should not
//!   require owning an ESP32, and because rehearsing a flash is the safest way
//!   to check a manifest is correct before touching real hardware.
//! * **Serial** — the real thing, writing to an ESP32 over USB via `espflash`.
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

/// Translate `espflash`'s chunk counting into the byte-based progress the UI
/// already understands.
///
/// `espflash` reports "chunk 7 of 40" per part and nothing about bytes, while
/// every other part of firmforge speaks in bytes. This holds the part sizes so
/// the two can be reconciled without the UI learning a second vocabulary.
struct ByteProgress {
    /// `(offset, size)` for each part, in the order they will be written.
    sizes: Vec<(u32, usize)>,
    total: usize,
    completed_bytes: usize,
    current_size: usize,
    current_chunks: usize,
}

impl ByteProgress {
    fn new(parts: &[(u32, Vec<u8>)]) -> Self {
        let sizes: Vec<(u32, usize)> = parts.iter().map(|(o, b)| (*o, b.len())).collect();
        let total = sizes.iter().map(|(_, s)| *s).sum();
        Self {
            sizes,
            total,
            completed_bytes: 0,
            current_size: 0,
            current_chunks: 0,
        }
    }

    fn start_part(&mut self, offset: u32, chunks: usize) {
        self.current_size = self
            .sizes
            .iter()
            .find(|(o, _)| *o == offset)
            .map(|(_, s)| *s)
            .unwrap_or(0);
        self.current_chunks = chunks;
    }

    /// Bytes written overall, given `chunks_done` of the current part.
    fn written(&self, chunks_done: usize) -> usize {
        if self.current_chunks == 0 {
            return self.completed_bytes;
        }
        let fraction = (chunks_done.min(self.current_chunks) as f64) / (self.current_chunks as f64);
        self.completed_bytes + (self.current_size as f64 * fraction).round() as usize
    }

    fn finish_part(&mut self) {
        self.completed_bytes += self.current_size;
        self.current_size = 0;
        self.current_chunks = 0;
    }
}

/// Write a prepared build to a real device.
///
/// `espflash` is entirely blocking, so the write runs on a blocking thread and
/// reports back over a channel. That keeps the webview responsive and, more
/// importantly, keeps progress flowing during the write instead of arriving in
/// one lump at the end — a progress bar that stalls for ninety seconds is how
/// users decide a flash has hung and unplug the board mid-write.
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub async fn run_serial<F>(
    port: &str,
    chip_family: &str,
    prepared: &Prepared,
    emit: F,
) -> Result<(), String>
where
    F: Fn(FlashEvent),
{
    use firmforge_flash::esp::{self, WriteEvent};

    let parts: Vec<(u32, Vec<u8>)> = prepared
        .parts
        .iter()
        .map(|p| (p.offset, p.bytes.clone()))
        .collect();
    let part_count = parts.len();
    let mut progress = ByteProgress::new(&parts);
    let total = progress.total;
    let started = std::time::Instant::now();

    emit(FlashEvent::Started {
        parts: part_count,
        total_bytes: total,
        demo: false,
    });

    for (index, part) in prepared.parts.iter().enumerate() {
        if !part.verified {
            emit(FlashEvent::Log {
                line: format!(
                    "Part {} of {} at 0x{:X} has no checksum in the manifest — \
                     firmforge could not confirm the download is intact.",
                    index + 1,
                    part_count,
                    part.offset
                ),
            });
        }
    }

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<WriteEvent>();

    let port_owned = port.to_string();
    let family_owned = chip_family.to_string();
    let write_parts = parts.clone();
    let handle = tokio::task::spawn_blocking(move || {
        let mut send = |event: WriteEvent| {
            let _ = tx.send(event);
        };
        esp::write_parts(&port_owned, &family_owned, &write_parts, &mut send)
    });

    let mut part_index = 0usize;

    while let Some(event) = rx.recv().await {
        match event {
            WriteEvent::Connected { identity } => {
                emit(FlashEvent::Log {
                    line: format!(
                        "Connected. Chip is {}{}, {} flash{}{}.",
                        identity.chip_family,
                        identity
                            .chip_revision
                            .map(|r| format!(" (revision v{r})"))
                            .unwrap_or_default(),
                        identity
                            .flash_size
                            .map(|b| format!("{} MB", b / (1024 * 1024)))
                            .unwrap_or_else(|| "unknown".into()),
                        if identity.has_psram { ", PSRAM" } else { "" },
                        identity
                            .mac
                            .as_ref()
                            .map(|m| format!(", MAC {m}"))
                            .unwrap_or_default(),
                    ),
                });
            }
            WriteEvent::PartStarted { offset, chunks } => {
                progress.start_part(offset, chunks);
                part_index += 1;
                emit(FlashEvent::PartStarted {
                    index: part_index,
                    parts: part_count,
                    offset: format!("0x{offset:X}"),
                    size_bytes: progress.current_size,
                });
            }
            WriteEvent::PartProgress { chunks_done } => {
                let written = progress.written(chunks_done);
                emit(FlashEvent::Progress {
                    written_bytes: written,
                    total_bytes: total,
                    percent: percent(written, total),
                });
            }
            WriteEvent::Verifying => {
                emit(FlashEvent::Log {
                    line: "Reading the region back to check it landed correctly…".into(),
                });
            }
            WriteEvent::PartFinished { skipped } => {
                progress.finish_part();
                emit(FlashEvent::Progress {
                    written_bytes: progress.completed_bytes,
                    total_bytes: total,
                    percent: percent(progress.completed_bytes, total),
                });
                if skipped {
                    emit(FlashEvent::Log {
                        line: "Already up to date — this region was left alone.".into(),
                    });
                }
            }
            WriteEvent::Log { line } => emit(FlashEvent::Log { line }),
        }
    }

    let outcome = handle
        .await
        .map_err(|e| format!("the flashing thread stopped unexpectedly: {e}"))?;

    match outcome {
        Ok(()) => {
            emit(FlashEvent::Finished {
                total_bytes: total,
                seconds: started.elapsed().as_secs_f32(),
                demo: false,
            });
            Ok(())
        }
        Err(e) => {
            let message = e.to_string();
            emit(FlashEvent::Failed {
                message: message.clone(),
            });
            Err(message)
        }
    }
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

    /// espflash counts chunks, the UI counts bytes, and the parts are wildly
    /// different sizes. Weighting by chunk count alone would make a 3 kB
    /// bootloader look like as much work as a 1.5 MB app image.
    #[test]
    fn chunk_progress_is_weighted_by_real_part_size() {
        let parts = vec![(0x0u32, vec![0u8; 1_000]), (0x10_000u32, vec![0u8; 9_000])];
        let mut p = ByteProgress::new(&parts);
        assert_eq!(p.total, 10_000);

        p.start_part(0x0, 10);
        assert_eq!(p.written(5), 500);
        p.finish_part();
        assert_eq!(p.completed_bytes, 1_000);

        p.start_part(0x10_000, 90);
        assert_eq!(p.written(45), 1_000 + 4_500);
        p.finish_part();
        assert_eq!(p.completed_bytes, 10_000);
    }

    /// espflash may report an offset we did not stage, and a part may be
    /// reported with zero chunks. Neither should panic or divide by zero.
    #[test]
    fn unknown_offsets_and_empty_parts_do_not_break_progress() {
        let parts = vec![(0x0u32, vec![0u8; 100])];
        let mut p = ByteProgress::new(&parts);

        p.start_part(0xDEAD, 4);
        assert_eq!(p.written(2), 0, "an unknown offset contributes nothing");

        p.start_part(0x0, 0);
        assert_eq!(p.written(7), 0, "no chunks means no division");
    }

    /// Overshooting the reported chunk count must not report more bytes than
    /// the part actually contains.
    #[test]
    fn progress_never_runs_past_the_part() {
        let parts = vec![(0x0u32, vec![0u8; 500])];
        let mut p = ByteProgress::new(&parts);
        p.start_part(0x0, 5);
        assert_eq!(p.written(99), 500);
    }
}
