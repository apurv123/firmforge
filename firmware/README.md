# `firmware/`

The folder firmforge reads. This is both the repository convention and a worked example — the app parses exactly this shape.

```
firmware/
├── manifest.json      # ESP Web Tools-compatible + firmforge extensions
├── manifest.sig       # detached Ed25519 signature over manifest.json (optional)
├── CHANGELOG.md
├── boards/            # per-board metadata: artwork, pinout, download-mode steps
└── builds/<version>/<chip>/*.bin
```

## Design rules

1. **`manifest.json` is a strict superset of the [ESP Web Tools](https://esphome.github.io/esp-web-tools/) manifest.** Every firmforge extension is optional and is ignored by ESP Web Tools, so the same file also drives a browser flasher. See `plan/spec/product-research/esp-web-tools/`.
2. **Builds are multi-part.** Each part has an absolute flash `offset`. The bootloader offset differs by chip family — `0x0` on ESP32-C3/C6/H2, `0x1000` on ESP32/S2/S3 — which is the classic hand-rolled-`esptool` footgun.
3. **`serialType`** is `"cdc"` (native USB) or `"uart"` (USB-to-UART bridge). **Omitting it means "fallback for any connection type."** firmforge prefers an exact match and falls back otherwise.
4. **Every part carries a `sha256`.** firmforge refuses to write an artifact whose hash does not match (R-SEC-1).
5. **Constraints are declared, not discovered.** `minFlashSize`, `psramRequired`, `minChipRevision` let the app dim an incompatible build *and say why*, instead of failing halfway through a write.
6. **Variants are explicit about what they drop.** `variant: "LITE_VERSION"` plus `variantOmits` is modelled directly on Bruce, whose lite build sacrifices SSH/WireGuard/the interpreter for M5Launcher partition compatibility.
7. **Assets are part of the install.** Files destined for SD or LittleFS are declared with their target paths so they are written in the same session — no removing the SD card.

Nothing here is mandatory for firmforge to work: a repository that publishes plain `.bin` files to GitHub Releases is still installable via chip detection. The manifest is what turns "installable" into "safe and obvious".
