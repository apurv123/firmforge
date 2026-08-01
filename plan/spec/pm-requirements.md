# PM Requirements Research — a desktop app that flashes firmware from a GitHub repo's `firmware/` folder

**Scope of this document.** What it actually takes, technically and product-wise, to ship a Rust-stack desktop application (plus a mobile sibling) that reads firmware artifacts out of a GitHub repository's `firmware/` directory and puts them on an ESP32-class device. This is the engineering-and-constraints companion to [`product-research/`](product-research/); the two feed [`firmforge-spec.md`](firmforge-spec.md).

**Verified 2026-08-01** against crates.io and vendor documentation. Every crate named below was version-checked.

---

## 1. Is the Rust stack viable for desktop *and* mobile? Yes — with one framework.

**Tauri v2 (2.11.x)** is the only mainstream Rust application framework that targets Windows, macOS, Linux, **Android and iOS** from a single Rust workspace. It uses the OS's own WebView for UI (WebView2 / WKWebView / WebKitGTK / Android System WebView), so binaries are small and there is no bundled browser engine.

Practical consequences:

| Consequence | Impact on the product |
|---|---|
| One Rust core, five targets | All device/catalogue/verification logic is written once in `firmforge-core` |
| System WebView, not Chromium | ~5–15 MB installers vs. ~100 MB+ for Electron; matters for a tool people fetch from a Releases page |
| WebView differs per platform | UI must be tested on WebKitGTK (the strictest) — no Chrome-only CSS/JS |
| Mobile plugins are Kotlin/Swift shims | Any capability the OS gates behind Java/Objective-C APIs (USB host, BLE background) needs a small native plugin. **Budget for this.** |
| Rust core is `no_std`-friendly-ish, but not on mobile serial | Serial access on Android does *not* go through the `serialport` crate — see §4 |

**Alternatives considered and rejected:** Electron (not Rust, 10× binary size); Dioxus/Slint (excellent Rust UI, but weaker mobile packaging/plugin story than Tauri today); egui (great for tools, poor mobile input/IME); Flutter+Rust bridge (Dart is then the app language, defeating the "Rust stack" requirement); native-per-platform (3–5× the work).

**Verdict: Tauri v2, Cargo workspace, shared core crate.**

---

## 2. Reading firmware from a GitHub repository

Three distinct sources exist inside one repository, and the product must handle all three because different firmware projects use different ones (Marauder = Releases; Bruce = Releases + Pages; small projects = files committed in-tree).

### 2.1 Source A — files committed under `firmware/`
- `GET /repos/{owner}/{repo}/contents/firmware?ref={ref}` lists entries with `name`, `path`, `size`, `sha`, `download_url`.
- **Hard limit: the contents API refuses files larger than 100 MB and returns base64 only up to 1 MB.** Always fetch bytes from `download_url` (raw), never from the base64 `content` field.
- For a whole tree in one call: `GET /repos/{o}/{r}/git/trees/{sha}?recursive=1` (watch the `truncated` flag).

### 2.2 Source B — GitHub Releases (the primary real-world channel)
- `GET /repos/{o}/{r}/releases` and `/releases/latest`; assets carry `name`, `size`, `browser_download_url`, `content_type`, `updated_at`, `download_count`.
- Releases give us for free: **version, channel (`prerelease` flag), publication date, changelog body, and per-asset download counts** — i.e. every field on the M5Burner firmware card and Bruce's release selector.
- Asset download requires `Accept: application/octet-stream` when authenticated.

### 2.3 Source C — Actions artifacts (bleeding edge)
- `GET /repos/{o}/{r}/actions/artifacts`. **Requires authentication even on public repos, and artifacts expire (90 days default).** Treat as an opt-in "nightly" channel, clearly labelled.

### 2.4 Rate limits — a real product constraint
| Mode | Limit |
|---|---|
| Unauthenticated | **60 requests/hour per IP** |
| Authenticated (PAT / OAuth / device flow) | 5,000 requests/hour |
| Conditional requests (`ETag` / `If-None-Match`) | **304 responses do not count against the limit** |

