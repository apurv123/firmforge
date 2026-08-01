//! Writing firmware to a real ESP32 over USB serial, via `espflash`.
//!
//! This is the part of firmforge that can destroy something. Everything here is
//! arranged around a single principle: **refuse rather than guess.** A confused
//! flash is not a failed operation you retry, it is a board that no longer
//! boots and, for some users, no longer enumerates over USB at all.
//!
//! Four gates stand between a download and a write:
//!
//! 1. The manifest's `chipFamily` must map to a chip `espflash` understands.
//!    ESP8266 does not, and is refused rather than approximated.
//! 2. That chip is handed to `Flasher::connect`, which reads the chip's magic
//!    number and refuses the connection if it disagrees. This is a check
//!    against the silicon, not against a filename, so it catches the classic
//!    "flashed the ESP32 build onto an S3" mistake at the protocol level.
//! 3. Every part is padded to a 4-byte boundary before it is offered.
//!    `write_bins_to_flash` does no padding of its own, and the MD5 check it
//!    performs afterwards uses the unpadded length.
//! 4. `verify = true`, so `espflash` reads back an MD5 of each region after
//!    writing it and fails loudly if what landed is not what was sent.
//!
//! Not compiled on Android or iOS, where a host serial port cannot be opened.

use crate::bridge::{identify_bridge, BridgeChip};
use crate::transport::TransportError;
use espflash::connection::{Connection, ResetAfterOperation, ResetBeforeOperation};
use espflash::flasher::Flasher;
use espflash::image_format::Segment;
use espflash::target::{Chip, ProgressCallbacks};
use firmforge_core::device::{DeviceIdentity, SerialType};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

/// The ROM bootloader only ever speaks 115,200 until told otherwise.
const ROM_BAUD: u32 = 115_200;

/// Faster than the ROM default and still comfortably reliable on the USB-to-UART
/// bridges these boards ship with. `espflash` warns above this.
const FLASH_BAUD: u32 = 460_800;

/// What happened during a real write, reported as it happens.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum WriteEvent {
    /// Connected, and this is what is actually on the other end.
    Connected {
        identity: Box<DeviceIdentity>,
    },
    /// Beginning one part.
    PartStarted {
        offset: u32,
        chunks: usize,
    },
    /// Chunks written so far for the current part.
    PartProgress {
        chunks_done: usize,
    },
    /// Reading the region back to check it.
    Verifying,
    /// One part finished. `skipped` means it already held these exact bytes.
    PartFinished {
        skipped: bool,
    },
    Log {
        line: String,
    },
}

/// Translate `espflash`'s callbacks into our event stream.
///
/// `espflash` counts in chunks rather than bytes and gives no total up front
/// beyond the chunk count, so byte-accurate percentages are computed by the
/// caller, which knows each part's real size.
struct ProgressBridge<'a> {
    emit: &'a mut dyn FnMut(WriteEvent),
}

impl ProgressCallbacks for ProgressBridge<'_> {
    fn init(&mut self, addr: u32, total: usize) {
        (self.emit)(WriteEvent::PartStarted {
            offset: addr,
            chunks: total,
        });
    }

    fn update(&mut self, current: usize) {
        (self.emit)(WriteEvent::PartProgress {
            chunks_done: current,
        });
    }

    fn verifying(&mut self) {
        (self.emit)(WriteEvent::Verifying);
    }

    fn finish(&mut self, skipped: bool) {
        (self.emit)(WriteEvent::PartFinished { skipped });
    }
}

/// Map an ESP Web Tools `chipFamily` string onto an `espflash` chip.
///
/// `espflash`'s own `FromStr` will not do this: it expects `esp32s3`, while
/// every manifest in the world says `ESP32-S3`. ESP8266 is deliberately absent
/// — `espflash` 4.x dropped support, and silently substituting anything else
/// would be the exact failure mode this module exists to prevent.
pub fn chip_for_family(family: &str) -> Option<Chip> {
    match family.to_ascii_uppercase().replace('_', "-").as_str() {
        "ESP32" => Some(Chip::Esp32),
        "ESP32-C2" => Some(Chip::Esp32c2),
        "ESP32-C3" => Some(Chip::Esp32c3),
        "ESP32-C5" => Some(Chip::Esp32c5),
        "ESP32-C6" => Some(Chip::Esp32c6),
        "ESP32-H2" => Some(Chip::Esp32h2),
        "ESP32-P4" => Some(Chip::Esp32p4),
        "ESP32-S2" => Some(Chip::Esp32s2),
        "ESP32-S3" => Some(Chip::Esp32s3),
        _ => None,
    }
}

