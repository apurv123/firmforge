# Teardown: ESPHome

> "Your own custom firmware, without writing code" — Open Home Foundation / Nabu Casa — [esphome.io](https://esphome.io/) — [esphome/esphome](https://github.com/esphome/esphome)

---

## 1. Product description

ESPHome turns a YAML file into a compiled ESP32/ESP8266/RP2040/nRF firmware image and then manages that image for the lifetime of the device. It is the most operationally mature project in this research set: it owns **build, flash, OTA, logs, config and integration** as a single loop, and it is the reference implementation of "firmware as a managed fleet" for hobbyists.

Three delivery surfaces:
- **ESPHome Builder** — the dashboard, normally run as a Home Assistant add-on or a local Docker/CLI install. Full build + OTA + logs.
- **ESPHome Web** (`web.esphome.io`) — the browser-only "lite variant": connect an ESP over WebSerial, prepare it for first use, install new versions, and read device logs. Explicitly framed as *privacy-preserving*: "runs 100% in your browser, no data will leave your computer" (`02-web-esphome-io.png`).
- **CLI** — `esphome run`, `esphome logs`, etc.

**Why it matters to firmforge:** ESPHome already proves the core thesis — that the valuable product is not the flash operation but the *ongoing relationship* with the device (update, observe, reconfigure, recover). It also has the most instructive backlog in this entire study.

## 2. Features

- **Compile-from-config**: YAML → PlatformIO/ESP-IDF build → per-device firmware. Thousands of supported components (`04-components-index.png`).
- **OTA component** (`05-ota-component.png`): wireless updates as a normal, expected, everyday operation — with password/auth, safe-mode fallback, and rollback on failed boot.
- **Live logs** streamed over serial *or* over the network from an already-deployed device.
- **Dashboard import / adoption**: a device advertising itself can be "adopted" into your dashboard — near-zero-config onboarding (`09-dashboard-import.png`).
- **Native API** to Home Assistant, plus MQTT, plus a captive-portal/AP fallback when WiFi credentials fail.
- **Secrets management** (`secrets.yaml`) so credentials are never baked into shared configs.
- **Manufacturer/creator support**: vendors ship ESPHome-based products with an `esp-web-tools` install button and a `home_assistant_domain` hint in the manifest.

## 3. Workflows

**A. First install:** ESPHome Web → Connect (WebSerial) → Prepare for first use → device boots into a WiFi-provisioning AP → enter credentials → device joins network and appears in the dashboard.
**B. Iterate:** edit YAML → Install → compiles → **OTA push over WiFi** (no cable) → device reboots → logs stream automatically.
**C. Recover:** failed boot triggers **safe mode**, which re-opens the OTA path; captive portal fallback if WiFi is wrong.
**D. Adopt a vendor device:** vendor's site → ESP Web Tools button → flash → device advertises → dashboard offers "adopt" → user now owns the config.

The critical detail: **after the first cable-flash, ESPHome users almost never touch a cable again.** That is the standard firmforge should hold itself to.

## 4. Screenshots

| File | Page |
|---|---|
| `01-homepage.png` | esphome.io landing |
| `02-web-esphome-io.png` | **ESPHome Web — "Connect", "Not connected" state, privacy framing** |
| `03-guides-getting-started.png` | Getting started (Home Assistant path) |
| `04-components-index.png` | Component catalogue — the breadth advantage |
| `05-ota-component.png` | **OTA component docs — the update model** |
| `06-github-issues-top.png` | Core repo issues by reactions |
| `07-feature-requests.png` | **Dedicated feature-requests repo, sorted by demand** |
| `08-builder-docs.png` | Command-line / builder install |
| `09-dashboard-import.png` | Creators / dashboard import & adoption |

## 5. Top user feedback

ESPHome is unusual in maintaining a **separate `esphome/feature-requests` repo**, which produces the cleanest demand ranking available anywhere in this category:

| Reactions | State | Ask |
|---|---|---|
| **446** | closed | **Zigbee support on ESP32-H2 / C6 / C5** |
| **171** | open | **LoRaWAN support** |
| **140** | open | ESP32/ESP8266 WiFi **mesh** networking |
| 139 | open | **Matter** application-layer support |
| 131 | open | Bluetooth A2DP (audio sender) |
| 98 | open | **IPv6 support** |
| 94 | open | **ESP-NOW** integration |
| 90 | closed | Reconnect WiFi — scan for strongest AP |
| 85 | open | M5Paper board support |
| 81 | open | SD card as a data sink; Create native API server |
| 70 | closed | Automation between nodes |

And from the **core** repo (bugs, not wishes) — a very different and equally instructive list:

| Reactions | Ask |
|---|---|
| 48 | "Component xxxxxx took a long time for an operation" (the single most-hated diagnostic message) |
| 20 | Build toolchain breakage: `esp_idf_size: error: unrecognized arguments: --ng` |
| 13 | `#warning "legacy pcnt driver is deprecated"` |
| 12 | **Upgrade 25.12.6 → 26.1.1 broke Bluetooth proxies (ESP-IDF regression); reverting fixed it** |
| 11 | `ModuleNotFoundError: No module named 'idf_component_manager'` — can't compile |
| 11 | Upload error resolving IP address with MQTT + `manual_ip` |
| 8 | Add-on update broke compiling on HAOS |

**Reading the signal:**
1. **Radios and protocols are the growth axis** (Zigbee, LoRa, Matter, ESP-NOW, mesh, IPv6). A firmware catalogue app should treat *radio capability* as a primary filter/facet, not a footnote.
2. **The overwhelming majority of ESPHome's own pain is toolchain and upgrade pain.** Users get burned by a version bump and want to go back. This is the strongest possible argument for firmforge's **pinned versions, changelog-before-update, and one-tap rollback**.
3. **"It broke after the update, reverting fixed it"** is a 12-reaction issue in a repo of this size. Rollback is not a nice-to-have.
4. Notably absent from ESPHome's top asks: a mobile app — because Home Assistant's app already fills that slot. Confirms that *mobile matters, but only when it does something the desktop can't*: proximity, provisioning, and field work.

## 6. What firmforge should steal

1. **OTA as the default steady-state transport**, with cable flashing reserved for first install and recovery.
2. **Safe mode / recovery boot + automatic rollback on failed boot** — the single highest-leverage reliability feature in this category.
3. **Streaming logs** attached to the device, over both serial and network, as a permanent pane rather than a modal.
4. **Adoption/discovery**: devices on the network announce themselves and can be claimed. Zero-config onboarding.
5. **Secrets kept out of shared artifacts** — config templates with placeholders, values stored in an OS keychain.
6. **A public, reaction-ranked feature-request repo** as the product's own roadmap input. Cheap, and it works.
7. The privacy line — "runs entirely on your machine, nothing leaves your computer" — is a *feature statement* for a security-adjacent audience. A native app can make that claim more credibly than a web page.

## 8. Where firmforge differs

ESPHome is a *build system* with an app attached; firmforge is an *app* with a catalogue attached. We do not compile YAML. We consume artifacts someone else's CI already built, from a GitHub repo, and take responsibility for getting them onto a device safely and keeping them healthy. That means:

| ESPHome | firmforge |
|---|---|
| Requires a build environment (HA add-on, Docker, or Python toolchain) | Single signed binary, no toolchain, no Python |
| Targets home-automation devices you own and configure | Targets prebuilt community firmware you install and live with |
| No mobile client of its own (delegates to Home Assistant) | Mobile is a first-class client |
| Config-first | Device-first |
