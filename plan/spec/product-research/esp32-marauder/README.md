# Teardown: ESP32 Marauder

> "A suite of WiFi/Bluetooth offensive and defensive tools for the ESP32" — 11,752 stars, 291 open issues — [justcallmekoko/ESP32Marauder](https://github.com/justcallmekoko/ESP32Marauder) — hardware sold at [justcallmekokollc.com](https://www.justcallmekokollc.com)

---

## 1. Product description

Marauder is the most-starred project in this research set and the de-facto standard ESP32 wireless-security firmware. It runs on a wide range of ESP32 / ESP32-S2 / ESP32-S3 boards and on purpose-built Marauder hardware sold by the author (a real hardware business, not just a repo). It also has a strong **Flipper Zero symbiosis**: a large share of its userbase runs Marauder as a Flipper GPIO module, using `flipper_sd_serial` builds that read config from — and write PCAPs to — the *Flipper's* SD card.

**Why it matters to firmforge:** Marauder is the control experiment. It has roughly twice Bruce's stars and **no companion app, no app store, and no official web flasher** — distribution is "download the right `.bin` from Releases and figure it out." The consequences are visible throughout its issue tracker, and they are exactly the pains firmforge exists to remove.

## 2. Distribution surface

| Surface | Status |
|---|---|
| Official web flasher | **None.** `justcallmekoko.github.io/ESP32Marauder/` returns a GitHub Pages **404** (`03-web-flasher.png` retained as evidence); `/webflash` and `/flasher` also 404, and the repo has **no homepage field set**. |
| GitHub Releases | The primary channel — a large matrix of per-board `.bin` files (`04-releases.png`) |
| Wiki | Hand-written flashing / updating instructions, one section per method (`02`, `06`, `07`) |
| SD-card update | Firmware `.bin` copied to SD, applied from the on-device menu |
| esptool / Arduino IDE / ESP32 Flash Download Tool | The documented "real" paths |
| Third-party flashers | Community-maintained, unaffiliated, variable trust |
| Community | Gitter, Twitter/Instagram, YouTube |

CI is mature — a `Build and Push` GitHub Actions badge sits at the top of the README, so **artifacts are produced automatically and dumped into Releases**. The build side is solved; only the *last mile to the device* is not. That is exactly the shape of problem a GitHub-backed installer app fits.

## 3. Features

- **WiFi:** AP and station scanning, probe/beacon sniffing, packet monitor with live signal graph, deauth, beacon spam, evil portal / evil twin captive portals, PCAP capture to SD.
- **Bluetooth:** BLE scanning, BLE spam, sniffing, device detection (including "detect Flipper/AirTag"-class trackers).
- **Storage/IO:** SD card for PCAPs, captive-portal HTML and config; serial CLI over USB; GPIO/serial bridge to Flipper Zero.
- **Build variants:** a large per-board release matrix, plus special variants (`flipper_sd_serial`, display-specific builds, "old hardware" builds). The variant explosion is itself a UX problem — see feedback.

## 4. Workflows

**A. First install — the painful one**
Identify your exact board and display → open Releases → scan a long list of similarly named `.bin` files → pick the right one (and the right *variant*) → install esptool or Arduino IDE or the Espressif Flash Download Tool → work out the correct flash offsets → put the board in download mode → flash → hope.

**B. Update**
Two paths: (i) repeat A entirely, or (ii) copy the new `.bin` to the SD card and trigger the update from the device's own menu. Path (ii) is genuinely good UX *once the device is already working*, and is a pattern worth stealing for large assets.

**C. Flipper Zero usage**
Flash a `flipper_sd_serial` build → attach the ESP32 to the Flipper's GPIO header → drive Marauder from the Flipper app → captures land on the Flipper's SD card.

**D. Get help**
Wiki → GitHub issues → Gitter. The wiki is thorough but is a wall of text; there is no interactive "which board do I have?" flow.

## 5. Screenshots

| File | Page |
|---|---|
| `01-github-repo.png` | Repo landing — README, badges, CI status |
| `02-wiki-home.png` | Wiki entry point |
| `03-web-flasher.png` | **GitHub Pages 404 — evidence that no official web flasher exists** |
| `04-releases.png` | **Releases page — the per-board `.bin` matrix users must navigate** |
| `05-issues-top.png` | Issues sorted by reactions |
| `06-wiki-flashing.png` | Wiki: update-firmware instructions |
| `07-wiki-updating-sd.png` | Wiki: updating via SD card |
| `08-hardware-store.png` | Official hardware store |

## 6. Top user feedback (GitHub issues by reactions)

| Reactions | State | Ask |
|---|---|---|
| 12 | closed | Cardputer ADV support |
| 10 | **open** | **BLE Spam — more settings and features** |
| 8 | closed | `flipper_sd_serial` compiled version not working |
| 7 | closed | Evil portal / evil twin attack |
| 5 | **open** | **CC1101 (sub-GHz) compatibility** |
| 5 | closed | Evil Twin Attack; Evil Portal reading config + captive-portal HTML from Flipper SD |
| 4 | closed | Include IR remote / allow attaching Flipper sub-GHz GPIO module or a CC1101 |
| 3 | **open** | **WiFi camera detector** |
| 3 | closed | Empty PCAP with SD-serial support on ESP-WROOM-32; ESP32-S2 (Lolin S2 Mini) support |

**Reading the signal:**
1. **"The compiled version doesn't work" and "empty PCAP on my board" are distribution failures, not firmware failures.** They are what happens when a user picks the wrong artifact from an undifferentiated list. A companion app that detects the chip over USB and offers *only* compatible builds eliminates this class of issue outright.
2. **Board-support requests dominate** (Cardputer ADV, Lolin S2 Mini, …) — same as Bruce. Board coverage is the currency of this category.
3. **Peripheral/hardware-extension awareness** (CC1101, NRF24, IR) recurs. Users think in terms of "my device plus these modules", so a companion app should model **device = board + attached modules**, and gate builds on that.

## 7. What firmforge should steal

1. **SD-card / on-device update path** as a supported transport for large payloads and for devices already in the field.
2. **Serial CLI as a first-class product surface** — Marauder's CLI is heavily used; a good console pane matters more here than in consumer IoT.
3. **The Flipper-as-host pattern**: recognise that a "device" may be a peripheral to another device. The data model should not assume one board, one USB port.

## 8. Where firmforge wins

| Marauder gap | firmforge answer |
|---|---|
| No official flasher at all — 404 where one should be | Native installer as the default distribution surface; works offline and on all three desktop OSes |
| Users must self-select from a large ambiguous `.bin` matrix | **Auto-detect chip family and USB serial type, then show only compatible builds** (ESP Web Tools' `chipFamily` + `serialType` fields make this mechanical) |
| Variant confusion causes "it doesn't work" issues | Show variant differences explicitly (what each build includes/excludes) before flashing |
| No mobile story whatsoever | Android USB-OTG flashing plus BLE console |
| SD-card assets (portals, configs, PCAPs) handled by physically moving the card | In-app file manager over serial/BLE; pull PCAPs off the device to phone or desktop |
| Support knowledge trapped in wiki prose and Gitter | Guided "identify my board" flow and error-code-keyed troubleshooting |