/// The inverse: what a detected chip should be called in manifest terms.
pub fn family_for_chip(chip: Chip) -> String {
    match chip {
        Chip::Esp32 => "ESP32",
        Chip::Esp32c2 => "ESP32-C2",
        Chip::Esp32c3 => "ESP32-C3",
        Chip::Esp32c5 => "ESP32-C5",
        Chip::Esp32c6 => "ESP32-C6",
        Chip::Esp32h2 => "ESP32-H2",
        Chip::Esp32p4 => "ESP32-P4",
        Chip::Esp32s2 => "ESP32-S2",
        Chip::Esp32s3 => "ESP32-S3",
        // `Chip` is non-exhaustive; anything newer than this build knows about
        // gets its espflash name rather than a wrong guess.
        other => return other.to_string().to_ascii_uppercase(),
    }
    .to_string()
}

/// Open a port and connect to whatever is on it.
///
/// `expected` is passed straight to `espflash`, which compares it against the
/// chip's magic number and refuses the connection on a mismatch.
fn connect(port_name: &str, expected: Option<Chip>) -> Result<Flasher, TransportError> {
    let port = serialport::new(port_name, ROM_BAUD)
        .flow_control(serialport::FlowControl::None)
        .open_native()
        .map_err(|e| {
            TransportError::Io(format!(
                "could not open {port_name}: {e}. Another program may be holding it — \
                 a serial monitor, or the Arduino IDE."
            ))
        })?;

    // Enumeration data is only used for reset heuristics; a placeholder is what
    // espflash's own CLI passes for non-USB ports.
    let usb_info = usb_info_for(port_name);

    let connection = Connection::new(
        port,
        usb_info,
        ResetAfterOperation::HardReset,
        ResetBeforeOperation::DefaultReset,
        FLASH_BAUD,
    );

    Flasher::connect(connection, true, true, false, expected, Some(FLASH_BAUD))
        .map_err(|e| TransportError::Io(explain(&e.to_string(), port_name)))
}

/// Look the port back up in the enumeration list so espflash gets the real USB
/// identifiers where we have them.
fn usb_info_for(port_name: &str) -> serialport::UsbPortInfo {
    let found = serialport::available_ports().ok().and_then(|ports| {
        ports.into_iter().find_map(|p| match p.port_type {
            serialport::SerialPortType::UsbPort(usb) if p.port_name == port_name => Some(usb),
            _ => None,
        })
    });

    found.unwrap_or_else(|| serialport::UsbPortInfo {
        vid: 0,
        pid: 0,
        serial_number: None,
        manufacturer: None,
        product: None,
    })
}

/// Turn espflash's wording into something a user can act on.
fn explain(error: &str, port_name: &str) -> String {
    let lower = error.to_lowercase();
    if lower.contains("chip mismatch") || lower.contains("chipmismatch") {
        format!(
            "{error}. firmforge stopped before writing anything: this firmware is built \
             for a different chip than the one on {port_name}."
        )
    } else if lower.contains("timed out") || lower.contains("timeout") || lower.contains("connect")
    {
        format!(
            "{error}. The board did not answer on {port_name}. Most boards need to be put \
             into download mode: hold BOOT, tap RESET, release BOOT, then try again."
        )
    } else if lower.contains("permission") || lower.contains("access") {
        format!("{error}. Something else is using {port_name} — close any serial monitor.")
    } else {
        error.to_string()
    }
}

/// Read the identity of the chip on a port, without writing anything.
///
/// This is what makes the catalogue honest: the chip family it filters by comes
/// from the silicon, not from what the user told us they had.
pub fn detect(port_name: &str) -> Result<DeviceIdentity, TransportError> {
    let mut flasher = connect(port_name, None)?;
    identity_of(&mut flasher, port_name)
}

/// Ask a connected flasher what it is talking to.
fn identity_of(flasher: &mut Flasher, port_name: &str) -> Result<DeviceIdentity, TransportError> {
    let info = flasher
        .device_info()
        .map_err(|e| TransportError::Io(e.to_string()))?;

    Ok(DeviceIdentity {
        chip_family: family_for_chip(info.chip),
        chip_revision: info.revision.map(|(major, _minor)| major),
        flash_size: Some(u64::from(info.flash_size.size())),
        has_psram: info
            .features
            .iter()
            .any(|f| f.to_lowercase().contains("psram")),
        serial_type: serial_type_for(port_name),
        mac: info.mac_address,
    })
}

/// Native USB means CDC; anything behind a bridge chip is UART. This is the
/// same discriminator ESP Web Tools manifests use to pick a build.
fn serial_type_for(port_name: &str) -> SerialType {
    let bridge = serialport::available_ports()
        .ok()
        .and_then(|ports| {
            ports.into_iter().find_map(|p| match p.port_type {
                serialport::SerialPortType::UsbPort(usb) if p.port_name == port_name => {
                    Some(identify_bridge(usb.vid, usb.pid))
                }
                _ => None,
            })
        })
        .unwrap_or(BridgeChip::Unknown);

    match bridge {
        BridgeChip::EspressifNativeUsb => SerialType::Cdc,
        _ => SerialType::Uart,
    }
}

