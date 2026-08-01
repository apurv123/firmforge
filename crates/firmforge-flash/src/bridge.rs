//! USB-to-UART bridge identification — the "driver doctor" (feature F-17).
//!
//! Missing bridge drivers are the single most common first-run failure across
//! every product in the competitive research. Identifying the exact chip from
//! its USB VID/PID lets the app link the right driver *before* the user hits a
//! confusing failure, and lets us infer whether the connection is a native USB
//! CDC link or an external bridge (the ESP Web Tools `serialType` axis).

use firmforge_core::device::SerialType;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BridgeChip {
    /// WCH CH340 / CH341.
    Ch34x,
    /// WCH CH9102 (common on newer M5Stack boards).
    Ch9102,
    /// Silicon Labs CP210x.
    Cp210x,
    /// FTDI FT232 and friends.
    Ftdi,
    /// Espressif native USB (JTAG/serial) — no bridge, no driver needed.
    EspressifNativeUsb,
    Unknown,
}

impl BridgeChip {
    /// Whether the host must install a vendor driver for this bridge.
    pub fn needs_vendor_driver(self) -> bool {
        !matches!(self, BridgeChip::EspressifNativeUsb | BridgeChip::Unknown)
    }

    /// The connection type this implies, used to pick between `cdc` and
    /// `uart` builds in a manifest.
    pub fn serial_type(self) -> SerialType {
        match self {
            BridgeChip::EspressifNativeUsb => SerialType::Cdc,
            _ => SerialType::Uart,
        }
    }

    pub fn driver_url(self) -> Option<&'static str> {
        match self {
            BridgeChip::Ch34x | BridgeChip::Ch9102 => {
                Some("https://www.wch-ic.com/downloads/CH341SER_EXE.html")
            }
            BridgeChip::Cp210x => {
                Some("https://www.silabs.com/developers/usb-to-uart-bridge-vcp-drivers")
            }
            BridgeChip::Ftdi => Some("https://ftdichip.com/drivers/vcp-drivers/"),
            _ => None,
        }
    }
}

/// Identify a bridge from its USB vendor and product id.
pub fn identify_bridge(vid: u16, pid: u16) -> BridgeChip {
    match (vid, pid) {
        (0x1a86, 0x7523) | (0x1a86, 0x5523) => BridgeChip::Ch34x,
        (0x1a86, 0x55d4) | (0x1a86, 0x55d3) => BridgeChip::Ch9102,
        (0x10c4, 0xea60) | (0x10c4, 0xea70) | (0x10c4, 0xea71) => BridgeChip::Cp210x,
        (0x0403, _) => BridgeChip::Ftdi,
        // Espressif native USB-serial/JTAG.
        (0x303a, _) => BridgeChip::EspressifNativeUsb,
        _ => BridgeChip::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifies_common_bridges() {
        assert_eq!(identify_bridge(0x1a86, 0x7523), BridgeChip::Ch34x);
        assert_eq!(identify_bridge(0x1a86, 0x55d4), BridgeChip::Ch9102);
        assert_eq!(identify_bridge(0x10c4, 0xea60), BridgeChip::Cp210x);
        assert_eq!(identify_bridge(0x0403, 0x6001), BridgeChip::Ftdi);
    }

    #[test]
    fn espressif_native_usb_needs_no_driver_and_implies_cdc() {
        let chip = identify_bridge(0x303a, 0x1001);
        assert_eq!(chip, BridgeChip::EspressifNativeUsb);
        assert!(!chip.needs_vendor_driver());
        assert_eq!(chip.serial_type(), SerialType::Cdc);
    }

    #[test]
    fn bridges_imply_uart_and_offer_a_driver_link() {
        let chip = identify_bridge(0x10c4, 0xea60);
        assert_eq!(chip.serial_type(), SerialType::Uart);
        assert!(chip.driver_url().is_some());
    }
}
