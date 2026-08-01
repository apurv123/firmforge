# Teardown: Bruce Firmware

> "Predatory ESP32 Firmware" — AGPL-3.0 · 6,311★ · 236 open issues · [BruceDevices/firmware](https://github.com/BruceDevices/firmware) · [bruce.computer](https://bruce.computer/)
> *(note: the historic `pr3y/Bruce` URL now 301-redirects to `BruceDevices/firmware` — the project has been organisation-ised, a sign of commercial maturity.)*

---

## 1. Product description

Bruce is a multi-tool offensive-security firmware for ESP32-class handhelds — M5Stack (Cardputer, StickC, Core/Core2/CoreS3), Lilygo (T-Deck, T-Embed), Elecrow, CYD boards, and its own open-hardware "Bruce Boards". Its stated origin is explicit competitive positioning: the community wanted Flipper Zero capability "without being that overpriced," on the modular ESP32 hardware ecosystem that already existed.

Functionally it bundles WiFi attacks, BLE, RF/sub-GHz (CC1101), NFC/RFID, IR, a JavaScript interpreter, TelNet/SSH/WireGuard clients, and a growing games/utilities layer.

**Why it matters to firmforge:** Bruce is not just firmware — it has grown an entire *distribution surface* around itself (web flasher, app store, theme builder, device manager, board shop, forum). It is the most complete demonstration of the problem firmforge wants to own, and simultaneously the clearest demonstration that this surface is currently **web-only and desktop-only**.

## 2. Distribution surface (the actual competitor)

| Surface | URL | What it is |
|---|---|---|
| **Web Flasher** | `bruce.computer/flasher` | ESP Web Tools–powered WebSerial installer |
| **App Store** | `bruce.computer/appstore` | Catalogue of ~50 community apps + themes |
| **Theme Builder** | `bruce.computer/build_theme.html` | In-browser theme authoring |
| **Bruce Lab** | `bruce.computer/my_bruce` | WebSerial device manager ("Connect") |
| **Boards** | `bruce.computer/boards` | Open hardware + shop |
| **Wiki / FAQ** | `wiki.bruce.computer` | Docs |
| **Community** | Discord, Matrix, Reddit, forum | Support |

## 3. Features

**Firmware**
- Device-tiered feature matrix. A `LITE_VERSION` deliberately drops TelNet, SSH, WireGuard, ScanHosts, RawSniffer, Brucegotchi, BLEBacon, BLEScan and the Interpreter — for **M5Launcher compatibility** (i.e. to fit a smaller partition). This is a real product decision firmforge must model: *the same release has capability-differentiated variants per device.*
- Modular capability set: WiFi / BLE / RF / RFID-NFC / IR / scripting.
- JS interpreter → user-authored scripts as first-class content.

**Web Flasher** (screenshot `03-web-flasher.png`) — the single best reference screen in this whole research set:
- **Release channel selector**: `Latest 1.16` / `Beta` / `Other`, with the release timestamp shown inline ("Released: 24/07/2026 06:35").
- **Manufacturer/category segmentation**: Bruce Boards · M5Stack · Lilygo · Elecrow · CYD · ESP32 · Custom Boards · Launcher.
- **Inline, per-device download-mode instructions** — e.g. Cardputer: "hold btn G0, then connect via USB"; StickC: "jumper GND↔G0, plug USB, remove jumper"; T-Embed: "hold encoder centre + press RST". Plus a Linux `setfacl -m u::rw /dev/ttyACM0` note.
- Powered by ESP Web Tools, explicitly credited.

**App Store** (screenshot `07-app-store.png`):
- Grid of apps/themes with icon, name, size in bytes, **author**, **version**, and one-line description.
- Filters: category chips (`All`, `Audio`, `Games`, `Infrared`, `RF`, `Themes`, `Tools`, `Utilities`, `WiFi`) plus an "All Devices" device dropdown and free-text search.
- A prominent **security warning banner**: "Only trust open-source scripts you can verify… never pay for scripts, firmware forks or themes," with rules — always read the code before executing, watch for scams selling "premium" scripts, official resources only via Bruce App Store, report suspicious activity.
- **Installation is manual**: "To install apps or themes, put files on the SD card/LittleFS — Apps: `/Bruce/…`, Themes: `/Bruce/themes/…`".

**Bruce Lab** (screenshot `08-bruce-lab.png`): a nearly-empty page with a single **Connect** button — a WebSerial device console. Functional, but conspicuously undeveloped compared to the rest of the site.

**Other install paths**: `esptool.py --port /dev/ttyACM0 write_flash 0x00000 Bruce-<device>.bin`; **OTA via M5Launcher**; or **M5Burner** (search "Bruce"; official builds are uploaded by "owner" and have photos).

## 4. Workflows

**A. First install (happy path)**
`bruce.computer` → Install → read flashing instructions → pick release channel (Latest/Beta/Other) → pick manufacturer → pick device → put board into download mode (per on-page instructions) → Connect (WebSerial port picker) → Flash → device reboots into Bruce.

**B. Update**
Either repeat A, or — on M5Stack only — OTA from M5Launcher. There is no update *notification*; the user has to go looking.

**C. Install an app or theme** (the weak link)
Browse App Store → filter by device/category → download file → **physically remove SD card or mount LittleFS** → copy into `/Bruce/` or `/Bruce/themes/` → reinsert → find it in the on-device menu. Every step after "download" is manual and off-app.

**D. Theme it**
Theme Builder in browser → produce theme file → same manual copy as C.

**E. Get help**
Wiki → FAQ → Discord/Matrix/forum.

## 5. Screenshots

| File | Page |
|---|---|
| `01-homepage.png` | bruce.computer landing |
| `02-github-repo.png` | GitHub repo (BruceDevices/firmware) |
| `03-web-flasher.png` | **Web Flasher — release channel + device matrix + download-mode instructions** |
| `04-docs-features.png` | Wiki entry |
| `05-issues.png` | Issues sorted by 👍 |
| `06-releases.png` | Releases / artifact naming |
| `07-app-store.png` | **App Store — cards, filters, security banner, manual install note** |
| `08-bruce-lab.png` | Bruce Lab (WebSerial "Connect") |
| `09-boards.png` | Open hardware / boards |
| `10-wiki.png` | wiki.bruce.computer |
| `11-theme-builder.png` | Theme Builder |

## 6. Top user feedback (GitHub issues by 👍 reactions)

| 👍 | State | Ask |
|---|---|---|
| 22 | open | Support for BW16 (RTL8720DN) dual-band WiFi module via Serial/GPIO |
| 8 | open | M5Stack Tab5 support |
| 7 | closed | Gyroscope navigation for keyboard (StickC Plus2 / StickS3) |
| 6 | open | Cardputer ADV doesn't find NRF24 |
| 6 | closed | Fixes for M5Stack StickS3 |
| **4** | **open** | **"Full-Featured Mobile App for Bruce (Control, App Store, Settings)"** |
| 4 | open | CCTV Toolkit |
| 4 | closed | LoRa functionality; T-Dongle S3 support; airmouse via gyroscope; RFID Mifare Classic 1K read failures |

**Reading the signal:** the demand is (a) *more boards, faster* and (b) an explicit, unbuilt **mobile app that combines control + app store + settings** — which is, almost word for word, the firmforge mobile product. Note also the long tail of "device X isn't detected / peripheral Y broke in version Z", which argues for a **device-and-version-aware compatibility matrix** rather than a flat firmware list.

## 7. What firmforge should steal

1. **Release channel selector as a first-class control** (Stable / Beta / Any tag) with release date shown. Copy this exactly.
2. **Manufacturer → device drill-down**, not a flat 200-item board list.
3. **Per-device download-mode instructions rendered at the moment of need** — with illustrations. This is the highest-value piece of content on the entire site.
4. **Capability-differentiated variants of one release** (`LITE_VERSION`) as a data model concept, surfaced in UI as "what you lose if you pick this build."
5. **The App Store security banner** — a curated catalogue must state its trust model loudly.
6. Credit-where-due: build on ESP Web Tools' manifest so Bruce's own catalogue could be consumed by firmforge on day one.

## 8. Where firmforge wins

| Bruce gap | firmforge answer |
|---|---|
| App/theme install requires manually copying files to SD/LittleFS | Atomic install bundle: firmware + assets pushed over the same USB/BLE session, no card removal |
| Bruce Lab is a bare "Connect" button | A real device workspace: serial console, file browser, config backup/restore, install history |
| Web-only ⇒ **no phone support at all** | Native Android (USB-OTG + BLE) and iOS (BLE/MFi) — the top open feature request |
| No update notifications; user must re-visit the site | Watch the GitHub repo, notify on new release, one-tap update with changelog |
| Unsigned binaries, "trust us" banner | Cryptographic signature + provenance chain (repo, commit, workflow run) shown before flashing |
| Knowledge lives in Discord | Device-aware in-app troubleshooting keyed to chip + error |
