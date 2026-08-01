//! firmforge desktop shell.
//!
//! Deliberately thin: every command delegates to `firmforge-app`, which is
//! shared verbatim with the mobile shell. The only logic that belongs here is
//! turning results into something the webview can consume, and forwarding
//! flash progress as Tauri events so the UI updates during a write rather than
//! after it.

use firmforge_app::{
    build_catalogue, catalogue_from_sources, flash, install, parse_manifest, CatalogueEntry,
    DiscoveredManifest, Source, Target,
};
use firmforge_core::device::DeviceIdentity;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

/// Everything the user needs to see before confirming an install.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InstallPlan {
    name: String,
    version: String,
    chip_family: String,
    total_bytes: usize,
    parts: Vec<install::PartSummary>,
    /// True when every part carried a checksum that matched.
    fully_verified: bool,
    /// True when no part is served from the manifest's own repository.
    all_upstream: bool,
}

#[tauri::command]
fn list_ports() -> Result<serde_json::Value, String> {
    serde_json::to_value(firmforge_app::list_serial_ports()).map_err(|e| e.to_string())
}

/// The simulated device, so the whole workflow can be exercised without
/// hardware and without writing to anything.
#[tauri::command]
fn demo_device() -> Target {
    flash::demo_target()
}

#[tauri::command]
fn catalogue(
    manifest_json: String,
    device: Option<DeviceIdentity>,
) -> Result<Vec<CatalogueEntry>, String> {
    let manifest = parse_manifest(manifest_json.as_bytes()).map_err(|e| e.to_string())?;
    Ok(build_catalogue(&manifest, device.as_ref()))
}

/// The chip families the app offers as its first-run choice.
#[tauri::command]
fn chip_families() -> Vec<firmforge_core::ChipFamily> {
    firmforge_core::chip::all()
}

/// The curated source list, so a fresh install is not an empty screen.
#[tauri::command]
fn builtin_sources() -> Vec<firmforge_core::BuiltinSource> {
    firmforge_core::builtin::all()
}

/// When the built-in list was last verified against the live URLs.
#[tauri::command]
fn builtin_curated_on() -> String {
    firmforge_core::builtin::CURATED_ON.to_string()
}

/// Load the sources a fresh install starts with.
#[tauri::command]
async fn load_bundled_sources() -> Vec<firmforge_app::SourceStatus> {
    firmforge_app::load_bundled().await
}

/// Reload a specific set of built-ins — the ones the user has kept.
#[tauri::command]
async fn load_builtin_sources(ids: Vec<String>) -> Vec<firmforge_app::SourceStatus> {
    firmforge_app::load_builtins(&ids).await
}

/// Add a firmware source: a GitHub repository, or a published manifest URL.
#[tauri::command]
async fn add_source(repo: String) -> Result<Vec<DiscoveredManifest>, String> {
    let source = Source::parse(&repo).map_err(|e| e.to_string())?;
    firmforge_app::discover(&source)
        .await
        .map_err(|e| e.to_string())
}

/// Turn discovered manifests into catalogue cards for a given device.
#[tauri::command]
fn catalogue_for(
    discovered: Vec<DiscoveredManifest>,
    device: Option<DeviceIdentity>,
) -> Vec<CatalogueEntry> {
    catalogue_from_sources(&discovered, device.as_ref())
}

/// Download and verify a build without writing anything, returning the plan the
/// user confirms. Failing here is safe; failing after a write has begun is not.
#[tauri::command]
async fn prepare_install(
    discovered: DiscoveredManifest,
    build_index: usize,
) -> Result<InstallPlan, String> {
    let build = discovered
        .manifest
        .builds
        .get(build_index)
        .ok_or_else(|| format!("build {build_index} is not in this manifest"))?;

    let prepared = install::prepare(build, &discovered.manifest_url)
        .await
        .map_err(|e| e.to_string())?;

    Ok(InstallPlan {
        name: discovered.manifest.name.clone(),
        version: discovered.manifest.display_version().to_string(),
        chip_family: build.chip_family.clone(),
        total_bytes: prepared.total_bytes(),
        fully_verified: !prepared.summaries.is_empty()
            && prepared.summaries.iter().all(|p| p.verified),
        all_upstream: prepared.summaries.iter().all(|p| p.upstream),
        parts: prepared.summaries,
    })
}

/// Identify the chip on a port without writing anything.
///
/// This is what lets the catalogue filter by what is actually connected rather
/// than by what the user believes they plugged in.
#[tauri::command]
async fn detect_device(port: String) -> Result<Target, String> {
    let identity = tokio::task::spawn_blocking({
        let port = port.clone();
        move || firmforge_flash::esp::detect(&port)
    })
    .await
    .map_err(|e| format!("device detection stopped unexpectedly: {e}"))?
    .map_err(|e| e.to_string())?;

    Ok(Target {
        port,
        identity,
        demo: false,
    })
}

/// Run the install. Progress arrives on the `flash` event channel.
///
/// `demo` writes nothing; it re-downloads, re-verifies and simulates the write
/// so a manifest can be rehearsed end to end before real hardware is risked.
/// When `demo` is false, `port` must name a connected device — the write is
/// refused if the chip on that port disagrees with the build's `chipFamily`.
#[tauri::command]
async fn run_install(
    app: AppHandle,
    discovered: DiscoveredManifest,
    build_index: usize,
    demo: bool,
    port: Option<String>,
) -> Result<(), String> {
    let emit = {
        let app = app.clone();
        move |event: flash::FlashEvent| {
            let _ = app.emit("flash", event);
        }
    };

    let port = if demo {
        None
    } else {
        let port = port.unwrap_or_default();
        if port.is_empty() {
            let message = "Choose the port your device is connected to first.".to_string();
            emit(flash::FlashEvent::Failed {
                message: message.clone(),
            });
            return Err(message);
        }
        Some(port)
    };

    let build = discovered
        .manifest
        .builds
        .get(build_index)
        .ok_or_else(|| format!("build {build_index} is not in this manifest"))?;

    let prepared = match install::prepare(build, &discovered.manifest_url).await {
        Ok(p) => p,
        Err(e) => {
            emit(flash::FlashEvent::Failed {
                message: e.to_string(),
            });
            return Err(e.to_string());
        }
    };

    match port {
        // Everything is downloaded and checksummed before this point, so the
        // window in which a failure can leave the board half-written is as
        // narrow as the write itself.
        Some(port) => flash::run_serial(&port, &build.chip_family, &prepared, emit).await,
        None => {
            flash::run_demo(&prepared, emit).await;
            Ok(())
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            list_ports,
            demo_device,
            catalogue,
            catalogue_for,
            add_source,
            chip_families,
            builtin_sources,
            builtin_curated_on,
            load_bundled_sources,
            load_builtin_sources,
            prepare_install,
            detect_device,
            run_install
        ])
        .run(tauri::generate_context!())
        .expect("error while running firmforge");
}
