//! firmforge desktop shell.
//!
//! Deliberately thin: every command delegates to `firmforge-app`, which is
//! shared verbatim with the mobile shell.

use firmforge_app::{build_catalogue, parse_manifest, CatalogueEntry};
use firmforge_core::device::DeviceIdentity;

#[tauri::command]
fn list_ports() -> Result<serde_json::Value, String> {
    serde_json::to_value(firmforge_app::list_serial_ports()).map_err(|e| e.to_string())
}

#[tauri::command]
fn catalogue(
    manifest_json: String,
    device: Option<DeviceIdentity>,
) -> Result<Vec<CatalogueEntry>, String> {
    let manifest = parse_manifest(manifest_json.as_bytes()).map_err(|e| e.to_string())?;
    Ok(build_catalogue(&manifest, device.as_ref()))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![list_ports, catalogue])
        .run(tauri::generate_context!())
        .expect("error while running firmforge");
}