**Requirements this generates:**
- R-GH-1: All catalogue reads MUST use conditional requests with stored `ETag`s and MUST respect `X-RateLimit-Remaining` / `Retry-After`.
- R-GH-2: The app MUST work unauthenticated for public repos, and MUST degrade gracefully (cached catalogue + clear "rate limited, retry at HH:MM" state) rather than erroring.
- R-GH-3: Optional sign-in via **OAuth device flow** (no embedded browser, no client secret on the client) to raise limits and reach private repos. Token stored in the OS keychain, never in a config file.
- R-GH-4: Downloaded artifacts MUST be content-addressed by SHA-256 in a local cache so re-flashing and rollback work fully offline.

### 2.5 The repository convention firmforge defines

The app must work with repos that know nothing about it (via Releases + heuristics), but should reward repos that opt in. Proposed convention, deliberately a **superset of the ESP Web Tools manifest** so the same file also powers a browser flasher:

```
firmware/
├── manifest.json          # ESP Web Tools-compatible + firmforge extensions
├── manifest.sig           # detached Ed25519 signature over manifest.json
├── CHANGELOG.md
├── boards/                # per-board metadata, images, download-mode instructions
│   └── m5stack-cardputer.json
└── builds/
    └── 1.16.0/
        ├── esp32s3/bootloader.bin, partitions.bin, boot_app0.bin, app.bin
        └── assets/        # SD-card / LittleFS payloads shipped with the build
```

