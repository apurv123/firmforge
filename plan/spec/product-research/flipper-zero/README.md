# Teardown: Flipper Zero — qFlipper, Flipper Lab & the mobile app

> The commercial benchmark. [flipperzero.one](https://flipperzero.one/) — [lab.flipper.net](https://lab.flipper.net/) — [flipperdevices/qFlipper](https://github.com/flipperdevices/qFlipper) (GPL-3.0, Qt/C++)

---

## 1. Product description

Flipper Zero is the polished, funded, retail product that this entire open-source category is reacting against — Bruce's README says so outright: the community wanted Flipper-class capability "without being that overpriced." Studying Flipper is therefore studying **the quality bar users have already been shown**, and which the open ESP32 world is measured against whether it likes it or not.

Three companion surfaces, and the split between them is the most instructive thing here:

| Surface | Role |
|---|---|
| **qFlipper** (desktop, Qt) | Firmware update over USB/DFU, file manager, recovery, streaming screen |
| **Flipper Lab / Web Updater** (`lab.flipper.net`) | Browser client: My Flipper, Apps, Files, CLI, NFC tools, Paint, Pulse Plotter |
| **Mobile apps** (iOS/Android) | Everyday driver: **firmware update over BLE**, app installs, remote control, file access |

## 2. UI teardown — Flipper Lab (`03-lab-web-updater.png`)

A **persistent left navigation rail** — `My Flipper`, `Apps`, `Files`, `CLI`, `NFC tools`, `Paint`, `Pulse Plotter`, with `Settings` and `Connect` pinned at the bottom — and a large empty content area holding a single `CONNECT` button when no device is attached.

This is the key architectural insight of the whole research set: **Flipper treats the device as a workspace with many tools, not as a target for one flash operation.** Firmware update is one item among seven. Bruce's equivalent ("Bruce Lab") is a bare Connect button with nothing behind it; Meshtastic's web client is closer but still device-config-centric. The left-rail device workspace is the right long-term shape for firmforge's device screen.

The **Apps catalogue** (`04-apps-catalog.png`) is the other lesson: a real app store for an embedded device, with categories, screenshots, descriptions, and — critically — **per-firmware-version compatibility gating**, so an app that will not run on your installed firmware is shown as incompatible rather than allowed to fail on-device.

## 3. Features

**qFlipper (desktop):** one-click firmware update with automatic channel selection (Release / Release-Candidate / Development); full **DFU recovery** for bricked devices; SD-card file manager with drag-and-drop; streaming/remote-control of the device screen; backup and restore of internal storage; region/provisioning handling.

**Mobile:** **firmware update over Bluetooth LE** (no cable at all); app installation from the catalogue; file browser; remote control; device info and battery; notifications about new firmware.

**Platform-wide:** signed firmware; staged release channels; an app catalogue with build-per-firmware-version compatibility; hardware add-on modules (WiFi devboard, sub-GHz) recognised by the software.

## 4. Workflows

**A. Update (mobile, the common case):** open app → BLE-connected Flipper appears → "Update available" → tap → firmware downloads to the phone, transfers over BLE, device self-flashes and reboots. No cable, no computer.
**B. Update (desktop):** qFlipper → USB → channel selector → Update → progress → done.
**C. Recover a brick:** hold the button combo → DFU mode → qFlipper detects and offers repair. **This is the feature that makes aggressive updating psychologically safe.**
**D. Install an app:** Lab or mobile → browse catalogue → app shows compatibility with the installed firmware version → Install → lands on the device's SD automatically.
**E. Manage files:** drag and drop in qFlipper, or browse from the phone.

## 5. Screenshots

| File | Page |
|---|---|
| `01-homepage.png` | flipperzero.one landing — the commercial polish bar |
| `02-update-qflipper.png` | **Update page: qFlipper downloads + update paths** |
| `03-lab-web-updater.png` | **Flipper Lab — left-rail device workspace (My Flipper / Apps / Files / CLI / NFC tools / Paint / Pulse Plotter)** |
| `04-apps-catalog.png` | **App catalogue with per-firmware-version compatibility** |
| `05-docs-mobile-app.png` | Mobile app docs |
| `06-qflipper-repo.png` | qFlipper source repo (Qt/C++, GPL-3.0) |
| `07-mobile-app-docs.png` | Android app documentation |
| `08-firmware-update-docs.png` | Firmware update documentation |

## 6. Top user feedback (qFlipper issues by reactions)

qFlipper's tracker is quiet — a healthy sign for a mature updater, and the issues that do exist are almost entirely about **desktop-app hygiene**, which is exactly what a Tauri app must get right:

| Reactions | State | Ask |
|---|---|---|
| 2 | closed | **Stolen-Flipper check** (verify device provenance) |
| 2 | **open** | Missing `libwayland-egl.so.1` on Gentoo |
| 2 | **open** | UI control improvements |
| 2 | **open** | Interface not shown under Wayland |
| 1 | closed | **Firmware update should be blocked if no SD card is installed** |
| 1 | **open** | **Crashes on M1 Mac at the end of firmware flashing** |
| 1 | closed | Installation/update fails; installation corrupted |
| 1 | closed | Drag and drop; window resizing; provide binaries on GitHub Releases |
| 1 | **open** | Connect via Bluetooth from desktop/laptop |

**Reading the signal:**
1. **Linux desktop packaging (Wayland, missing libs) and Apple-silicon crashes are the recurring pain of a native cross-platform GUI.** Tauri (system WebView, no bundled Qt/Chromium) sidesteps most of the Qt-specific version of this, but Linux WebKitGTK packaging and macOS notarisation must be treated as first-class release engineering, not an afterthought.
2. **"Block the update if no SD card is installed"** — precondition checks before writing. Generalise it: verify flash size, chip revision, battery level, and required storage *before* touching the device.
3. **"Stolen-Flipper check"** shows device-identity/provenance is on users' minds even in a consumer context.
4. **"Connect via Bluetooth from desktop"** — users want the transports to be symmetric across clients. Design the transport layer so BLE is available on desktop too, not only on mobile.
5. **"Provide binaries on GitHub Releases"** — ship the app itself the way this audience expects to receive software.

## 7. What firmforge should steal

1. **The left-rail device workspace.** Flash is one tool among many: Overview, Firmware, Files, Console, Config, History.
2. **DFU / recovery mode as a headline feature.** "You cannot permanently brick this" is what makes users willing to update at all. Espressif's ROM bootloader gives us this for free — we must *surface* it.
3. **Firmware update over BLE from the phone.** The single highest-value mobile capability, and completely unserved in the ESP32 open-source world.
4. **Staged release channels** (Release / RC / Dev) as a user-visible setting, matching Bruce's Latest/Beta/Other.
5. **App/asset compatibility gating against the installed firmware version** — never offer an install that cannot work.
6. **Pre-flight precondition checks** (SD present, flash size, battery, free space) with clear blocking messages.
7. **Signed firmware end to end.** Flipper does it; nobody in the ESP32 open-source world does; it is a cheap, credible differentiator.
8. **Backup and restore of device storage** before any destructive operation.

## 8. Where firmforge wins

| Flipper gap | firmforge answer |
|---|---|
| Single-vendor, single-device, closed ecosystem | Any ESP32-class board, any GitHub-hosted firmware |
| Qt desktop app with Linux/Wayland and Apple-silicon packaging pain | Tauri v2: small system-WebView binaries, one Rust core shared with mobile |
| Catalogue is curated by the vendor | Catalogue is *your repo*; no gatekeeper, with signature-based trust instead of editorial trust |
| ~$169 hardware | Runs on $10–$40 boards the user already owns |