/// Write firmware parts to a real device.
///
/// `expected_family` is the manifest's `chipFamily`. It is *not* advisory: the
/// write is refused unless the silicon agrees.
pub fn write_parts(
    port_name: &str,
    expected_family: &str,
    parts: &[(u32, Vec<u8>)],
    emit: &mut dyn FnMut(WriteEvent),
) -> Result<(), TransportError> {
    if parts.is_empty() {
        return Err(TransportError::Io(
            "there is nothing to write — this build publishes no flashable parts".into(),
        ));
    }

    let expected = chip_for_family(expected_family).ok_or_else(|| {
        TransportError::Io(format!(
            "firmforge cannot flash {expected_family} devices. Its flashing library supports \
             the ESP32 family only, and will not guess at another chip's protocol."
        ))
    })?;

    emit(WriteEvent::Log {
        line: format!("Opening {port_name} and identifying the chip…"),
    });

    let mut flasher = connect(port_name, Some(expected))?;

    let identity = identity_of(&mut flasher, port_name)?;

    emit(WriteEvent::Connected {
        identity: Box::new(identity),
    });

    // espflash pads the on-device write to its block size but checks MD5 against
    // the length we hand it, so an unaligned part would verify against a length
    // the device never stored. Pad here, with 0xFF to match erased flash.
    let padded: Vec<(u32, Vec<u8>)> = parts
        .iter()
        .map(|(offset, data)| {
            let mut bytes = data.clone();
            let remainder = bytes.len() % 4;
            if remainder != 0 {
                bytes.extend(std::iter::repeat_n(0xFF, 4 - remainder));
            }
            (*offset, bytes)
        })
        .collect();

    let mut segments: Vec<Segment<'_>> = padded
        .iter()
        .map(|(offset, bytes)| Segment {
            addr: *offset,
            data: Cow::Borrowed(bytes.as_slice()),
        })
        .collect();
    segments.sort_by_key(|s| s.addr);

    emit(WriteEvent::Log {
        line: format!(
            "Writing {} part(s). Do not unplug the board.",
            segments.len()
        ),
    });

    // Reborrowed and scoped so `emit` is usable again afterwards — a `&mut` is
    // not `Copy`, and moving it into the bridge would end its life here.
    {
        let mut progress = ProgressBridge { emit: &mut *emit };
        flasher
            .write_bins_to_flash(&segments, &mut progress)
            .map_err(|e| TransportError::Io(explain(&e.to_string(), port_name)))?;
    }

    emit(WriteEvent::Log {
        line: "Write complete and verified. The board has been reset.".into(),
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Manifests say `ESP32-S3`; espflash says `esp32s3`. Getting this mapping
    /// wrong in either direction points a flash at the wrong protocol.
    #[test]
    fn maps_manifest_families_onto_espflash_chips() {
        assert_eq!(chip_for_family("ESP32-S3"), Some(Chip::Esp32s3));
        assert_eq!(chip_for_family("esp32-s3"), Some(Chip::Esp32s3));
        assert_eq!(chip_for_family("ESP32"), Some(Chip::Esp32));
        assert_eq!(chip_for_family("ESP32-C3"), Some(Chip::Esp32c3));
    }

    /// espflash 4.x cannot flash an ESP8266. Refusing is the whole point;
    /// falling back to "something ESP32-ish" would brick the board.
    #[test]
    fn refuses_chips_espflash_cannot_drive() {
        assert_eq!(chip_for_family("ESP8266"), None);
        assert_eq!(chip_for_family("RP2040"), None);
        assert_eq!(chip_for_family(""), None);
    }

    #[test]
    fn family_names_round_trip() {
        for family in ["ESP32", "ESP32-C3", "ESP32-C6", "ESP32-S2", "ESP32-S3"] {
            let chip = chip_for_family(family).expect(family);
            assert_eq!(family_for_chip(chip), family);
        }
    }

    #[test]
    fn writing_nothing_is_an_error_not_a_no_op() {
        let mut events = Vec::new();
        let result = write_parts("COM-nonexistent", "ESP32-S3", &[], &mut |e| events.push(e));
        assert!(result.is_err());
        assert!(events.is_empty(), "must not touch the port with no parts");
    }

    /// An unsupported chip must be rejected before the port is even opened.
    #[test]
    fn an_unsupported_chip_is_refused_before_opening_the_port() {
        let mut events = Vec::new();
        let parts = vec![(0x0u32, vec![0u8; 16])];
        let err = write_parts("COM-nonexistent", "ESP8266", &parts, &mut |e| {
            events.push(e)
        })
        .expect_err("ESP8266 must be refused");
        assert!(err.to_string().contains("ESP8266"));
        assert!(events.is_empty());
    }

    #[test]
    fn connection_failures_tell_the_user_what_to_try() {
        let message = explain("Timed out waiting for packet", "COM6");
        assert!(message.contains("BOOT"), "should mention download mode");
        let mismatch = explain("Chip mismatch: expected esp32s3, found esp32", "COM6");
        assert!(mismatch.contains("stopped before writing"));
    }
}
