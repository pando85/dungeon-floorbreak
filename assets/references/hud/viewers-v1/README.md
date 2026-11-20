# AUDIENCE-01 viewer HUD reference image intake

**State: received and verified.** All five image binaries are committed to the PR branch; `sha256sum -c SHA256SUMS` passes and the byte sizes and 640x360 dimensions match `manifest.json`. This is a reference-only directory for the compact **Option A** individual-counter overlay on the owner-approved HUD-HYBRID-A-01.

**Source of truth:** `docs/02-art/individual-viewers-hud-v1.md`, `docs/07-agents/individual-viewers-hud-workorder.md`, and the real integrated Bevy capture `docs/evidence/hud-hybrid-a/hud-REAL-normal-640x360-1x.png`.

The assets are **640x360 transparent overlay references only**, not full scene captures, game screens, export atlases or production sprites. Their pixels must be composited over the existing Bevy screenshot for review; the background is deliberately transparent. Source art must be authored/validated in the `assets/source/hud/` grid+atlas pipeline before runtime use. SHA-256, byte sizes, PNG resolution, generation/provenance and rights recorded in `manifest.json` and `SHA256SUMS`.

| Slot | Expected destination | Purpose |
| --- | --- | --- |
| locked | `01-inline-locked-640x360.png` | fully transparent: before narrative unlock no viewer field exists |
| normal | `02-inline-normal-640x360.png` | Carl + Donut baseline viewer numbers at existing plate positions |
| spectacular | `03-inline-spectacular-640x360.png` | large gains per hero, inline trend cues |
| anticipation | `04-inline-anticipation-640x360.png` | unexplained, authored pre-danger spike; **same UI cue as a spectacular gain** |
| falling | `05-inline-falling-640x360.png` | dim declining audience trend cue |

Receipt: the importer verified the source files, kept the bytes exact and recorded SHA-256, byte sizes, PNG resolution, generation/provenance and rights in `manifest.json` and `SHA256SUMS`.

Visual approval: owner chose a compact in-plate indicator after rejecting larger and off-plate proposals. This is design approval **only**; not final tuning, a finished native asset or an implementation. Source: project conversation, ChatGPT-assisted overlay sketches 2026-11-19. Fan-concept rights: owner-reported `ART-PUBLICATION-ALL-V1`, without general public reuse or game distribution rights.
