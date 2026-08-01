//! firmforge transports.
//!
//! The transport layer is deliberately abstract because the platform matrix is
//! not uniform (see `plan/spec/pm-requirements.md` §4):
//!
//! | Transport      | Desktop | Android | iOS |
//! |----------------|---------|---------|-----|
//! | USB serial     | yes (`serialport`) | yes (Kotlin USB-host plugin) | **impossible** — no MFi |
//! | BLE            | yes     | yes     | yes |
//! | OTA (WiFi/BLE) | yes     | yes     | yes |
//!
//! Android cannot use the `serialport` crate at all: apps may not open
//! `/dev/tty*` and must go through the Java `UsbManager` API. That is why this
//! crate exposes a trait rather than a concrete serial type.

pub mod bridge;
pub mod transport;

#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod serial;

pub use bridge::{identify_bridge, BridgeChip};
pub use transport::{Transport, TransportError, TransportKind};
