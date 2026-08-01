//! Desktop host serial enumeration.
//!
//! Not compiled on Android or iOS: neither platform allows an application to
//! open a host serial port. See `plan/spec/pm-requirements.md` §4.

use crate::bridge::{identify_bridge, BridgeChip};
use crate::transport::TransportError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerialPortInfo {
    pub port_name: String,
    pub vid: Option<u16>,
    pub pid: Option<u16>,
    pub product: Option<String>,
    pub bridge: BridgeChip,
    /// True when the port looks like a bridge whose driver may be missing.
    pub needs_vendor_driver: bool,
}

/// List candidate USB serial ports on the host.
pub fn list_ports() -> Result<Vec<SerialPortInfo>, TransportError> {
    let ports = serialport::available_ports().map_err(|e| TransportError::Io(e.to_string()))?;

    Ok(ports
        .into_iter()
        .filter_map(|p| match p.port_type {
            serialport::SerialPortType::UsbPort(usb) => {
                let bridge = identify_bridge(usb.vid, usb.pid);
                Some(SerialPortInfo {
                    port_name: p.port_name,
                    vid: Some(usb.vid),
                    pid: Some(usb.pid),
                    product: usb.product,
                    bridge,
                    needs_vendor_driver: bridge.needs_vendor_driver(),
                })
            }
            _ => None,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Enumeration must never panic, on any host, with or without a device
    /// attached — it runs on every app launch.
    #[test]
    fn enumeration_does_not_panic() {
        let _ = list_ports();
    }
}
