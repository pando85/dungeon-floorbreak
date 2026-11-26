# AREA-HAZARDS-01B evidence: teal acid, ring tape backing, entry splash, corrosion and Rot marks

Prototype art (`review_status: prototype_fixture_unapproved`); nothing here approves art, tuning or game feel. Spec: [area-hazards-01b-spec.md](../../02-art/area-hazards-01b-spec.md). Owner art review pending. The design-step keys and the readability gate are in [keys/](keys/README.md).

Everything is drawn from the committed grid sources by `scripts/author_danger_fx.py` (which loads the two frozen keys verbatim and draws every other frame through `scripts/author_hazard_decals.py`), exported by `bash scripts/export_assets.sh` and rendered by the shared toolkit; the stills in `scene/` are real windowed captures of `floor01_scene` (the Bevy presentation), each ending on the same state hash as its headless replay.

## Regenerate

```bash
python3 scripts/author_danger_fx.py --meta          # grids, per-grid configs, manifests (14 sheets)
for g in assets/source/props/floor01/dfx_*.grid.json; do python3 scripts/pixel_grid_to_ase.py build "$g" "${g%.grid.json}.ase"; done
bash scripts/export_assets.sh                       # atlases under generated/art (assets/source stays clean)
for n in dfx_acid_splash dfx_corr_face dfx_corr_top dfx_corr_wall dfx_rot_nest dfx_rot_trail dfx_rot_drip; do
  python3 -m scripts.pixelart evidence "assets/source/props/floor01/$n.pixelart.json" --out "docs/evidence/area-hazards-01b/$n"
  python3 -m scripts.pixelart review-pack "assets/source/props/floor01/$n.pixelart.json" --out "docs/evidence/area-hazards-01b/review_pack/$n"
done
cargo build --locked --bin floor01_scene
bash scripts/make_hz01b_evidence.sh                 # scene/: the 640x360 window stills (two passes) and the splash GIF
python3 scripts/hz01b_evidence.py                   # contact_sheet_x8.png, splash_x1.gif, splash_x4.gif, cvd_room4_slime_pool.png
```

## What is in this folder

| File or folder | Content |
| --- | --- |
| `contact_sheet_x8.png` | every frame of every new grid at 8x on the dim lane tone, one labelled row per tag |
| `splash_x1.gif`, `splash_x4.gif` | the splash at tick timing (2, 3, 3, 4, 4 ticks), unmirrored then mirrored, at 1x and 4x |
| `cvd_room4_slime_pool.png` | the room 4 slime on its teal pool at 6x; rows from the top: normal, protan, deutan, tritan (Machado 2009, severity 1.0) |
| `dfx_<sheet>/` (7) | per grid: the toolkit's contact sheets (x1, x4), the tag GIFs at tick timing, the state strip, a lit-lane scene composite and `SHA256SUMS` |
| `review_pack/dfx_<sheet>/` (7) | the three-image review pack and the validator report (every sheet: PASS) |
| `scene/` | 13 window stills at 640 x 360 of rooms 4 and 6, the room 4 splash GIF, `capture_lines.txt`, `SHA256SUMS` |
| `keys/` | the design-step keys, the readability gate composites and the ring tape preview |
| `SHA256SUMS` | the checksum of every image in this folder's root |

## New sheets

| Grid | Canvas | Pivot | Frames | State to tag |
| --- | --- | --- | --- | --- |
| `dfx_acid_splash` | 32 x 24 | (16, 23) | 5 (2, 3, 3, 4, 4 ticks; frame 2 is the key) | an acid `HeroHurt`, `EnvironmentHurt` or `EnvironmentKill` -> `splash` once, 16 ticks |
| `dfx_corr_face` | 32 x 10 | (16, 0) | 1 (the key) | under every acid patch, tiled over the authored `x0..x1` |
| `dfx_corr_top` | 32 x 6 | (16, 5) | 1 | room 4's entry stain at x 150 |
| `dfx_corr_wall` | 12 x 24 | (0, 23) | 1 | on the backwall, one per patch at `x0 + 6` |
| `dfx_rot_nest` | 32 x 12 | (16, 0) | 1 | at the `start` of every ceiling track |
| `dfx_rot_trail` | 16 x 3 | (8, 0) | 1 | every 16 px along `lo..=hi`, more than 16 px from the nest |
| `dfx_rot_drip` | 8 x 16 | ceiling (4, 0), wall (0, 0) | 14 | a spent sticker: `drip_run_c`/`_w` 4 x 6 ticks from `detonate_tick`, then `drip_drop_c`/`_w` 3 x 20 ticks looping |

`dfx_acid` (recoloured, rows unchanged) and `dfx_ring_r48` / `dfx_ring_r64` (outer row backed with `flame_00`) are regenerated in [the danger FX evidence](../f01-danger-fx/README.md).

## In-scene stills (`scene/`, 640 x 360)

