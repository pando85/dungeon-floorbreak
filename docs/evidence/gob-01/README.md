# GOB-01 evidence: goblin size study, keys and the full 27-frame set (A and B)

These are review renders for the [GOB-01 design spec](../../02-art/gob-01-goblin-spec.md). They are composed PNGs and GIFs lit with the A1 compositor (`scripts/env_kit_light.py`), not Bevy captures. The goblin is not wired into any binary. Status: candidate, **full set drawn (Sonnet), awaiting independent Opus review**. The size (36 px) was decided by an independent Opus agent. The 11 keys are the Opus keys, frozen byte for byte in `tests/fixtures/goblin_gob01_keys.json`; the other 16 frames per variant are in-betweens.

Regenerate with:

```sh
bash scripts/export_assets.sh                      # builds the .ase, the atlases and the Floor 1 kit atlases the compositor needs
python3 scripts/author_goblin_frames.py            # rewrites the two grids and manifests (--check verifies them)
python3 scripts/make_goblin_gob01_review.py        # size study, key pose sheet, key scenes -> generated/art/gob01_review/
python3 scripts/make_goblin_gob01_set_review.py    # full-set evidence -> generated/art/gob01_set/
```

The files are copied here unchanged. `SHA256SUMS` lists their hashes; `measurements.json` (size study and keys) and `measurements_set.json` (all 27 frames of A and B, GIF delays) record opaque bboxes, heights, areas and colour counts.

## Full set (phase 3)

| File | What it shows |
| --- | --- |
| `<tag>_x3.gif`, `<tag>_x6.gif` (11 tags: `idle`, `walk`, `attack_tell`, `attack_swing`, `attack_recover`, `hurt`, `knockback`, `down`, `getup`, `corpse`, `knockback_contact`) | variant A on the left and B on the right, on the flat review floor, at the spec's tick table at 60 Hz. GIF delays are whole centiseconds, so each delay is rounded on the cumulative tick count (a 30-tick walk cycle is exactly 500 ms). Once tags rest 20 ticks on their last frame and hold tags show the hold for 30 ticks before the loop restarts |
| `contact_sheet_x1.png` | all 27 frames of A and B at 1x, with the contact row 55 (cyan) and the pivot (yellow); keys are numbered in yellow |
| `contact_sheet_A_x6.png`, `contact_sheet_B_x6.png` | all 27 frames at x6 (9 by 3), with frame, tag, ticks, KEY or in-between, bbox size and lowest row |
| `floor01_set_640x360.png`, `floor01_set_1920x1080.png` | Carl with 3 A goblins (walking behind him, tell, idle) and 2 B goblins (walk, stagger) on lit Floor 1 |
| `tell_hurt_1x.png` | attack tell, hurt and stagger for A and B at native size, side by side |
| `cp02_strip.png` | CP-02 staging: Carl kick, goblin 1 launch (18), flight (19), contact (26), goblin 2 hurt (16) and stagger (17), goblin 1 down (20), with tick numbers counted from the kick's active tick T. It is an illustration of the proposed contract, not a simulation. Carl's kick frames come from the ANIM-CARL-02 candidate (PR 43) through `tests/fixtures/carl_kick_cp02_frames.json` |
| `layout_diagram.png` | the 64 x 56 frame with the pivot, the standing and hurt boxes, the frame 13 hitbox, the 28 x 18 knockback box, the atlas strip with tag brackets and ticks, and the tick bars for the attack (24 + 2 + 4 + 20) and the cooldowns |

## Phase 1 and 2 (size study and keys)

| File | What it shows |
| --- | --- |
| `size_study_sheet.png` | top: the approved Carl 52 and Donut 40 idles and the goblin block-ins at 36, 40 and 44 px at x6 on one floor, with height guides; bottom: lit Floor 1 crops at x3 for each option |
| `floor01_size_h36_*.png`, `floor01_size_h40_*.png`, `floor01_size_h44_*.png` | Donut, Carl and two goblins of one option facing him at 54 and 104 px, at 640 x 360 and an exact x3 1920 x 1080 |
| `floor01_lineup_*.png` | Donut, Carl and the three options in one corridor |
| `key_pose_sheet.png` | the 11 keys (atlas frames 0, 4, 7, 11, 12, 13, 16, 19, 22, 25, 26) at 1x for A and B (tell and hurt side by side), then x3 cells for A and B, then Carl facing a mixed A/B group on lit Floor 1 at 1x and x3 |
| `floor01_keys_*.png` | the same scene, full frame, at 640 x 360 and 1920 x 1080 |

The block-ins in the size study are rasterized natively at each height by one generator (`scripts/author_goblin_gob01.py size-study`), so the three options are compared with one method. They are not production pixels.

## Known weak spots for the reviewer

- Frame 18 (launch) is the key 16 body leaned back by translating whole rows (no pixel is rotated), so its arm and the staircase of the lean are the roughest frame of the set.
- Frames 23 (push up) and 24 (kneel) are composed from frame 0 and frame 22 parts plus ASCII legs; the legs are flat bars at this size.
- The two walk passing feet (frames 6 and 9) are a small ASCII stamp and read thin at x6.

Provenance: design and keys by Claude Opus; size decided by an independent Claude Opus agent; in-betweens, atlases, validators and evidence by Claude Sonnet; no tracing of the concept references; review pending (independent Opus).
