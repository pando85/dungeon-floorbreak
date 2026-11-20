# ART-DONUT-CF02 evidence: Donut's CF02 acting sheet

**Status: 3 frames in the sheet (2 frozen ACT-DESIGN keys copied pixel for pixel, 1 derived frame); independent review pending; not owner-approved; not Bevy captures (nothing is wired into a runtime).**

Review renders of the [CF02 acting spec](../../02-art/cf02-acting-spec.md), section 4 (Donut, budget 4, 3 used, 1 spare kept in reserve). Everything is composed from the grid source `assets/source/characters/donut/donut_h40_cf02.grid.json`, rewritten deterministically by `python3 scripts/author_donut_cf02.py` (`--check` verifies it). Regenerate this folder with:

```sh
python3 scripts/author_donut_cf02.py --evidence                                                   # scenes, enlarged frames, timing diagram
python3 -m scripts.pixelart evidence assets/source/characters/donut/donut_h40_cf02.pixelart.json --out docs/evidence/cf02-donut
# then delete the toolkit's generic tell_attack_strip.png, scene_*.png and its SHA256SUMS (not meaningful for an overlay set) and rewrite SHA256SUMS
```

| File | What it shows |
| --- | --- |
| `donut_cf02_floor1_640x360.png`, `donut_cf02_floor1_1920x1080.png` | the lit Floor 1 corridor at the 640 x 360 virtual view (gameplay scale, x1) and the exact 3x 1080p copy: Donut and Carl at their real stand positions (idle), then idle Donut, `startle_puff`, `startle_settle` and `smug` 64 px apart, with a goblin (idle) on the right as the likely trigger |
| `donut_cf02_beside_carl_1x.png`, `donut_cf02_beside_carl_3x.png` | Donut at her real position next to idle Carl in four states, left to right: idle, `startle_puff`, `startle_settle`, `smug` (floor crop of the lit scene) |
| `frames_x8.png` | idle (approved) and the three CF02 frames at 8x, pivot (yellow) and floor row; keys labelled |
| `contact_sheet_x1.png`, `contact_sheet_x4.png` | all frames at native size and at 4x with tag, ticks, KEY flag, contact row (cyan) and pivot (yellow) |
| `gif_startle_x1.gif`, `gif_startle_x3.gif`, `gif_smug_x1.gif`, `gif_smug_x3.gif` | the two tags at 1x and 3x at the 60 Hz tick durations on the flat review background |
| `timing_diagram.png` | frame layout, pivot and the 60 Hz tick windows of startle (t0-5 puff, t6-11 settle, cooldown 180) and smug (t0-19, cooldown 300) from the manifest (`assets/manifests/donut_h40_cf02.json`) |

How `startle_settle` is derived (spec section 4 row 2, translations only, no new head): the body block drops 2 px and 2 of the 4 stiff shaft rows of the legs are removed (the body rides 1 px above idle, feet on row 43); the tail stays bottle-brushed with its fur rim but sits 2 px lower; the pupils return to the idle size (the idle face cel at the head's place); head, face and tiara move 1 px toward the facing side; collar and charm travel with the body, the tiara with the head; one `K` outline pixel on the body layer closes the cheek and chest outline that the head shift opened.

`SHA256SUMS` lists the hash of every file here.
