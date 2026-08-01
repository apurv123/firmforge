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

The **desktop app works end to end, except for the final write to hardware.**
Builds and tests are green on Windows, macOS and Linux, and installers are
produced by CI — see [Installing and testing](#installing-and-testing).

**What works today**

- Add a GitHub repository as a firmware source, by `owner/repo` or URL
- Read `firmware/manifest.json` from its default branch, and `manifest.json`
  assets attached to its releases
- Browse a catalogue of every build, checked against the selected device, with
  plain-language reasons when something does not fit ("needs 8 MB flash, yours
  has 4 MB") rather than silently hiding it
- Download every part and verify its SHA-256 **before** anything is written; a
  mismatch refuses the install rather than warning
- See exactly where each byte comes from, and whether it is the publisher's own
  release asset or a rehosted copy
- Run the whole thing against a **demo device**, so the workflow can be tried
  without owning an ESP32 and without writing to anything

**What does not work yet**

- Writing to real hardware. `espflash` is not wired in, so selecting a real
  serial port will tell you so rather than pretending.
- The mobile app. The shared crates are structured for it, but no Android or
  iOS project exists yet.
- Signature verification, rollback, and the serial console.

**Crates**

- **`firmforge-core`** — the ESP Web Tools-compatible manifest format plus firmforge extensions (channels, per-part SHA-256, signatures, constraints, variants, assets), device identity, compatibility matching, part URL resolution and artifact verification.
- **`firmforge-flash`** — transport abstraction across USB serial / Android USB host / BLE / OTA, USB-to-UART bridge identification for the driver doctor, and desktop serial enumeration.
- **`firmforge-app`** — GitHub sourcing, install preparation and the shared command surface used by both shells.
- **`apps/desktop`** — the Tauri v2 shell.
- **`firmware/`** — the repository convention, as a worked example that describes real published firmware and is checked in tests.

## Documentation

| Document | What it is |
|---|---|
| [`plan/spec/firmforge-spec.md`](plan/spec/firmforge-spec.md) | **The product specification** — desktop and mobile app descriptions, features, 7 workflows, 19 UX screens, architecture, roadmap, metrics |
| [`plan/spec/pm-requirements.md`](plan/spec/pm-requirements.md) | PM/technical requirements — GitHub sourcing, flashing stack, platform constraints, security model, NFRs, risks |
| [`plan/spec/product-research/`](plan/spec/product-research/) | Teardowns of Bruce, ESP32 Marauder, ESPHome, ESP Web Tools, M5Burner, Meshtastic and Flipper Zero, with 52 screenshots |
| [`plan/spec/legal-and-licensing.md`](plan/spec/legal-and-licensing.md) | Licence obligations around redistributing firmware, trademark and app-store constraints |
| [`firmware/README.md`](firmware/README.md) | The firmware repository convention |

## Installing and testing

> **What you get today:** the full workflow — add a GitHub source, browse a
> catalogue, download and verify firmware — against a demo device. Writing to
> real hardware is not implemented yet.

### Try it in five clicks

1. Install a build (below) and open firmforge.
2. **Sources** → type `apurv123/firmforge` → *Add source*.
3. **Device** → *Use demo device*.
4. **Catalogue** → *Install* on the ESP32-S3 build. The ESP32 build beside it
   stays dimmed, and tells you why.
5. **Install** → check the parts table, then *Run demo install*, and watch the
   **Console**.

That downloads ~1.5 MB of real ESP32 Marauder firmware from Marauder's own
release page, verifies all four SHA-256 digests, and simulates the write.
Nothing is written to any hardware.

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