firmforge extensions to the manifest (all optional, ignored by ESP Web Tools):
`channel` (stable|beta|nightly) · `releaseNotesUrl` · `sha256` per part · `signature` · `minFlashSize` / `psramRequired` / `minChipRevision` · `capabilities[]` (wifi, ble, subghz, nfc, ir, lora…) · `variant` (e.g. Bruce's `LITE_VERSION`) and what it drops · `assets[]` (files destined for SD/LittleFS with target paths) · `boardIds[]`.

---

## 3. Flashing on the desktop

### 3.1 The stack
| Layer | Crate | Version |
|---|---|---|
| Serial ports | `serialport` | 4.9 |
| ESP flashing protocol | `espflash` (library, not just the CLI) | 4.5 |
| Partition tables | `esp-idf-part` | 0.6 |
| BLE (desktop + mobile) | `btleplug` | 0.12 |
| GitHub API | `octocrab` | 0.54 |
| HTTP | `reqwest` | 0.13 |
| Hashing / signatures | `sha2` 0.11, `ed25519-dalek` 3.0 | |
| Archives | `zip` 8.6 | |

`espflash` as a **library** gives chip detection, stub loader upload, compressed writes, flash-size/mode/frequency handling, MD5 verification and monitor support — i.e. we do not reimplement the Espressif protocol.

### 3.2 Requirements
- R-FL-1: **Detect the chip before offering firmware.** Read chip family, revision, flash size, MAC and USB serial type; filter the catalogue to compatible builds only. (This is what eliminates Marauder's "compiled version not working" issue class.)
- R-FL-2: Support multi-part writes at explicit offsets (bootloader offset differs: `0x0` on C3/C6/H2, `0x1000` on ESP32/S2/S3) — never assume a single blob.
- R-FL-3: Distinguish **native USB-CDC** from **USB-to-UART bridge** connections and honour the manifest's `serialType` with fallback-to-unspecified semantics.
- R-FL-4: Offer **Update** (preserve NVS/settings partitions) vs **Erase & install clean**, described in consequences, not jargon.
- R-FL-5: **Pre-flight checks** that block with a clear reason: wrong chip, insufficient flash size, missing SD card where required, unstable/low battery, insufficient free space, unsupported chip revision.
- R-FL-6: **Snapshot before write** — read back NVS/config partitions to a local backup so a bad update is recoverable, and offer restore afterwards. (Directly answers Meshtastic's "keep my favourites" and M5Burner's export feature.)
- R-FL-7: **Verify after write** (MD5/SHA readback) and report a definitive success/failure, not just "100%".
- R-FL-8: **Recovery path always available** — document and detect the ROM download-mode entry per board; a failed flash must never be terminal. Surface this prominently ("you cannot brick this device").
- R-FL-9: **Serial console** as a first-class pane with configurable baud, timestamps, log save, and ANSI handling.
- R-FL-10: Progress must be **cancellable and resumable**, with per-part granularity.

### 3.3 Drivers and permissions — the number-one first-run failure
| Platform | Issue | Requirement |
|---|---|---|
| Windows | CH340/CH9102/CP210x/FTDI bridge drivers often missing | R-DRV-1: detect VID/PID of an unbound bridge and link the exact driver; consider bundling in the installer |
| macOS | Apple-silicon quirks; Gatekeeper; app must be **signed and notarised** | R-DRV-2: notarised builds, hardened runtime, correct entitlements |
| Linux | `/dev/ttyACM*` owned by `dialout`/`uucp`; users hit permission-denied (Bruce documents `setfacl` as a workaround) | R-DRV-3: detect the permission error specifically and offer the udev-rule / group fix inline, with a copyable command |
| All | Power-only USB cables | R-DRV-4: show the "not a power-only cable" warning **before** the connect attempt (Meshtastic's best single UX detail) |

---

## 4. Mobile — where the constraints bite

This is the section most likely to be underestimated, so it is deliberately blunt.

### 4.1 Android
- **The `serialport` crate does not work.** Android apps cannot open `/dev/tty*`; USB serial goes through the Java **USB Host API** (`UsbManager`, `UsbDeviceConnection`, `UsbEndpoint`), with per-device user permission granted through a system dialog, plus a `device_filter.xml` intent-filter for auto-launch on attach.
- **Requirement R-AND-1:** write a Tauri v2 Android plugin (Kotlin) exposing bulk transfer + control transfer for CDC-ACM and the common bridge chips (CH34x, CP210x, FTDI), and implement the espflash transport over that channel rather than over `serialport`. This is the single largest platform-specific work item in the project — plan it as its own milestone.
- USB-OTG requires a suitable cable/adapter; many phones supply limited bus power. **R-AND-2:** warn when the device draws more than the host can supply, and recommend a powered OTG hub for larger boards.
- BLE requires runtime `BLUETOOTH_SCAN` / `BLUETOOTH_CONNECT` permissions (API 31+) and, on older versions, location permission — which this privacy-sensitive audience dislikes. **R-AND-3:** request `neverForLocation` where possible and explain each permission at the point of use.
- **R-AND-4:** distribute via Play Store **and** F-Droid **and** direct APK on GitHub Releases (Meshtastic's 15-reaction request confirms the demand).

### 4.2 iOS
- **There is no general USB-serial access.** Arbitrary USB/UART devices are reachable only through the MFi ExternalAccessory programme; a generic ESP32 board is not MFi. Assume **no cable flashing on iOS, ever.**
- Therefore iOS is **BLE-only**: device discovery, config, console, and **OTA firmware update over BLE** to devices whose firmware supports it.
- Background BLE needs the `bluetooth-central` background mode; App Store review requires a clear justification.
- **R-IOS-1:** scope the iOS app as a *companion/monitor + BLE OTA* client, and say so in the product description rather than shipping a crippled flasher.
- **R-IOS-2:** Apple Developer Programme membership, provisioning profiles and TestFlight are hard prerequisites — a cost and calendar item, not a technical one.

### 4.3 OTA — the transport that makes mobile worth building
ESP32 OTA over BLE or WiFi requires cooperating firmware (an OTA service/endpoint). Two tiers:
- **Tier 1 (works everywhere, no firmware cooperation):** USB serial flashing — desktop always, Android with the plugin.
- **Tier 2 (requires OTA-capable firmware):** BLE/WiFi OTA — the only path on iOS, and the best path in the field.
**R-OTA-1:** the manifest must declare whether a build supports OTA and which OTA protocol, so the app knows which transports it can offer for a given firmware.

---

## 5. Trust, security and safety

No competitor does any of this. It is the cheapest credible differentiator available.

- R-SEC-1: **Verify SHA-256 of every downloaded part** against the manifest before writing a single byte.
- R-SEC-2: **Optional Ed25519 signature verification** of the manifest, with the publisher key pinned on first use (TOFU) and a visible change warning if it ever rotates.
- R-SEC-3: **Show provenance before flashing**: repository, owner, tag, commit SHA, workflow run, build timestamp. "Where did this binary come from" should be one glance, not an investigation.
- R-SEC-4: A **trust filter** ("verified publishers only") equivalent to M5Burner's "Only Official", but backed by cryptography rather than a self-asserted flag.
- R-SEC-5: **Loud, unmissable warnings for unsigned or unverifiable artifacts** — Bruce's App Store banner is the right tone; make it contextual and per-item.
- R-SEC-6: Secrets (GitHub token, WiFi credentials used for provisioning, device passwords) in the **OS keychain**; never in plain config; never in exported/shared artifacts. (ESPHome's `secrets.yaml` model.)
- R-SEC-7: **Legal/compliance framing.** Much of this firmware (Bruce, Marauder) is offensive-security tooling distributed under AGPL with explicit "authorised testing only" disclaimers. The app must surface each firmware's licence and disclaimer, and must not itself imply endorsement. This also affects app-store viability: **an app whose store listing emphasises WiFi deauth will have Play/App Store problems.** Position firmforge as a neutral *firmware manager*, with the catalogue supplied by the user.
- R-SEC-8: **Telemetry off by default**, stated plainly. This audience is the same one that asked Meshtastic to default location sharing to off and to publish on F-Droid.

---

## 6. Non-functional requirements

| ID | Requirement |
|---|---|
| R-NF-1 | Cold start to interactive < 2 s on a 5-year-old laptop |
| R-NF-2 | Desktop installer < 20 MB per platform |
| R-NF-3 | Full offline operation once a firmware is cached: browse cache, flash, console, rollback |
| R-NF-4 | Flash a 4 MB image over USB in < 60 s (requires high-baud negotiation + compressed writes) |
| R-NF-5 | Accessibility: keyboard-navigable, screen-reader labels, no colour-only status |
| R-NF-6 | i18n from v1 (top open ask on ESP Web Tools; Meshtastic ships a language picker) |
| R-NF-7 | Crash-free flashing: an app crash mid-write must leave a recoverable device and a resumable job |
| R-NF-8 | Self-update for the desktop app (Tauri updater), signed; app binaries published on GitHub Releases |
| R-NF-9 | CI matrix building Windows/macOS(arm+x64)/Linux/Android/iOS artifacts on every tag |
| R-NF-10 | Linux packaging tested on Wayland and X11 (qFlipper's recurring failure mode) |

---

## 7. Risk register

| Risk | Severity | Mitigation |
|---|---|---|
| Android USB-host plugin is larger than expected | **High** | Milestone it separately; ship desktop v1 first; prototype the Kotlin bridge before committing to a mobile date |
| iOS cannot flash over USB — perceived as a broken product | High | Set expectations in copy and store listing; make BLE OTA genuinely excellent |
| App-store rejection due to offensive-security firmware associations | High | Neutral positioning; no bundled attack tooling; user-supplied catalogues; clear disclaimers |
| GitHub rate limits degrade first-run experience | Medium | ETags, aggressive caching, optional sign-in, graceful rate-limited state |
| Bricking a user's device | Medium | Pre-flight checks, config snapshot, verify-after-write, prominent ROM recovery flow |
| Board/variant matrix explosion (the pain in every competitor) | Medium | Chip auto-detection + capability filtering; community-editable board metadata in the repo |
| Linux WebKitGTK rendering/packaging differences | Medium | CI smoke tests on Wayland + X11; avoid Chrome-only CSS |
| Upstream firmware repos change layout | Low | Heuristic Releases-based fallback that needs no cooperation; manifest is an optimisation, not a requirement |

---

## 8. Requirement summary — what "done" means for v1 desktop

1. Add a GitHub repo by URL; the app finds firmware in `firmware/`, Releases, or both.
2. Plug in a board; the app identifies the chip and shows only builds that fit it.
3. See version, channel, release date, changelog, provenance and verification status before doing anything.
4. Choose Update or Erase & install, with the consequence spelled out.
5. Config is snapshotted, parts are verified, the write is progressive and cancellable, the result is verified.
6. A serial console opens automatically afterwards.
7. The device is remembered: its history, its config backups, and a one-tap rollback to the previous known-good build.
8. All of it works with no network once cached, and with no GitHub account.
