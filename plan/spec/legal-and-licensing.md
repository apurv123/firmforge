# Legal and licensing notes

> **This is engineering research, not legal advice.** It records what the licences
> actually say and where the real risks sit, so the product decisions below are
> deliberate rather than accidental. For anything consequential, get a lawyer.

There are three separable questions, and conflating them is the usual mistake:

1. What licence covers **firmforge's own code**?
2. What are the obligations of **distributing someone else's firmware binaries**?
3. What is the exposure from **what that firmware does** once it runs?

---

## 1. firmforge's own code

MIT (see `LICENSE`). Permissive, compatible with the dependency tree.

Dependency licences are all permissive: Tauri, `espflash`, `serialport`,
`btleplug`, `octocrab`, `serde`, `sha2` are MIT and/or Apache-2.0. One nuance:
on Linux the desktop shell links **WebKitGTK**, which is LGPL. Dynamic linking —
which is what Tauri does — keeps MIT distribution fine. Statically linking it
would not.

**Action:** run `cargo deny check licenses` in CI before the first public
release, so a future transitive GPL dependency is caught automatically rather
than discovered by a user.

---

## 2. Distributing other people's firmware

This is where the actual risk lives, and it is a direct consequence of the
`firmware/` folder convention in this repo.

Verified licences of the projects studied (checked against the GitHub API on
2026-08-01; note that GitHub's auto-detection is unreliable — Marauder reports
"no licence" via the API but ships a plain MIT `LICENSE` file):

| Project | Licence | Redistributing binaries |
|---|---|---|
| ESP32 Marauder | MIT | Easy — keep the copyright notice with the binary |
| ESP Web Tools | Apache-2.0 | Easy — notice + NOTICE file if present |
| Meshtastic firmware | GPL-3.0 | **Triggers a source obligation** |
| Bruce | AGPL-3.0 | **Triggers a source obligation** |
| ESPHome | Mixed (GPL-3.0 core, Apache-2.0 parts) | Depends on the component |
| M5Burner | Proprietary, no licence file | **Do not redistribute** |

### The GPL/AGPL binary problem

GPL-3.0 §6 and AGPL-3.0 §6 mean that if **you** convey a binary of that
firmware, **you** must also offer the corresponding complete source for that
exact build — including build scripts and the toolchain configuration needed to
reproduce it. Linking to the upstream repo is *not* sufficient on its own; the
offer must accompany your distribution and stay valid.

Mirroring a `Bruce.bin` into your `firmware/` folder therefore makes you a
distributor with obligations, not merely a person sharing a link.

### The design that avoids it

The manifest format already supports this: a part is a **path or URL**, so a
manifest can point at the *upstream project's own release asset* instead of a
copy you host.

**Recommended default — reference, don't rehost.** Let the manifest resolve to
upstream GitHub release URLs. The bytes flow from the original publisher to the
user; you ship metadata. This also means the SHA-256 in the manifest becomes a
genuine integrity check on someone else's artifact rather than a checksum of
your own copy.

Rehost only when you have a specific reason (an air-gapped mirror, a build you
compiled yourself), and when you do:

- keep the upstream `LICENSE` next to the binary,
- record the upstream commit SHA and release tag in the manifest (the provenance
  fields exist for this),
- for GPL/AGPL builds, host the corresponding source or a durable written offer,
- never rehost M5Burner or any artifact with no licence grant.

**Status:** the M1 GitHub-sourcing work should implement URL-referenced parts
first, and treat local mirroring as the exception. This is a legal constraint on
the architecture, so it is recorded here rather than left to implementation
taste.

---

## 3. What the firmware does

Much of this ecosystem is offensive-security tooling. Three distinct issues:

**Radio regulation.** Deauthentication attacks and sub-GHz transmission are
regulated. In the US the FCC has taken enforcement action over WiFi
deauthentication specifically — Marriott paid a $600,000 settlement in 2014 for
blocking personal hotspots. Sub-GHz transmit power and duty cycle limits differ
by region (FCC Part 15, ETSI EN 300 220). Users can and do break these rules
without realising.

**Computer-misuse law.** Using these tools against networks or devices you do
not own or have written authorisation to test is a criminal offence in most
jurisdictions (US CFAA, UK Computer Misuse Act 1990, India IT Act §43/§66).
Distributing the tools is generally lawful; using them is where liability
attaches. Germany's §202c is the notable outlier that has historically worried
tool authors.

**App store policy.** This is the most likely practical blocker. Apple's App
Review Guideline 4.3/2.5.x and Google Play's Device and Network Abuse policy
both prohibit apps that facilitate network attacks. An app whose store listing
advertises deauth or packet injection will be rejected.

The product response, already recorded as **R-SEC-7** in the PM requirements and
principle 8 in the spec: firmforge is a **neutral firmware manager**. It ships
with an empty catalogue; the user adds their own repositories. It does not
bundle, curate, promote or editorialise about offensive tooling, and it surfaces
each firmware's own licence and disclaimer rather than substituting its own.
That is both the honest description and the defensible one.

---

## 4. Trademarks

"Flipper Zero", "M5Stack", "ESPHome" and "Meshtastic" are other people's marks.
Using them factually — "compatible with M5Stack Cardputer", "flashes Meshtastic
firmware" — is nominative fair use. What to avoid: third-party logos in the app
icon or store listing, naming a build "official", or any presentation implying
endorsement or affiliation.

Meshtastic in particular has an explicit trademark policy and has asked
downstream projects to comply. If firmforge ever ships a Meshtastic-branded
experience, read it first.

---

## 5. Export control

Cryptographic functionality (SHA-256, Ed25519, TLS) is in scope for US EAR
category 5D002 in principle, but publicly available open-source software
qualifies for the §742.15(b) exception. In practice, a public MIT GitHub repo
with standard crypto libraries is the ordinary case and needs no licence. This
changes if the project ever ships a closed-source binary with crypto.

---

## 6. Practical checklist

- [x] firmforge licensed MIT, permissive dependency tree
- [x] Neutral positioning recorded in the spec and PM requirements (R-SEC-7)
- [x] Manifest carries provenance and per-part SHA-256
- [ ] Prefer upstream URLs over rehosted binaries when M1 lands
- [ ] `cargo deny check licenses` in CI
- [ ] Ship an empty default catalogue
- [ ] Show each firmware's licence and disclaimer before first flash
- [ ] Store listing describes a firmware manager, never an attack tool
- [ ] If any GPL/AGPL binary is ever rehosted, publish corresponding source