Room 4 (`--f01-room hazards`): with the `direct` pilot Carl walks into the first pool and is hurt on tick 41 (x 238, left of the pool's centre: a mirrored splash) and on tick 100 (x 299, right of the centre: not mirrored); the crown frame shows on age 5. With the `haz_spectacular` pilot the slime is defeated on tick 164 (the pool drains on 164..223), sticker 0 is provoked on 373 and blasts on 397, Carl's stick lands on 522. Room 6 (`--f01-room mastery`, the `mastery` pilot): the slime is defeated on 101, the draining pool hurts Carl on 123, the sticker is provoked on 677 and blasts on 701.

| File (`_640x360.png`) | Room, pilot, tick | Shows |
| --- | --- | --- |
| `r4_slime_pool` | 4, `haz_spectacular`, 120 | the green slime on its teal pool, Donut and Carl beside it, the entry stain on the floor at x 150 |
| `r4_splash_left_entry` | 4, `direct`, 46 | Carl enters from the left: the mirrored crown at the pool's left end |
| `r4_splash_right_entry` | 4, `direct`, 105 | Carl enters from the right of the centre: the unmirrored crown at his boots |
| `r4_rot_ring_march` | 4, `haz_spectacular`, 385 | the 48 px march ring of sticker 0 on mid-dark brick, the nest on the slab above it |
| `r4_dyn_ring_march_lit_brick` | 4, `haz_spectacular`, 560 | the 64 px march ring of Carl's stick on the lit brick by the torch |
| `r4_dyn_ring_hot_lit_brick` | 4, `haz_spectacular`, 630 | the same ring solid hot |
| `r4_corrosion_drained` | 4, `haz_spectacular`, 240 | the first pool gone: its face stain and wall trail remain |
| `r4_nest_drip` | 4, `haz_spectacular`, 440 | the nest and scuffs on the slab, the spent drip's drop loop |
| `r6_slime_pool` | 6, `mastery`, 90 | the slime and the 40 px teal pool |
| `r6_splash` | 6, `mastery`, 128 | Carl hurt by the draining pool: the mirrored crown clamped inside its live rectangle |
| `r6_corrosion_drained` | 6, `mastery`, 210 | the drained pool's stain |
| `r6_ring_march_dark_brick` | 6, `mastery`, 690 | the 48 px march ring on the dark brick of the closing arena |
| `r6_nest_drip` | 6, `mastery`, 780 | the nest on the right perch's slab and the spent drip |

`splash_room4_640x360.gif`: ticks 40..62 of the `direct` run in room 4, every tick (game speed).

`scene/capture_lines.txt` holds one `HZ01B_STILL` line per still and one `HZ01B_GIF` line: every line ends `hash_match=yes` (`window_hash == headless_hash`) and every still `passes_byte_identical=yes` (two full capture passes, byte-identical PNGs).

## Simulation unchanged (read-only presentation)

- Every capture's window run and headless replay end on the same `COMBAT_STATE` hash (`scene/capture_lines.txt`).
- `acid_splash::tests::drawing_never_changes_the_state_hash` and `hazard_decals::tests::drawing_never_changes_the_state_hash` step and view the splash cue and compute every decal and drip on every tick of a hazard room (acid with a linked slime, a ceiling sticker, the live party) and compare the state hash and the event count with an undrawn twin.
- No golden, pinned state hash, capture hash or replay log was edited: the render replay lines of rooms 4 and 6 (`docs/evidence/f01-playtest/render_replay_lines.txt`) hold state hashes only, so the pixel change does not move them (see the PR body).

## Moved pins

| Pin | Why |
| --- | --- |
| `tests/fixtures/f01-dfx-ring-r48_keys.json`, `tests/fixtures/f01-dfx-ring-r64_keys.json` | HZB-AD2: the outer band row of the march and hot frames is `flame_00`, so frame 0 (the frozen key) changes |
| `assets/manifests/f01-dfx-acid.json`, `f01-dfx-ring-r48.json`, `f01-dfx-ring-r64.json` | the palette id and provenance of the acid (HZB-AD1) and the ring frame digests (HZB-AD2); the acid frame digests are unchanged because they hash the rows |
| the `dfx_acid`, `dfx_ring_r48`, `dfx_ring_r64` atlases and the evidence `SHA256SUMS` of their folders | the pixels (teal ramp, tape backing) |

Unmoved on purpose: `tests/fixtures/f01-dfx-acid_keys.json` (a rows-only digest of rows that did not change), every golden and every state hash.

## Not covered

- Room 6's non-mirrored splash: every stock pilot enters the 40 px pool from its left (tick 123 at x 316 or 313, centre 340). The mirror rule is room independent and tested (`acid_splash::tests::the_splash_is_mirrored_below_the_centre`).
- A goblin landing splash needs `fu.acid_hurt_event` for a non-lethal landing; a lethal landing splashes through its `EnvironmentKill`. No room pilot lands a goblin in a pool; the triggers are covered by `acid_splash::tests::only_acid_events_trigger`.
- Wall nests are deferred (HZB-AD6); no authored room has a wall sticker, so the wall drip is covered by a test (`hazard_decals::tests::a_wall_drip_is_mirrored_on_the_right_wall`), not a capture.
- Human pad play and a real-GPU frame time were not run.
