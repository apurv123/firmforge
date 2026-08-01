# apps/mobile

The Tauri v2 mobile shell (Android + iOS). Scheduled for **M5 — Pocket** and **M6 — Wireless** in [the spec's roadmap](../../plan/spec/firmforge-spec.md#11-roadmap).

It is intentionally not scaffolded yet, because the mobile work is gated on one prerequisite that must be prototyped first.

## The prerequisite: an Android USB-host plugin

**The `serialport` crate cannot be used on Android.** Android applications may not open `/dev/tty*`; USB serial must go through the Java `UsbManager` / `UsbDeviceConnection` / `UsbEndpoint` API, with per-device permission granted by a system dialog and a `device_filter.xml` intent filter for auto-launch on attach.

So `firmforge-flash` must gain an `android-usb` transport backed by a Tauri v2 Kotlin plugin implementing bulk and control transfers for CDC-ACM plus the CH34x / CP210x / FTDI bridges. This is the **single largest platform-specific work item in the project** and is tracked as its own milestone rather than being folded into "add mobile".

## iOS is BLE-only — by physics, not by choice

There is no general USB-serial access on iOS; arbitrary UART devices are reachable only through the MFi ExternalAccessory programme, and a generic ESP32 board is not MFi. The iOS app is therefore scoped as a **companion + BLE OTA** client, and the product says so plainly rather than shipping a crippled flasher.

See [`plan/spec/pm-requirements.md` §4](../../plan/spec/pm-requirements.md) for the full constraint analysis.
