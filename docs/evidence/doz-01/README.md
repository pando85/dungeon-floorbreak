# DOZ-01 evidence: Goblin Murder Dozer, full frame set

These are review renders for the [DOZ-01 design spec](../../02-art/doz-01-dozer-spec.md). They are composed PNGs and GIFs, not Bevy captures. The lit scenes use the A1 compositor (`scripts/env_kit_light.py`); the state GIFs are unlit on a plain floor. The Dozer is not wired into any binary.

Status:

- **Design:** Claude Opus, phases 1 and 2.
- **Size:** h120, decided by a separate Claude Opus agent.
- **Execution:** Claude Sonnet. The full frame set is drawn (spec section 7.3; notes and open points in spec section 15).
- **Review:** the independent Claude Opus review returned PASS WITH FOLLOW-UPS.
- **Design fix round** (Claude Opus; spec section 7.4): D2 bigger, lighter black smoke keys; D3 a short trailing wheel smear on 1 charge tick in 4; D7 a deeper engineer pot. Every render here was regenerated with them.
- **Pending:** Sonnet's redraw list in spec section 7.4 (the black smoke in-betweens and a check of the engineer frames), then the owner's review.

## Regenerate

```sh
python3 scripts/author_dozer_doz01.py          # rewrites the grids, manifests and the rig contract (--check verifies them)
bash scripts/export_assets.sh                  # builds the .ase, the atlases and the Floor 1 kit atlases the compositor needs
python3 scripts/make_dozer_doz01_review.py     --out docs/evidence/doz-01   # size study, layout diagram, size-option scenes
python3 scripts/make_dozer_doz01_set_review.py --out docs/evidence/doz-01   # GIFs, lit chase, contact sheet, rig and timing diagram
```

`SHA256SUMS` lists the hash of every file here except itself and this README.

## Files

| File | What it shows |
| --- | --- |
| `cruise_x1.gif`, `cruise_x3.gif` | Cruise at 160 px/s for 120 ticks: tread frame = floor(distance) mod 9, wheel at 2 ticks per frame, rattle with the riders 2 ticks late, black puffs from the stack (every 12 ticks) and the rear exhaust (every 20), idle 0/1 for the riders (B 8 ticks late) and the engineer |
| `tell_x1.gif`, `tell_x3.gif` | The 36 tick charge tell: wheel at 1 tick per frame, stack puffs every 4 ticks, a 3-puff steam burst, the engineer's lever pull, the riders' yell 0/1 (6 ticks each, 3 loops), the hull and riders squat 1 px |
| `charge_x1.gif`, `charge_x3.gif` | The 90 tick charge at 240 px/s (4 px per tick): lurch back for 8 ticks, then yell and point/jab; the wheel smear on 1 charge tick in 4 (D3; whether a GIF frame lands on a smear tick depends on the GIF tick sampling, which Sonnet is reworking); stack puffs every 8 ticks |
| `brake_x1.gif`, `brake_x3.gif` | The 24 tick brake: the treads stop, the rig slides 4 px, lurch forward for 10 ticks, a steam burst |
| `strain_x1.gif`, `strain_x3.gif` | The 90 tick strain: rattle at double rate, puffs from all three emitters, the wheel at 3 ticks per frame |
| `floor01_chase_lit_640x360.png`, `floor01_chase_lit_1920x1080.png` | The machine chasing Carl and Donut on lit Floor 1, the wheel in front of a **torch-lit** stretch of brick (native and x3). Carl is 213 px ahead of the wheel tip, Donut behind him. The contact shadow is the tread footprint |
| `floor01_chase_unlit_640x360.png`, `floor01_chase_unlit_1920x1080.png` | The same, with the wheel in front of an **unlit** stretch (the nearest lit torch is about 190 px behind the wheel). A raised step shows at the right edge |
| `rig_timing_diagram.png` | The assembled rig at x3 with part anchors and the rig origin; the frame size, pivot and anchor of every part; the 60 Hz tick bars of the five states in one chain (tread and wheel frames, engineer, rider A and B, puff spawns, hull and rider offsets); the state flow |
| `part_contact_sheet.png` | Every frame of every part atlas (hull body, both front boxes, yoke, wheel 4 + smear, tread 9, riders 19, smoke 11, spark 3), enlarged, with pivots and tick counts |
| `size_study_sheet.png`, `floor01_chase_h96_*`, `floor01_chase_h120_*`, `floor01_chase_h150_*` | The size study (block-ins, unchanged design evidence): the three options on one floor next to Carl 52 and Donut 40, and lit chase scenes. Their riders include the fist-to-stick fix |
| `floor01_chase_keys_h120_*` | The decided option assembled from its parts with the rig contract |
| `part_layout_diagram.png` | The assembled rig at x3 with anchors, the proposed lethal wheel circle (r 44), the proposed pushing wall, the occluder rows, the emitters and the light points; the exploded parts; the proposed charge timeline |
| `measurements.json`, `set_measurements.json` | Size-option measurements; frame counts, colours and lint per atlas, and the GIF delays |

## How the GIFs keep true tick timing

The state table (ticks, speed, wheel rate, rattle, puff schedule) and the emitters are read from `dozer_rig_h120.json`. The simulation advances in 60 Hz ticks; one GIF frame is drawn every second tick, and GIF delays are whole centiseconds rounded on the cumulative tick count (3, 3, 4 cs), so a 90 tick state lasts exactly 1.5 s. The first frames of each GIF carry the smoke of the state before it. The label on each frame shows the tick, the tread and wheel frame and the hull offsets.

## Weakest points (for the reviewer)

- The black smoke keys are fixed (D2), but the in-betweens `black_1` and `black_3` to `black_5` keep their phase-2 shapes until Sonnet redraws them, so the puffs still shrink late in their life.
- The charge smear is fixed (D3): twelve short trailing arcs with no ring.
- The cab posts are dark iron against the plum outline and read faintly at 1x.
- The yoke's second rivet row has only 4 rivets.
- The engineer's idle 0 has no pointing arm; it reads as an empty-handed idle at 1x.
