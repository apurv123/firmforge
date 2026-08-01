# Teardown: ESP Web Tools

> "Allow flashing ESPHome or other ESP-based firmwares via the browser. Will automatically detect the board type and select a supported firmware."
> [esphome.github.io/esp-web-tools](https://esphome.github.io/esp-web-tools/) — [esphome/esp-web-tools](https://github.com/esphome/esp-web-tools) — Open Home Foundation

---

## 1. Product description

ESP Web Tools is not an application — it is a **web component and a file format**. Vendors and firmware authors drop `<esp-web-install-button manifest="firmware/manifest.json">` onto a page and get a working WebSerial installer for free.

It is the most strategically important item in this research set, because it quietly became the **interoperability standard**: Bruce's flasher is built on it ("Installer powered by ESP Web Tools", credited in the footer of `bruce-firmware/03-web-flasher.png`), ESPHome Web is built on it, and a long tail of commercial ESP products ship an install button using it.

**Why it matters to firmforge:** if firmforge speaks `manifest.json` natively, it can install *any* firmware in this ecosystem on day one, with zero cooperation from the firmware author — and any repo that adds a manifest for firmforge simultaneously gets a free web flasher. Adopting the format is close to a free network effect.

## 2. The format (the actual asset)

```json
{
  "name": "ESPHome",
  "version": "2021.10.3",
  "home_assistant_domain": "esphome",
  "funding_url": "https://esphome.io/guides/supporters.html",
  "builds": [
    {
      "chipFamily": "ESP32",
      "parts": [
        { "path": "bootloader_dout_40m.bin", "offset": 4096 },
        { "path": "partitions.bin",          "offset": 32768 },
        { "path": "boot_app0.bin",           "offset": 57344 },
        { "path": "esp32.bin",               "offset": 65536 }
      ]
    },
    { "chipFamily": "ESP32-C3",  "parts": [ /* … offset 0 bootloader … */ ] },
    { "chipFamily": "ESP32-S2",  "parts": [ /* … */ ] },
    { "chipFamily": "ESP32-S3", "serialType": "uart", "parts": [ /* … */ ] },
    { "chipFamily": "ESP32-S3", "serialType": "cdc",  "parts": [ /* esp32-s3-cdc.bin */ ] },
    { "chipFamily": "ESP8266",   "parts": [ { "path": "esp8266.bin", "offset": 0 } ] }
  ]
}
```

Design points worth internalising:

- **`chipFamily` drives automatic build selection.** The tool reads the chip over serial, then picks the matching build. The user never chooses a `.bin`.
- **`parts[]` is an ordered list of `{path, offset}`** — firmware is explicitly *multi-part*, not one blob. Note the C3's bootloader at offset `0` vs. `0x1000` on ESP32/S2/S3: exactly the footgun that kills manual `esptool` users.
- **`serialType: "cdc" | "uart"`** disambiguates native-USB-CDC boards from USB-to-UART-bridge boards for chips that support both (notably ESP32-S3). Builds without the field act as a **fallback for any connection type** — a well-designed graceful-degradation rule.
- **`version`** enables update comparison; **`funding_url`** and **`home_assistant_domain`** show that the manifest is a place for *product metadata*, not just bytes.

What the format **lacks**, and what firmforge should add (backwards-compatibly, as optional keys): signatures/checksums, changelog/release-notes URL, minimum-bootloader or hardware-revision constraints, required flash size / PSRAM, non-binary assets (SD/LittleFS payloads), release channel, and capability tags.

## 3. Features and limits

**Features:** automatic chip detection; automatic build selection; erase-vs-update choice; improv-serial WiFi provisioning after install; log/console view; a drop-in custom-element API.

**Hard limits — all of them are firmforge's opening:**
- **WebSerial only** ⇒ **Chrome / Edge / Opera on desktop only**. No Safari. No Firefox. **No mobile browser on any platform.**
- No background updates, no notifications, no install history.
- No offline use — the page and the binaries must be fetched live.
- No signature verification.
- Serial speed is fixed (an open feature request).

## 4. Workflows

**A. End user:** open the vendor's page → click Install → browser serial-port picker → chip detected → matching build selected automatically → erase/update choice → progress bar → optional WiFi provisioning via improv → done.
**B. Firmware author:** run CI → publish `.bin` parts + `manifest.json` to GitHub Pages or a release → embed one HTML tag.

## 5. Screenshots

| File | Page |
|---|---|
| `01-landing.png` | Landing page — live install button + documentation |
| `02-github-repo.png` | **Repo README containing the canonical manifest schema** |
| `03-issues-top.png` | Issues by reactions |

## 6. Top user feedback

| Reactions | State | Ask |
|---|---|---|
| 8 | closed | Add support for LOLIN S2 Mini |
| 5 | **open** | **Language localisation** (plus a duplicate "is it possible to add a translation?") |
| 3 | closed | Pass a JSON manifest **object** directly instead of a file path |
| 3 | **open** | **Installation failed on Chrome@macOS** |
| 2 | **open** | **Allow serial speed to be increased** |
| 1 | **open** | **Chrome and Edge crash with esp-web-tools (including the demo)** — plus a separate "crashes Chrome on macOS" |
| 1 | open | No firmware for ESP32-H2 |
| 1 | open | "Serial port is not ready" |
| 1 | open | JS callback when installation completes |
| 0 | closed | Flash mode and MCU clock control |

**Reading the signal:** the complaint list is dominated by **browser-platform failures the project cannot fix** — Chrome/Edge crashes, macOS serial failures, "serial port is not ready", fixed baud rate, no low-level flash-mode control. These are not bugs in the product; they are the ceiling of WebSerial. A native Rust implementation using real serial APIs is immune to every single one, can negotiate higher baud rates, and can expose flash mode/clock. **Localisation** being a top open ask is also notable — a native app with proper i18n is an easy win.

## 7. What firmforge should steal

1. **Consume `manifest.json` verbatim** as an input format — instant compatibility with Bruce, ESPHome and the vendor long tail.
2. **Emit `manifest.json`** from the firmforge repo convention, so any firmforge-published firmware also works in a browser.
3. **Automatic chip detection → automatic build selection.** Never make the user pick a `.bin`.
4. **The `serialType` cdc/uart discriminator with fallback semantics** — copy the rule exactly.
5. **improv-serial / improv-BLE WiFi provisioning** immediately after install, as a built-in step rather than a separate chore.
6. **Erase vs. update** presented as an explicit, explained choice (keep settings or start clean).

## 8. Positioning statement

> ESP Web Tools is the protocol. firmforge is the client that runs everywhere the browser can't — including your phone — and adds the things a web page structurally cannot: signatures, offline caching, update notifications, rollback, and device history.
