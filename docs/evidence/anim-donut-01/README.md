# ANIM-DONUT-01: evidence for the Donut locomotion candidate

Review aids for a human art decision, **not approvals**. Status: draft, awaiting owner art review. **Round 2** (after the independent Opus review of round 1). Write-up, frame table and self-review: [`docs/02-art/donut-locomotion-anim-01.md`](../../02-art/donut-locomotion-anim-01.md).

- **Rights record:** `ART-PUBLICATION-ALL-V1` in [`docs/04-production/rights-and-release.md`](../../04-production/rights-and-release.md) (owner-reported publication scope, 2026-11-15, not independently verified; no public reuse licence and no game or standalone asset-pack distribution right).
- **Integrity:** `SHA256SUMS` lists every file; `python3 scripts/check_docs.py` verifies it.
- **Carl** in the Floor 1 renders is the approved Carl 52 px idle frame 0 from main (`assets/manifests/carl_book1_h52.json`), unchanged.
- **Lighting** is the Pillow A1 model of `scripts/env_kit_light.py` on the ENV-01 sample layout (view `abc_640x360`); these are not Bevy captures.
- **Rights/Provenance:** design: Opus; execution: Sonnet; review: pending independent Opus. Round 1 was authored and self-reviewed by Claude Opus.

| File | Content |
| --- | --- |
| `contact_sheet_1x.png` | all 16 frames at native size, rows: idle, run, jump_start/rise/fall/land; brown line = walking line |
| `contact_sheet_8x_grid.png` | the same at 8× with a pixel grid (8 px major lines), the pivot pixel (green) and index/tag/duration labels |
| `run_footfall_diagram.png` | **for the owner:** run frames 4-9 at 8×, ground line and pivot marked, the paw on row 43 highlighted and labelled per frame, airborne frames labelled with their gap to the ground |
| `round2_keys_6x.png` | the five round-2 key poses (frames 4, 6, 8, 10, 12) at 6× |
| `key_pose_review_round1_vs_final_5x.png` | key poses contact/gathered/extended: rejected round 1 (top) vs final (bottom) |
| `run_4x.gif` | run loop (two cycles), 4×, with the walking line; frame times are the 185 px/s values |
| `jump_land_4x.gif` | idle → jump_start → jump_rise (held, lifted 26 px) → jump_fall (held, lifted 14 px) → land → idle, 4× |
| `run_flip_2x.gif` | right-facing and mirrored left-facing run, pivot on the same matte column, 2× |
| `floor01_trail_run_640x360.png` / `_1920x1080.png` | Carl with Donut (run frame 4) trailing 32 px behind and drawn behind him, as in COMP-01; native and exact 3× |
| `floor01_trail_run_x3.gif` | the run cycle in that lit scene, 3× crop around both actors |
| `floor01_lineup_640x360.png` / `_1920x1080.png` | left to right: Carl idle, Donut idle, run F0, F2, F4, jump crouch, rise (lifted), fall (lifted), landing impact |
| `measurements.json` | per-frame bbox, opaque height, lowest row, colours; scene positions |

## Measured geometry (round 2, replaces the round-1 "no skating" and "back arched" claims)

Measured on the committed grid and pinned by `tests/test_art_donut_locomotion.py`.

| Run frame | Role | Paw on row 43 | Lowest paw row | Back top y at x 20/24/28/32/36 | Face top-left | ms |
| --- | --- | --- | --- | --- | --- | --- |
| 4 | key | near fore, x 51-56 | 43 | -/24/21/21/21 | (43, 20) | 80 |
| 5 | in-between | far fore, x 53-58 | 43 | -/25/22/20/20 | (43, 21) | 80 |
| 6 | key | near hind, x 36-41 | 43 | -/27/21/18/18 | (43, 21) | 80 |
| 7 | in-between | far hind, x 32-37 | 43 | 23/20/19/19/20 | (43, 20) | 80 |
| 8 | key | none (airborne) | 39 | 20/20/20/20/20 | (44, 20) | 90 |
| 9 | in-between | none (airborne) | 42 | 21/21/21/21/22 | (43, 20) | 80 |

- **No skating:** each paw is on row 43 in exactly one frame and no paw is on row 43 in two consecutive frames, so a planted paw cannot slide between frames at any speed (round 1: forepaws slid about 8-11 px at trail speed).
- **Spine:** back top at x 32 goes 21, 20, 18, 19, 20, 21: a 3 px arch in frame 6 with the rump curling under, and a flat back 9 px longer in frame 8 (body starts at x 13 against x 22 in frame 4). No step along the back is larger than 2 px past the rump curve.
- **Head and tail:** the face bobs 1 px (y 20 or 21) and lunges 1 px forward in frame 8 only; the tail tip lean changes by at most 2 px between neighbouring run frames (round 1 swung up to 7 px).

## Playback rule (for the integration agent)

Donut trails Carl at his ground speed `vx` (up to `RUN_SPEED` 185 px/s). The run is designed for D = 14.8 px of ground per frame:

```
frame_ms(i) = clamp( round( w[i] * 1000 * D / |vx| ), 50, 130 ),  D = 14.8 px
w = [1, 1, 1, 1, 1.125, 1]     (frame 8, the suspension, is held a little longer)
```

| \|vx\| | Base frame | Frame 8 | Cycle | Stride per cycle |
| --- | --- | --- | --- | --- |
| 185 px/s | 80 ms | 90 ms | 490 ms | 90.7 px |
| 145 px/s | 102 ms | 115 ms | 625 ms | 90.6 px |
| < 114 px/s | 130 ms (clamped) | 130 ms | 780 ms | below the design speed; a walk cycle is out of scope |

The atlas stores the 185 px/s values (80, 80, 80, 80, 90, 80 ms). This replaces round 1's fixed "460 ms cycle".

## Regenerate

```bash
ART_CLI=/path/to/aseprite-or-libresprite bash scripts/export_assets.sh
python3 scripts/make_donut_locomotion_review.py --out docs/evidence/anim-donut-01
python3 scripts/make_donut_run_diagram.py --out docs/evidence/anim-donut-01/run_footfall_diagram.png
```

The renders are byte-identical across runs for the same inputs. `key_pose_review_round1_vs_final_5x.png` was made once from the scratch round-1 rig, which is not committed; it documents the review history only.
