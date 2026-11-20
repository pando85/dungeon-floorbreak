# ART-CARL-CF02 evidence: Carl combat-acting sheet

**Status: 13 frames (9 frozen ACT-DESIGN keys copied pixel for pixel, 4 derived frames drawn); independent review pending; not owner-approved; not Bevy captures (nothing is wired into a runtime, CF02-I wires it).**

Review renders of the [acting spec](../../02-art/cf02-acting-spec.md) section 3. Everything is composed from the grid source (`assets/source/characters/carl/carl_h52_cf02.grid.json`) and the tick contract of its manifest (`assets/manifests/carl_book1_h52_cf02.json`). The grid is rewritten deterministically by `python3 scripts/author_carl_cf02.py` (`--check` verifies it). Regenerate this folder with:

```sh
bash scripts/export_assets.sh                                   # once: .ase, atlas and the Floor 1 kit atlases the scenes need
python3 -m scripts.pixelart evidence assets/source/characters/carl/carl_h52_cf02.pixelart.json --out docs/evidence/carl-cf02
python3 scripts/make_carl_cf02_evidence.py                      # lit sets, 1x checks, in-scene GIFs, timing diagram
# then delete the toolkit's generic scene_*.png and rewrite SHA256SUMS
```

| File | What it shows |
| --- | --- |
| `contact_sheet_x1.png`, `contact_sheet_x4.png` | all 13 frames at native size and at 4x (the enlarged frames): tag, tick count, frozen keys flagged KEY, contact row (cyan), pivot (yellow) |
| `carl_set_a_floor1_640x360.png`, `carl_set_b_floor1_640x360.png` (and the exact `_1920x1080.png` 3x copies) | the lit Floor 1 corridor at the 640 x 360 virtual view (gameplay scale, x1): idle Donut and Carl, then the 13 frames in atlas order (set A: stomp, air kick, tuck, plunge; set B: heavy landing, bail-out, flinch, frustration), 56 px apart, a goblin at the right end for scale. Airborne frames are drawn at the floor so the poses can be compared side by side; in play they sit on the sim height |
| `check_chamber_vs_tuck_x1.png`, `_x3.png` | the acceptance check at 1x: idle, `air_kick_chamber`, `dive_tuck`, `air_kick_boot`, `dive_plunge`. The chamber hangs its far leg and shows the knee of the near leg only; the tuck is a ball |
| `check_frustration_pair_x1.png`, `_x3.png` | the acceptance check at 1x: idle, `frustration_shake`, `frustration_flap`, `flinch`, idle, goblin. The open fan hand reads as a dismissive wave, not a guard |
| `scene_stomp_lying_goblin_x1.gif`, `scene_air_kick_jump_in_x1.gif`, `scene_dive_whiff_landing_x1.gif`, `scene_bailout_missed_kick_x1.gif` (and `_x3.gif`) | the four in-scene 1x GIFs the acceptance targets ask for, in the lit Floor 1 corridor at 60 Hz tick timing (every 2nd tick is drawn). Tick-driven mock-ups: a parabola for jumps, a straight plunge, a constant retreat; the stomp lands on the approved GOB-01 `down` frame (a stand-in until ART-GOB-CF02 has its stomped frames), the bail-out follows a whiffed approved push kick (k0-k3) with a GOB-01 swing. They are not the COMBAT-FUN-02 physics. The GIF palette is the toolkit's 64-colour adaptive palette, so lit colours look flatter than the PNGs |
| `gif_<tag>_x1.gif`, `gif_<tag>_x3.gif` | one GIF per tag on the flat review background with the walking line, at the manifest tick durations |
| `tell_attack_strip.png` | the toolkit strip of the stomp, air-kick and bail-out tags |
| `timing_diagram.png` | frame layout, pivot and 60 Hz tick windows of every move, drawn from the manifest |
| `SHA256SUMS` | the hash of every file here |

Airborne frames are lifted 2 px (lowest row 53) except `bailout_scramble`, which is 55 px tall and is lifted 1 px (lowest row 54); see the design question in the PR. The approved DEMO-01 sheets are byte-identical.
