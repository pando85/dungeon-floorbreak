# AREA-GOBLINWORKSHOP-01 evidence: the Goblin Workshop prop kit

Prototype art (`review_status: prototype_fixture_unapproved`); nothing here approves art, tuning or game feel. Spec: [goblin-workshop-prop-kit.md](../../02-art/goblin-workshop-prop-kit.md). The independent top-tier review is pending.

Everything below is drawn from the committed grid sources by `scripts/author_gwkit_frames.py` (the keys come from `scripts/author_gwkit_keys.py`), exported by `bash scripts/export_assets.sh` and rendered by the shared toolkit (`python3 -m scripts.pixelart evidence`); the in-scene stills are real windowed captures of `floor01_scene` (the Bevy presentation, 640 x 360).

## Regenerate

```bash
python3 scripts/author_gwkit_frames.py --meta          # grids, per-grid configs, manifests
for g in assets/source/props/floor01/gw_*.grid.json; do python3 scripts/pixel_grid_to_ase.py build "$g" "${g%.grid.json}.ase"; done
bash scripts/export_assets.sh                          # atlases under generated/art (git status -- assets/source stays clean)
for c in assets/source/props/floor01/gw_*.pixelart.json; do
  python3 -m scripts.pixelart evidence "$c" --out "docs/evidence/f01-gwkit/$(basename "$c" .pixelart.json)"
done
cargo build --locked --bin floor01_scene
BIN=./target/debug/floor01_scene bash scripts/room_capture.sh boiler chain 440:5:85 artifacts/f01-gwkit-captures
```

## What is in this folder

| Folder | Content |
| --- | --- |
| `gw_<prop>/` (13) | per grid: `contact_sheet_x1.png`, `contact_sheet_x4.png` (every frame, key frames marked, pivot and contact row drawn), `gif_<tag>_x1.gif` and `_x3.gif` at tick timing, `tell_attack_strip.png` (the state strip with tick windows), `scene_640x360.png` and `scene_1920x1080.png` (the frames on the lit Floor 1 lane next to Carl and Donut), `SHA256SUMS` |
| `review_pack/gw_<prop>/` (boiler, valve, brace, weight, drum) | the three-image review pack (contact sheet, lit scene, state strip) with the validator report, for the independent review |
| `scene/` | eight in-scene stills of the Boiler room at gameplay scale and the plume composite (below), with `SHA256SUMS` |

## Frames, pivots and the state-to-tag map

Pivot is the bottom-centre contact point of the prop (the rig, chain and tells use the top-left corner). Ticks are at 60 Hz; `duration_ms` is `round(ticks * 1000 / 60)`. Frame order in a grid follows its tags (a tag is one contiguous run), so a frozen key's grid index is not its spec order.

| Grid | Canvas | Pivot | Frames | Tags |
| --- | --- | --- | --- | --- |
| `gw_boiler` | 192 x 208 | (96, 207) | 8 | `boiler_idle`, `boiler_strain`, `boiler_damaged` |
| `gw_valve` | 32 x 48 | (16, 47) | 9 | `valve_intact`, `valve_warning`, `valve_venting`, `valve_spent` |
| `gw_brace` | 32 x 40 | (16, 39) | 4 | `brace_supported`, `brace_weakened`, `brace_stump` |
| `gw_weight` | 64 x 48 | (32, 47) | 2 | `weight_hang`, `weight_fall` |
| `gw_remnant` | 64 x 32 | (32, 31) | 1 | `remnant` |
| `gw_drum` | 36 x 36 | (18, 35) | 3 | `drum_armed`, `drum_telegraph` |
| `gw_scorch` | 48 x 8 | (24, 7) | 1 | `scorch` |
| `gw_rig` | 112 x 24 | (0, 0) | 3 | `rig`, `rig_run` |
| `gw_chain` | 4 x 8 | (0, 0) | 1 | `chain` |
| `gw_tell_sputter` | 16 x 16 | (0, 0) | 3 | `steam_sputter` |
| `gw_tell_plume` | 80 x 96 | (0, 0) | 4 | `steam_plume` |
| `gw_tell_shadow` | 64 x 8 | (0, 0) | 3 | `fall_shadow_tell`, `fall_shadow` |
| `gw_tell_ring` | 96 x 8 | (0, 0) | 2 | `drum_blast_tell` |

Frames added by execution (E): boiler idle 1 to 3, strain 2, damaged 2; valve warning 2, venting 3, spent wisp 2; brace creak 1; weight fall 1; rig run 2; drum telegraph 2; all 12 tell frames. Frozen keys: boiler 0 and 6, valve 0 and 6, brace 0, 1 and 3, and weight, remnant, drum, scorch, rig and chain frame 0 (`tests/fixtures/area-gw-*_keys.json`). The two keys changed under the gate approval are `valve_spent` (petals filled with copper) and `remnant` (sides slanted out); the other 11 are byte-identical to the gate head.

