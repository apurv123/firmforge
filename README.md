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

- Pick your chip family first. ESP32-S3 is supported; the other six ESP32-class
  families are listed and marked unsupported rather than hidden, so you find out
  before you install, not after
- Start with firmware already there. ESPHome, MicroPython, WLED and Tasmota are
  added on first run, so there is something to browse before you have configured
  anything. Bruce and ESP32 Marauder are listed but not added — they are
  security-testing tools, so adding them is a deliberate choice
- Remove any built-in source you do not want; the removal sticks, and
  restore-defaults brings them back
- Add your own source, by `owner/repo` or by a link to a published
  `manifest.json` — most projects publish theirs outside their repository
- Read `firmware/manifest.json` from a repository's default branch, and
  `manifest.json` assets attached to its releases
- Browse a catalogue of every build, filtered to your chip and checked against
  the selected device, with plain-language reasons when something does not fit
  ("needs 8 MB flash, yours has 4 MB") rather than silently hiding it
- Download every part and verify its SHA-256 **before** anything is written; a
  mismatch refuses the install rather than warning
- See exactly where each byte comes from, and whether it is the publisher's own
  release asset or a rehosted copy
- **Flash a real ESP32** over USB: firmforge identifies the chip by talking to
  its ROM bootloader, refuses any build meant for a different chip, writes each
  part, and reads it back to confirm what landed
- **Talk to the board afterwards.** The Playground screen is a real serial
  console: watch output, type at the prompt, interrupt a running program with
  Ctrl-C, soft reboot with Ctrl-D. On MicroPython it also runs scripts and
  saves files to the board — name one `main.py` and it runs on every power-up.
  That is the job people currently install Thonny for.

**What does not work yet**

- The mobile app. The shared crates are structured for it, but no Android or
  iOS project exists yet.
- Signature verification, rollback, and the serial console.

**About those built-in sources**

They are a snapshot taken at release, and some publishers pin a version into the
URL, so a few will go stale. firmforge refetches them on every launch rather
than caching, so staleness is visible: a source that no longer loads says so in
plain language, with the date the list was checked, and can be removed. Bruce is
listed but cannot be installed at all — it publishes bare `.bin` files with no
manifest, so the flash offsets are unknown, and guessing them is how boards get
bricked.

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

> **What you get today:** the full workflow — connect a board, browse firmware
> that is already there, download and verify it, flash it over USB, then talk to
> it. Three screens: **Device**, **Flash**, **Playground**.

### With a real board: firmware, then Python, without leaving the app

1. Install a build (below) and open firmforge. On a fresh install it asks once
   which chip you have — *ESP32-S3*. ESPHome, MicroPython, WLED and Tasmota load
   on their own. After that the chip lives in the top bar.
2. **Device** → firmforge scans on launch and, if exactly one recognisable board
   is plugged in, identifies it by itself. Otherwise pick the port and press
   *Identify this device*. Either way it reads the chip, revision, flash size
   and MAC from the silicon itself, and the top bar shows what you are plugged
   into from then on. *What is on this board?* tells you which firmware is
   already there before you overwrite it.
3. **Flash** → *Install* on **MicroPython**, then *Flash*. Builds meant for
   another chip stay dimmed, and firmforge refuses one even if you force it.
4. **Playground** → pick the same port → *Connect* → *Interrupt (Ctrl-C)*. The
   `>>>` prompt appears. Type `print(2 ** 10)` and press Enter.
5. Paste a script into **Run a script** and press *Run on board*. Indentation
   survives, and an exception comes back as an error rather than being lost in
   the output.
6. Put it in **Put a file on the board** as `main.py` and press *Save and
   reboot*. It now runs every time the board powers on.

Steps 4 to 6 are what Thonny is normally installed for.

**Settings** (top right) is where firmware sources live — add ESP32 Marauder
there from *Also available* and it appears in **Flash** alongside the rest.
**Activity** opens a log of everything firmforge did, over whatever screen you
are on.

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
