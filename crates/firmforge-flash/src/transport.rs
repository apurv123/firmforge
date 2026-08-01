use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TransportKind {
    /// Host serial port (desktop only).
    UsbSerial,
    /// Android USB host API via the native Kotlin plugin.
    AndroidUsbHost,
    /// Bluetooth Low Energy — the only flashing path available on iOS.
    Ble,
    /// Over-the-air update handled by cooperating firmware.
    Ota,
}

#[derive(Debug, Error)]
pub enum TransportError {
    #[error("no device found on this transport")]
    NotFound,

    #[error("permission denied opening {port}: {hint}")]
    PermissionDenied { port: String, hint: String },

    #[error("transport is not supported on this platform: {0}")]
    Unsupported(&'static str),

    #[error("io error: {0}")]
    Io(String),
}

/// A byte-oriented link to a device.
///
/// Implementations: host serial (desktop), Android USB host (via plugin), BLE
/// (via `btleplug`), and OTA.
pub trait Transport {
    fn kind(&self) -> TransportKind;
    fn write(&mut self, bytes: &[u8]) -> Result<usize, TransportError>;
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, TransportError>;
    fn set_baud(&mut self, baud: u32) -> Result<(), TransportError>;
}
