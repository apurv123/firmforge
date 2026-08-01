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
use firmforge_flash::{micropython, Console, TransportError};
use serde::Serialize;
use std::sync::{Arc, Mutex};
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

/// The open serial console, if there is one.
///
/// A serial port can only be held by one program at a time, so this is also the
/// interlock: flashing takes the port away from the console rather than failing
/// halfway through with a permission error nobody can act on.
#[derive(Default)]
struct ConsoleState(Arc<Mutex<Option<Console>>>);

/// Close the console if it is holding `port`. Returns true if it was.
fn release_port(state: &ConsoleState, port: &str) -> bool {
    let mut guard = match state.0.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    if guard.as_ref().is_some_and(|c| c.port_name() == port) {
        // Dropping stops the reader thread and closes the handle.
        *guard = None;
        true
    } else {
        false
    }
}

/// Run `f` against the open console.
fn with_console<T>(
    state: &ConsoleState,
    f: impl FnOnce(&mut Console) -> Result<T, TransportError>,
) -> Result<T, String> {
    let mut guard = match state.0.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    let console = guard
        .as_mut()
        .ok_or_else(|| "Not connected to a board yet.".to_string())?;
    f(console).map_err(|e| e.to_string())
}

/// Open a serial console and start streaming whatever the board says.
#[tauri::command]
fn console_open(
    app: AppHandle,
    state: tauri::State<'_, ConsoleState>,
    port: String,
    baud: Option<u32>,
) -> Result<(), String> {
    let baud = baud.unwrap_or(firmforge_flash::console::DEFAULT_BAUD);
    let emitter = app.clone();
    let console = Console::open(&port, baud, move |text| {
        let _ = emitter.emit("serial", text);
    })
    .map_err(|e| e.to_string())?;

    let mut guard = match state.0.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    // Assigning drops any previous console, releasing the port it held.
    *guard = Some(console);
    Ok(())
}

#[tauri::command]
fn console_close(state: tauri::State<'_, ConsoleState>) {
    let mut guard = match state.0.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    *guard = None;
}

/// The port the console currently holds, if any.
#[tauri::command]
fn console_port(state: tauri::State<'_, ConsoleState>) -> Option<String> {
    let guard = match state.0.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    guard.as_ref().map(|c| c.port_name().to_string())
}

/// Send a line, as if it had been typed at the prompt.
#[tauri::command]
fn console_send(state: tauri::State<'_, ConsoleState>, text: String) -> Result<(), String> {
    with_console(&state, |c| {
        c.write(text.as_bytes())?;
        c.write(b"\r\n")
    })
}

/// Send one of the control codes a MicroPython user needs.
///
/// These are the keystrokes the Thonny instructions describe: Ctrl-C to break
/// into a running program, Ctrl-D to soft reboot.
#[tauri::command]
fn console_control(state: tauri::State<'_, ConsoleState>, code: String) -> Result<(), String> {
    with_console(&state, |c| match code.as_str() {
        "interrupt" => micropython::interrupt(c),
        "softReboot" => micropython::soft_reboot(c),
        "hardReset" => c.hardware_reset(),
        "enter" => c.write(b"\r\n"),
        other => Err(TransportError::Io(format!(
            "unknown console action {other}"
        ))),
    })
}

/// Ask the board whether MicroPython is running, and what it is.
#[tauri::command]
async fn console_probe(
    state: tauri::State<'_, ConsoleState>,
) -> Result<micropython::Probe, String> {
    let shared = Arc::clone(&state.0);
    tokio::task::spawn_blocking(move || {
        let mut guard = match shared.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        match guard.as_mut() {
            Some(console) => Ok(micropython::probe(console)),
            None => Err("Not connected to a board yet.".to_string()),
        }
    })
    .await
    .map_err(|e| format!("the board check stopped unexpectedly: {e}"))?
}

/// Run a snippet of Python on the board and return what it printed.
///
/// This goes through the raw REPL rather than being typed at the prompt, so
/// multi-line code is not mangled by auto-indent and a traceback comes back
/// separately from ordinary output.
#[tauri::command]
async fn console_run(
    state: tauri::State<'_, ConsoleState>,
    code: String,
) -> Result<micropython::ExecOutput, String> {
    let shared = Arc::clone(&state.0);
    tokio::task::spawn_blocking(move || {
        let mut guard = match shared.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        match guard.as_mut() {
            Some(console) => micropython::exec(console, &code).map_err(|e| e.to_string()),
            None => Err("Not connected to a board yet.".to_string()),
        }
    })
    .await
    .map_err(|e| format!("the script stopped unexpectedly: {e}"))?
}

/// Put a file on the board's filesystem, then check what landed.
///
/// Saving as `main.py` is how a script gets to run on boot, which is the whole
/// point of putting it on the board rather than running it from here.
#[tauri::command]
async fn console_upload(
    app: AppHandle,
    state: tauri::State<'_, ConsoleState>,
    path: String,
    contents: String,
) -> Result<micropython::Upload, String> {
    let shared = Arc::clone(&state.0);
    tokio::task::spawn_blocking(move || {
        let mut guard = match shared.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        let console = guard
            .as_mut()
            .ok_or_else(|| "Not connected to a board yet.".to_string())?;

        let data = contents.as_bytes();
        let total = data.len();
        micropython::upload(console, &path, data, |sent| {
            let _ = app.emit(
                "upload",
                serde_json::json!({ "sentBytes": sent, "totalBytes": total }),
            );
        })
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("the upload stopped unexpectedly: {e}"))?
}

/// Identify the chip on a port without writing anything.
///
/// This is what lets the catalogue filter by what is actually connected rather
/// than by what the user believes they plugged in.
#[tauri::command]
async fn detect_device(
    state: tauri::State<'_, ConsoleState>,
    port: String,
) -> Result<Target, String> {
    // Identifying drives the ROM bootloader, which needs the port to itself.
    release_port(&state, &port);

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
    state: tauri::State<'_, ConsoleState>,
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
        // A port belongs to one program at a time. Taking it from the console
        // here turns what would be an opaque "access denied" into something
        // that just works, and says so.
        if release_port(&state, &port) {
            emit(flash::FlashEvent::Log {
                line: format!("Closed the terminal on {port} so it can be flashed."),
            });
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
        .manage(ConsoleState::default())
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
            console_open,
            console_close,
            console_port,
            console_send,
            console_control,
            console_probe,
            console_run,
            console_upload,
            run_install
        ])
        .run(tauri::generate_context!())
        .expect("error while running firmforge");
}
