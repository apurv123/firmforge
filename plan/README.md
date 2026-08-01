# plan/

Planning artifacts for firmforge.

```
plan/
└── spec/
    ├── firmforge-spec.md      ← the product specification (start here)
    ├── pm-requirements.md     ← technical & PM requirements research
    └── product-research/      ← competitor teardowns + Playwright screenshots
        ├── README.md          ← competitor set, method, cross-cutting conclusions
        ├── bruce-firmware/
        ├── esp32-marauder/
        ├── esphome/
        ├── esp-web-tools/
        ├── m5burner/
        ├── meshtastic/
        └── flipper-zero/
```

**Reading order:** `product-research/README.md` → individual teardowns → `pm-requirements.md` → `firmforge-spec.md`.

Screenshots were captured with headless Chromium via Playwright (`tools/research/capture.mjs`, 1440×900, full-page) on 2026-08-01. User-demand tables were pulled from the GitHub REST API sorted by 👍 reactions. Re-run with `node tools/research/capture.mjs`.
