# firmforge

**A Rust-first monorepo for a cross-platform firmware companion app — desktop (Windows/macOS/Linux) and mobile (Android/iOS) — that discovers, downloads, verifies and flashes firmware straight from a GitHub repository's `firmware/` folder.**

Built on [Tauri v2](https://v2.tauri.app/), which is the only mainstream Rust application framework that ships **one codebase to both desktop and mobile**.

---

## Why this exists

Flashing firmware to ESP32-class devices today means juggling `esptool.py`, PlatformIO, vendor web-flashers, Discord links to `.bin` files, and a lot of tribal knowledge. Projects like **Bruce**, **ESP32 Marauder**, **ESPHome** and **M5Burner** each solved a slice of it. `firmforge` aims to be the *companion app layer*: a signed, versioned, GitHub-backed firmware catalogue with a one-tap flash + post-flash device console, on the desktop **and** the phone in your pocket.

## Repository layout

```
firmforge/
├── plan/
│   └── spec/
│       ├── product-research/     # competitor teardowns + screenshots
│       ├── pm-requirements.md    # PM requirements research
│       └── firmforge-spec.md     # the combined product spec
├── crates/
│   ├── firmforge-core/           # platform-agnostic Rust: catalogue, GitHub client, verification
│   ├── firmforge-flash/          # serial/DFU/OTA transports
│   └── firmforge-app/            # shared Tauri command surface (desktop + mobile)
├── apps/
│   ├── desktop/                  # Tauri v2 desktop shell
│   └── mobile/                   # Tauri v2 mobile shell (Android/iOS)
├── ui/                           # shared frontend (TypeScript)
└── firmware/                     # firmware artifacts + manifest consumed by the app
```

## Status

[![ci](https://github.com/apurv123/firmforge/actions/workflows/ci.yml/badge.svg)](https://github.com/apurv123/firmforge/actions/workflows/ci.yml)

**Milestone M0 — Skeleton: complete.** The workspace builds and 20 tests pass on Windows, macOS and Linux, and the desktop app has been built and launched successfully on all three platforms. See [Installing and testing](#installing-and-testing) to try it.

It is **not yet a usable flasher**: the app enumerates serial ports and renders the
first screen, but no firmware can be written to a device until M2.

Implemented so far:

- **`firmforge-core`** — the ESP Web Tools-compatible manifest format plus firmforge extensions (channels, per-part SHA-256, signatures, constraints, variants, assets), device identity, compatibility matching with plain-language reasons, and artifact verification.
- **`firmforge-flash`** — transport abstraction across USB serial / Android USB host / BLE / OTA, USB-to-UART bridge identification for the driver doctor, and desktop serial enumeration.
- **`firmforge-app`** — the shared command surface used by both shells (and, later, a headless CLI).
- **`apps/desktop`** — a Tauri v2 shell rendering spec screen D2 and calling the real `list_ports` command.
- **`firmware/`** — the repository convention with a worked, parsed-in-tests manifest.

Next: **M1 — Read** (GitHub sourcing, catalogue UI, cache). See the [roadmap](plan/spec/firmforge-spec.md#11-roadmap).

## Documentation

| Document | What it is |
|---|---|
| [`plan/spec/firmforge-spec.md`](plan/spec/firmforge-spec.md) | **The product specification** — desktop and mobile app descriptions, features, 7 workflows, 19 UX screens, architecture, roadmap, metrics |
| [`plan/spec/pm-requirements.md`](plan/spec/pm-requirements.md) | PM/technical requirements — GitHub sourcing, flashing stack, platform constraints, security model, NFRs, risks |
| [`plan/spec/product-research/`](plan/spec/product-research/) | Teardowns of Bruce, ESP32 Marauder, ESPHome, ESP Web Tools, M5Burner, Meshtastic and Flipper Zero, with 52 screenshots |
| [`firmware/README.md`](firmware/README.md) | The firmware repository convention |

## Installing and testing

> **What you get today:** the app opens screen D2 and enumerates real serial ports.
> It cannot flash a device yet — that arrives in M2. Treat this as a walking
> skeleton, not a tool.

### Option A — install a prebuilt binary (no toolchain needed)

1. Go to [**Actions → desktop-build**](https://github.com/apurv123/firmforge/actions/workflows/desktop-build.yml) and press **Run workflow** on `main`.
2. When it finishes, download the artifact for your platform from the run summary:

   | Platform | Artifact | Contents |
   |---|---|---|
   | Windows | `firmforge-windows` | `.msi`, NSIS `-setup.exe`, and a portable `firmforge-desktop.exe` |
   | macOS (Apple silicon) | `firmforge-macos-apple-silicon` | `.dmg` and `.app` |
   | Linux | `firmforge-linux` | `.deb`, `.AppImage`, and a portable binary |

3. Unzip and run. The portable executable needs no installation.

The builds are **unsigned**, so Windows SmartScreen shows "Windows protected your
PC" (choose *More info → Run anyway*) and macOS requires *right-click → Open* the
first time. Windows also needs the WebView2 runtime, which ships with Windows 11
and current Windows 10.

### Option B — build and run from source

```bash
cargo install tauri-cli --version "^2" --locked
cd apps/desktop/src-tauri
cargo tauri dev      # hot-reloading dev build
cargo tauri build    # release bundles in ../../../target/release/bundle
```

### Running the tests

```bash
cargo test -p firmforge-core -p firmforge-flash -p firmforge-app
```

This covers manifest parsing, compatibility matching, artifact verification and
USB bridge identification, and needs no GUI toolchain.

On Linux the desktop shell additionally needs `libudev-dev` and `libwebkit2gtk-4.1-dev`
(see [`ci.yml`](.github/workflows/ci.yml) for the full package list).

### If a local build is blocked

Some managed Windows machines run an Application Control (WDAC) policy that
refuses to execute freshly compiled build scripts, failing with
`An Application Control policy has blocked this file. (os error 4551)`. Cargo
cannot work around this, and you should not try to disable the policy. Use
**Option A** instead — CI-produced binaries run normally — or build under WSL or
on an unmanaged machine.

## License

MIT (see `LICENSE`).
