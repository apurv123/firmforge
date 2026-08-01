# Teardown: M5Burner (M5Stack)

> "M5Burner is a firmware burning software that integrates firmware burning, exporting, publishing, sharing and other functions."
> [docs.m5stack.com/en/uiflow/m5burner/intro](https://docs.m5stack.com/en/uiflow/m5burner/intro) — Windows / macOS / Linux desktop application

---

## 1. Product description

M5Burner is M5Stack's native desktop firmware manager, and it is **the closest existing product to firmforge desktop**. Unlike everything else in this research set, it is not a web page and not a CLI: it is a downloadable cross-platform GUI whose entire job is browsing, downloading and burning firmware to devices, plus **publishing and sharing your own firmware** into a shared catalogue.

Its relevance is amplified by the fact that **Bruce ships through it**: Bruce's own README tells M5Stack users to "burn it directly from the m5burner tool, just search for 'Bruce'… official builds will be uploaded by *owner* and have photos." So M5Burner already functions as a third-party firmware store for exactly the audience firmforge targets — and Bruce already had to invent an ad-hoc trust heuristic ("uploaded by owner, has photos") to compensate for M5Burner's lack of verified publishing.

## 2. UI teardown (from `02-m5burner-docs.png`, which embeds the app UI)

The layout is a three-part pattern worth copying almost wholesale:

- **Left rail — device categories:** `CORE`, `CORE2 & TOUGH`, `CORES3`, `STICKC`, `ATOM`, `ATOMS3`, `STICKV & UNITV`, `T-LITE`, `CAMERA`, `CORELINK`, `STAMP`, `STAMPS3`, `CAPSULE`, `DIAL`, `AIRQ`… — i.e. **hardware model is the primary navigation axis**, not firmware name.
- **Top bar — search + an "Only Official" checkbox.** One checkbox is the entire trust UI. Cheap, legible, and clearly insufficient — but better than nothing, which is what everyone else has.
- **Main area — firmware cards**, each with: large product artwork, firmware title, an **"official" badge**, a one-line description with hardware/flash-size qualifiers ("UIFlow2.0 for CORE/M5GO/GRAY (4MB & 16MB)"), a **version dropdown** (`v2.0.9-16MB`), publisher name, "Published At" date, download-count and like-count, and a blue **Download** button that becomes Burn.

That card — artwork, badge, version dropdown, publisher, date, download count, one action button — is a near-perfect template for the firmforge catalogue tile.

## 3. Features

- Firmware **burning** to M5Stack devices over USB.
- **Firmware export** — package a device's current firmware back out to a file.
- **Firmware publish** — upload your own build into the shared catalogue.
- **Sharing** — distribute firmware to other users via the catalogue.
- Per-device-category browsing, official/community filtering, version selection.
- Bundled device USB driver distribution (M5Stack ships a "Device USB Driver" download alongside) — a reminder that on Windows, **driver installation is part of the product**, not an external concern.
- Sits inside a wider first-party toolchain: UIFlow 1/2 (`04-uiflow.png`), Block Designer, EzData, EasyLoader Packer, VLW Font Creator.

## 4. Workflows

**A. Burn official firmware:** launch M5Burner → pick device category in the left rail → find firmware card → choose version from the dropdown → Download → Burn → device runs it.
**B. Publish your own:** build firmware → Publish from within M5Burner → it appears in the catalogue for others (this is how Bruce reaches M5Stack users).
**C. Export:** pull the current firmware off a device to a file — useful for backup before experimenting, and one of the few "protect the user" features in this whole category.
**D. Trust decision:** tick "Only Official", or eyeball the publisher name and artwork.

## 5. Screenshots

| File | Page |
|---|---|
| `01-m5stack-download.png` | M5Stack download hub — where M5Burner sits among the tools |
| `02-m5burner-docs.png` | **M5Burner docs page, embedding the desktop UI: left device rail, "Only Official" filter, firmware cards with version dropdowns** |
| `03-community.png` | M5Burner community forum category |
| `04-uiflow.png` | UIFlow2 — the adjacent visual programming tool |
| `05-m5stack-products.png` | Hardware catalogue — the device universe being served |

## 6. Top user feedback

M5Burner's feedback is not on GitHub (it is closed-source), it is on the **community forum** — which is itself a finding: there is no public issue tracker, no reaction-ranked backlog, and no visible roadmap. Recurring themes visible in the M5Burner forum category and in cross-project references:

- **Login / account requirement friction** — M5Burner gates some functionality behind an M5Stack account, which users resent for a local flashing tool.
- **Serial driver problems on Windows and macOS** (CH9102/CP210x/FTDI), the number-one first-run failure across the entire category.
- **Firmware/version confusion** — 4MB vs 16MB variants of the same firmware, which the version dropdown exposes but does not explain.
- **Trust ambiguity in community uploads** — hence Bruce's need to tell users to look for "uploaded by owner, has photos."
- **Vendor lock-in** — M5Burner is for M5Stack devices; the same user's Lilygo or generic ESP32 board needs a different tool.

## 7. What firmforge should steal

1. **The three-pane desktop layout**: device-category rail, search + trust filter, firmware card grid. It is the correct information architecture for this job and it is already validated.
2. **The firmware card contents** — badge, version dropdown, publisher, publish date, download count, single primary action.
3. **"Only Official" as a persistent trust filter**, but implemented properly: *verified publisher* = signature chain to a known GitHub identity, not a self-asserted flag.
4. **Firmware export / device backup before burn.** Almost nobody offers this and everybody wants it after the first time they lose a working setup.
5. **Ship the USB drivers story explicitly** — detect the bridge chip, link or bundle the right driver, and say so before the user hits a failure.
6. **Publish/share flow** — let a firmware author push a release into the catalogue. In firmforge this becomes "point at a GitHub repo," which is strictly better than an upload form.

## 8. Where firmforge wins

| M5Burner gap | firmforge answer |
|---|---|
| M5Stack devices only | Vendor-neutral: any board, driven by manifest + chip detection |
| Closed source, no public issue tracker, no roadmap | Open source, public reaction-ranked backlog |
| Account/login friction for a local operation | Works fully offline and anonymously; auth only for private repos |
| "Only Official" is a self-asserted flag | Verified publisher via signature + GitHub provenance |
| Desktop only | Desktop **and** mobile |
| Catalogue is a proprietary upload silo | Catalogue is *your GitHub repo* — no upload, no lock-in, no review queue |
