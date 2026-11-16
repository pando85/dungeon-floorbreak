# ANIM-DONUT-01: evidence for the Donut locomotion candidate

Review aids for a human art decision, **not approvals**. Status: draft, awaiting owner art review. Write-up, frame table and self-review: [`docs/02-art/donut-locomotion-anim-01.md`](../../02-art/donut-locomotion-anim-01.md).

- **Rights record:** `ART-PUBLICATION-ALL-V1` in [`docs/04-production/rights-and-release.md`](../../04-production/rights-and-release.md) (owner-reported publication scope, 2026-11-15, not independently verified; no public reuse licence and no game or standalone asset-pack distribution right).
- **Integrity:** `SHA256SUMS` lists every file; `python3 scripts/check_docs.py` verifies it.
- **Carl** in the Floor 1 renders is the approved Carl 52 px idle frame 0 from main (`assets/manifests/carl_book1_h52.json`), unchanged.
- **Lighting** is the Pillow A1 model of `scripts/env_kit_light.py` on the ENV-01 sample layout (view `abc_640x360`); these are not Bevy captures.
- Authored and self-reviewed by Claude Opus.

| File | Content |
| --- | --- |
| `contact_sheet_1x.png` | all 16 frames at native size, rows: idle, run, jump_start/rise/fall/land; brown line = walking line |
| `contact_sheet_8x_grid.png` | the same at 8× with a pixel grid (8 px major lines), the pivot pixel (green) and index/tag/duration labels |
| `key_pose_review_round1_vs_final_5x.png` | key poses contact/gathered/extended: rejected round 1 (top) vs final (bottom) |
| `run_4x.gif` | run loop (two cycles), 4×, with the walking line |
| `jump_land_4x.gif` | idle → jump_start → jump_rise (held, lifted 26 px) → jump_fall (held, lifted 14 px) → land → idle, 4× |
| `run_flip_2x.gif` | right-facing and mirrored left-facing run, pivot on the same matte column, 2× |
| `floor01_trail_run_640x360.png` / `_1920x1080.png` | Carl with Donut (run frame 4) trailing 32 px behind and drawn behind him, as in COMP-01; native and exact 3× |
| `floor01_trail_run_x3.gif` | the run cycle in that lit scene, 3× crop around both actors |
| `floor01_lineup_640x360.png` / `_1920x1080.png` | left to right: Carl idle, Donut idle, run F0, F2, F4, jump crouch, rise (lifted), fall (lifted), landing impact |
| `measurements.json` | per-frame bbox, opaque height, lowest row, colours; scene positions |

## Regenerate

```bash
ART_CLI=/path/to/aseprite-or-libresprite bash scripts/export_assets.sh
python3 scripts/make_donut_locomotion_review.py --out docs/evidence/anim-donut-01
```

The renders are byte-identical across runs for the same inputs. `key_pose_review_round1_vs_final_5x.png` was made once from the scratch round-1 rig, which is not committed; it documents the review history only.
