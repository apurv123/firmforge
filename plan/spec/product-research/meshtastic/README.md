# Teardown: Meshtastic

> Open-source, off-grid, decentralised LoRa mesh — [meshtastic.org](https://meshtastic.org/) — [meshtastic/firmware](https://github.com/meshtastic/firmware) — Meshtastic® is a registered trademark of Meshtastic LLC

---

## 1. Product description

Meshtastic is the best-executed **desktop + mobile + web trio** in hobbyist embedded, and the strongest existing proof of the firmforge thesis: that a firmware project needs a *client* on every surface its users actually hold.

The ecosystem comprises:
- **Meshtastic Web Flasher** (`flasher.meshtastic.org`) — a polished three-step WebSerial installer.
- **Meshtastic Web Client** (`client.meshtastic.org`) — browser-based device management.
- **Android app** (Play Store + F-Droid) and **iOS app** — full node configuration and messaging over **BLE, USB-OTG serial, or WiFi/TCP**.
- **Meshtastic CLI / Python API**, plus a **Site Planner** for RF range prediction.

Crucially, the phone is not a lesser client here — for most users it *is* the primary interface, because the device lives in a backpack on a mountain and the laptop does not.

## 2. UI teardown — Web Flasher (`02-web-flasher.png`)

The cleanest flashing UX found anywhere in this research, and the direct model for the firmforge install screen. Three numbered cards, left to right, with **progressive enablement** (steps 2 and 3 are visibly greyed until step 1 completes):

| Step | Card | Control | Helper text |
|---|---|---|---|
| **1** | **DEVICE** — illustration of the actual node | `Select Target Device` (primary) + a small "auto-detect" rocket button | "Plug in your device via USB. **Please ensure the cable is not a power-only one.**" |
| **2** | **FIRMWARE** — download-folder icon | `Select Firmware ▾` + a "open local file" button | "Choose from the release options **or upload a release zip downloaded from GitHub**." |
| **3** | **FLASH** — lightning icon | `Flash` | "Choose whether you wish to **update your device or wipe the flash and install from scratch**." |

Below: a persistent utility bar — `Open Serial Monitor >_`, tooling menu, `Meshtastic Docs`, `Contribute on GitHub`, a **language selector**, and a light/dark toggle.

Details worth stealing verbatim:
- **The power-only-cable warning is inline, before the failure**, not in a FAQ. This one sentence prevents the single most common support ticket in the category.
- **Update vs. wipe is stated in plain language** at the point of decision, in terms of consequence ("install from scratch"), not jargon ("erase flash").
- **Local-file escape hatch alongside the online catalogue** — power users can always sideload a release zip.
- **Serial monitor is a peer of flashing**, promoted in the same chrome.
- **Localisation is built in.**

## 3. Features

- Device catalogue with per-board firmware selection; official releases pulled from GitHub Releases (`08-firmware-releases.png`).
- Update vs. full-erase install modes.
- Serial monitor in-browser.
- **Mobile**: BLE pairing, USB-OTG serial, and WiFi/TCP transports; node list; map; messaging; full LoRa/module configuration; remote admin of other nodes.
- Roles and fleet semantics (`CLIENT`, `ROUTER`, `CLIENT_BASE`, favourites) — genuine multi-device management.
- Firmware releases are versioned with alpha/beta/stable channels.
- Site Planner for RF planning.

## 4. Workflows

**A. First flash:** web flasher → select target device → select firmware version (or upload a GitHub release zip) → choose update or wipe → flash → serial monitor confirms boot.
**B. Configure:** open the phone app → pair over BLE → device settings, channels, region, role → changes applied live.
**C. Field operation:** phone in pocket, node on a pole; messaging, node list and telemetry via BLE. **The desktop is not present.**
**D. Update in the field:** phone app warns about outdated firmware; user updates when back at a machine (still a gap — see feedback).
**E. Remote admin:** configure *other* nodes across the mesh without physical access.

## 5. Screenshots

| File | Page |
|---|---|
| `01-homepage.png` | meshtastic.org landing |
| `02-web-flasher.png` | **Web Flasher — the three-step DEVICE → FIRMWARE → FLASH pattern** |
| `03-downloads.png` | Software hub |
| `04-android-app-docs.png` | Android app usage docs |
| `05-github-firmware-issues.png` | Firmware issues by reactions |
| `06-web-client.png` | Web client |
| `07-android-download.png` | Play Store listing |
| `08-firmware-releases.png` | **GitHub Releases — the artifact source the flasher consumes** |

## 6. Top user feedback

**Firmware repo (`meshtastic/firmware`):**

| Reactions | State | Ask |
|---|---|---|
| 33 | closed | `CLIENT_BASE` mode: ROUTER for favourites, CLIENT otherwise (attic/roof nodes) |
| 26 | **open** | New message type for **EWS (Emergency Warning Service)** |
| 26 | closed | **Updated device screen UI modes for all devices** |
| 22 | **open** | `CLIENT_BASE` should not decrement hop count for favourites |
| 21 | closed | **Option to turn off LED indicators** |
| 16 | closed | Buzzer is all-or-nothing; T-Lora Pager board support |
| 16 | **open** | Define a first hop on channel messages |
| 15 | **open** | Enhance telemetry algorithm |
| 14 | closed | **When NodeDB is reset, keep favourite nodes** |
| 13 | open | Expiry on router roles, falling back to `router_late` |
| 12 | closed | Make mandatory-rebroadcast roles settable only via remote admin |

**Android app (`meshtastic/Meshtastic-Android`):**

| Reactions | State | Ask |
|---|---|---|
| 20 | closed | Duty-cycle override checkbox disappeared from LoRa settings |
| 19 | closed | Show "hops away" |
| 17 | closed | **Android Auto compatibility** |
| 15 | closed | **Publish to F-Droid** |
| 14 | closed | Flag messages as RF vs MQTT |
| **13** | closed | **Warn users of old firmware version** |
| 11 | closed | Filter node list/map by time last heard |
| 10 | closed | **Android widgets**; DNS support for node connection |
| 9 | closed | Default "provide phone location to mesh" to **off** |
| 8 | closed | Show nodes seen without NodeInfo |
| 7 | closed | Graphical signal strength/quality |

**Reading the signal:**
1. **"Warn users of old firmware version" (13, from the *mobile app*)** is the single most on-point data point in this entire research set. Users want the phone to tell them their device is out of date. firmforge mobile should ship this on day one.
2. **"When NodeDB is reset, keep favourites" (14)** — users are terrified of losing device state across updates. Config backup/restore around flashing is a top-tier feature, not a v3 nicety.
3. **"Publish to F-Droid" (15)** — this audience is privacy-conscious and distrusts app-store-only distribution. Plan for F-Droid/APK distribution alongside Play.
4. **Privacy defaults matter** ("location sharing should default to off", 9).
5. **Device-screen UI and LED/buzzer control (26, 21, 16)** show that users want the *companion app to control on-device presentation*, not just flash it.
6. Discoverability asks — widgets, Android Auto, filtering, signal graphs — show a mature mobile client being pushed toward ambient, glanceable use.

## 7. What firmforge should steal

1. **The three-step progressive-enablement flasher layout** (DEVICE → FIRMWARE → FLASH), including greying out later steps.
2. **The power-only-cable warning**, inline, verbatim in spirit.
3. **Update vs. wipe framed by consequence**, with an explicit "keep my settings" implication.
4. **Local release-zip sideload** next to the online catalogue.
5. **Serial monitor promoted to peer status** with flashing.
6. **Built-in language selector** from v1.
7. **Mobile-first firmware staleness warnings** — push notification when a watched repo publishes a newer release than what is on the device.
8. **Config backup/restore as part of every flash**, with favourites/state preserved across wipes.
9. **F-Droid + direct APK distribution** alongside the Play Store.

## 8. Where firmforge wins

| Meshtastic gap | firmforge answer |
|---|---|
| Flashing is still web/desktop-only; the phone can configure but not (re)flash | **Android USB-OTG flashing** — flash in the field, no laptop |
| Single-project catalogue (Meshtastic firmware only) | Any GitHub repo with a firmware folder becomes a catalogue |
| Firmware staleness warning had to be requested; no proactive channel | Watched repos + release notifications, built in |
| No signature/provenance verification of downloaded releases | Signature + provenance chain shown pre-flash |
| Update/wipe choice, but no automatic config snapshot | Snapshot device config before every write; restore after |