| Sim state | Sprite (sheet: tag) | Timing |
| --- | --- | --- |
| nothing broken | boiler: `boiler_idle`; valve: `valve_intact`; brace: `brace_supported`; weight: `weight_hang`; rig: `rig`; drum: `drum_armed` | idle loop 4 x 8 ticks |
| valve Warning | valve: `valve_warning`; boiler: `boiler_strain`; steam telegraph: `steam_sputter` | 2 x 4 ticks (`FLASH_TICKS`), sputter 3 x 4 |
| valve Venting | valve: `valve_venting`; boiler: `boiler_damaged`; steam: `steam_plume` (alpha 0x90) | 3 x 4 ticks, plume 4 x 4 |
| valve Spent | valve: `valve_spent` (wisp loop); boiler: `boiler_damaged` | 3 x 10 ticks, damaged 2 x 10 |
| mass Weakened | brace: `brace_weakened` (creak) | 2 x 6 ticks |
| mass Released | brace: `brace_stump`; weight: `weight_hang` swaying 1 px every 6 ticks; rig: `rig_run`; shadow: `fall_shadow_tell` (alpha 0x58) | rig 2 x 6 ticks, shadow 2 x 4 |
| mass Falling | weight: `weight_fall`; rig: `rig_run` last frame; shadow: `fall_shadow` | static |
| mass Landed | remnant: `remnant`; boiler: `boiler_damaged`; rig: `rig_run` last frame | static |
| drum Telegraph | drum: `drum_telegraph`; floor ring: `drum_blast_tell` | 2 x 4 ticks |
| drum Detonated | scorch: `scorch` | static |

Chains tile `gw_chain` from layout y 376: the brace chain at x 758 down to the brace while it stands, the weight chain at x 814 down to the weight until it falls. Anchors (layout px, canvas top-left): boiler (496, 372), valve (464, 532), brace (744, 540), weight (mass.x, mass.y - 16), remnant (784, 548), drum (846, 544), scorch (844, 572), rig (744, 352); a test (`the_anchors_match_the_encounter_rects`) checks them against `f01_boiler.encounter.json`.

## In-scene stills (`scene/`, chain plan, solo, 640 x 360)

| File | Tick | Shows |
| --- | --- | --- |
| `01_intact_room_t440.png` | 440 | intact room: boiler idle, valve, drum |
| `02_valve_warning_t495.png` | 495 | valve warning, boiler strain (firebox at full glow), the steam warning footprint at the right edge |
| `03_venting_plume_t535.png` | 535 | valve venting, damaged boiler, steam plume over the lane |
| `04_weight_telegraph_t735.png` | 735 | released weight swaying, stump, floor shadow tell |
| `05_weight_falls_t750.png` | 750 | the falling weight, the rig's loose chain end |
| `06_remnant_drum_telegraph_t765.png` | 765 | landed remnant, drum telegraph with the blast-area outline and floor ring |
| `07_scorch_t795.png` | 795 | scorch mark where the drum was |
| `08_all_broken_end_t860.png` | 860 | all-broken end state |
| `09_plume_over_lane_actors_composite_640x360.png`, `..._1920x1080.png`, `09_lane_actors_no_plume_640x360.png`, `09_plume_vs_baseline_crop_x3.png` | n/a | **composite, not a window capture** (acceptance 6): the real `steam_plume` frame 2 at the in-game alpha 0x90 on the tell layer, above real Carl, Donut and a goblin standing in the 80 x 96 lane on the lit Floor 1 lane, without and with the plume (left and right of the crop). Every scripted and aligned Boiler pilot keeps the actors out of the lane while the steam is Active, and the scene binary has no flag to place one, so a true capture needs a new pilot plan (a code change, out of this evidence-only pass). The sprites and lighting are the ones of the other stills (the toolkit's `Floor` renderer, `scripts/pixelart/scene.py`). |

## Simulation unchanged (read-only presentation)

- The window run and the headless run of the capture above end on the same `COMBAT_STATE` hash (`window_hash == headless_hash`, `match=yes`).
- Re-running the eight committed Boiler clip plans of `scripts/make_f01_boiler_evidence.sh` into a scratch folder with the kit drawn gives hash lines byte-identical to `docs/evidence/f01-boiler/capture_lines.txt`, and `python3 scripts/room_pilot_hashes.py boiler` is byte-identical to `pilot_hashes.txt` (compared with `cmp`; the committed GIFs of `docs/evidence/f01-boiler` were not touched).
- `drawing_the_pieces_and_sprites_never_changes_the_state_hash` (unit test) draws `pieces` and `sprites` every tick through the whole chain on the inert and the live roster and compares the state hash with an undrawn twin.

## Pending

- The `docs/evidence/f01-boiler` GIFs still show the earlier stand-ins. They are regenerated (twice, byte-identical) after the Boiler room retune of PR 257 merges, because that PR moves their hashes and GIFs.
- Not covered by this pass, by design: the scorch ember fade and the landing dust (both need an event tick the simulation does not keep). The independent review and the owner's game-feel review are pending.
