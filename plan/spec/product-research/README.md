# firmforge — Competitive Product Research

Teardowns of the products that already solved parts of "get firmware from the internet onto my ESP32-class device, then live with that device."
Every folder contains a `README.md` teardown and a `screenshots/` directory captured with Playwright (headless Chromium, 1440×900, full-page).

**Captured:** 2026-08-01 · **Method:** `tools/research/capture.mjs` + GitHub REST API (`search/issues?sort=reactions`) for user-demand signals.

## Competitor set and why each was chosen

| # | Product | Folder | Why it's in the set |
|---|---------|--------|---------------------|
| 1 | **Bruce** (BruceDevices/firmware) | [`bruce-firmware/`](bruce-firmware/) | Explicitly requested. 6.3k★. The single closest analogue: GitHub-hosted firmware + web flasher + app store + theme builder + "Bruce Lab" device manager. |
| 2 | **ESP32 Marauder** | [`esp32-marauder/`](esp32-marauder/) | Explicitly requested. 11.8k★, the biggest ESP32 offensive-security firmware — and deliberately *has no companion app*. The clearest "unserved demand" case. |
| 3 | **ESPHome** | [`esphome/`](esphome/) | Explicitly requested ("esp32home"). The gold standard for firmware lifecycle: build → flash → OTA → logs, with a 446-reaction feature-request backlog that reads like a roadmap. |
| 4 | **ESP Web Tools** | [`esp-web-tools/`](esp-web-tools/) | Independent contender. Not an app — a *primitive*. Its `manifest.json` is the de-facto industry standard for "here is my firmware"; Bruce, ESPHome and dozens of vendors all use it. We should adopt/extend it. |
| 5 | **M5Burner** | [`m5burner/`](m5burner/) | Independent contender. The only true **native desktop firmware store** in this space (Win/macOS/Linux), with publish/export/share. Closest UI analogue to firmforge desktop. |
| 6 | **Meshtastic** | [`meshtastic/`](meshtastic/) | Independent contender, and the best-executed **desktop + mobile + web trio** in hobbyist embedded. Proves the mobile-companion thesis (BLE/USB-OTG config on the phone). |
| 7 | **Flipper Zero / qFlipper** | [`flipper-zero/`](flipper-zero/) | Independent contender. The commercial benchmark for polish: signed OTA, an app catalogue with per-firmware-version compatibility gating, and phone-first updates over BLE. Bruce exists *because* of Flipper's price. |

## Cross-cutting conclusions

These are the findings that survived across all seven teardowns and drive the spec.

1. **The web flasher won the "first flash" battle; nobody won the "second week" battle.** Every product has a decent WebSerial installer. Almost none give you an ongoing relationship with the device — update notifications, rollback, config backup, per-device history. That gap is firmforge's wedge.
2. **`manifest.json` (ESP Web Tools) is the interop layer.** Chip family → ordered list of `{path, offset}` parts, plus `version`, `name`, `funding_url`, and the newer `serialType: "cdc" | "uart"` discriminator. Consuming *and emitting* this format buys instant compatibility with the whole ecosystem.
3. **WebSerial's ceiling is the opportunity.** Chrome/Edge desktop only — no Safari, no Firefox, and **no mobile browser**. Every web flasher in this set is structurally incapable of serving a phone. A native Rust app with USB-OTG + BLE has no competitor on Android.
4. **Assets beyond the .bin are an unsolved mess.** Bruce's App Store tells users to *manually copy files to the SD card or LittleFS*; Marauder ships SD-card update paths; Flipper solved it with qFlipper's file manager. Firmware ≠ one binary. Treat "firmware + assets + config" as one atomic install unit.
5. **Users repeatedly ask for the same five things** (evidence in each teardown): a real **mobile app**, **new board support**, **OTA/wireless update**, **serial log/console access**, and **not losing my settings on update**.
6. **Trust is unaddressed.** Every one of these tools happily flashes an unsigned binary downloaded over HTTPS from a GitHub Actions artifact. Nobody verifies a signature or shows a provenance chain. For a red-team-adjacent tool this is a glaring, and easily won, differentiator.
7. **Community lives on Discord, and knowledge dies there.** Bruce, Marauder and Meshtastic all route support to Discord. In-app contextual docs + a device-aware troubleshooting flow is disproportionately valuable.

## Capture log

Machine-readable record of every URL captured: [`capture-log.json`](capture-log.json).
