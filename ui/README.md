# ui/

The shared frontend, served by the Tauri shells.

`dist/index.html` is currently a dependency-free static shell that renders screen **D2 (Connect a device)** from the spec — the Meshtastic-derived three-step `DEVICE → FIRMWARE → FLASH` layout with progressive enablement — and calls the real `list_ports` Tauri command.

It is deliberately plain HTML/CSS/JS for now so that:

- `cargo check`/`cargo build` of the desktop shell needs no Node toolchain, and
- the frontend framework choice stays open until milestone **M1 — Read**, when the catalogue screens (D3/D4) start needing real state management.

The same bundle is intended to serve both shells, switching from the desktop left rail to the mobile bottom tab bar responsively (spec §9).
