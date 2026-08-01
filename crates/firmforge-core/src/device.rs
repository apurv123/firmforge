//! Device identity as read from the chip over the wire.

use serde::{Deserialize, Serialize};

/// How the host is connected to the chip. Mirrors the ESP Web Tools
/// `serialType` discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SerialType {
    /// Native USB CDC (built-in USB peripheral).
    Cdc,
    /// External USB-to-UART bridge (CH34x, CP210x, FTDI, ...).
    Uart,
}

impl SerialType {
    pub fn as_str(self) -> &'static str {
        match self {
            SerialType::Cdc => "cdc",
            SerialType::Uart => "uart",
        }
    }
}

/// What we learn from the chip *before* offering any firmware.
///
/// Detecting this first is what structurally eliminates the "I flashed the
/// wrong .bin" class of support issue seen throughout the ESP32 Marauder
/// tracker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceIdentity {
    /// e.g. `ESP32`, `ESP32-S3`, `ESP32-C3`.
    pub chip_family: String,
    pub chip_revision: Option<u32>,
    pub flash_size: Option<u64>,
    pub has_psram: bool,
    pub serial_type: SerialType,
    /// Stable identity across reflashes.
    pub mac: Option<String>,
}

/// A device remembered in the user's library.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub id: String,
    pub name: String,
    pub identity: DeviceIdentity,
    /// Board id resolved from the catalogue, e.g. `m5stack-cardputer`.
    pub board_id: Option<String>,
    /// Version string of the build currently believed to be installed.
    pub installed_version: Option<String>,
    /// Peripheral modules the user has attached (CC1101, NRF24, IR, ...).
    /// Users think in terms of "my board plus these modules"; builds are
    /// gated accordingly.
    pub modules: Vec<String>,
}
