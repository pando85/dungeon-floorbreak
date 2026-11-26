# F01 danger FX evidence: acid pool, blast reach rings, blast waves, room 5 gate

Prototype art (`review_status: prototype_fixture_unapproved`); nothing here approves art, tuning or game feel. Spec: [floor01-danger-fx-spec.md](../../02-art/floor01-danger-fx-spec.md). The independent top-tier art-director review passed (fix round applied); the owner's art and game-feel review are pending.

**F01-SIM-TICKS** ([spec](../../02-art/f01-sim-ticks-spec.md), [evidence](../f01-sim-ticks/README.md)) added the four `gate_drop` frames to `dfx_gate`: its `dfx_gate` folder and review pack were regenerated (frames 0 and 1 and the frozen key are unchanged). The `gate_*` stills in `scene/` predate the drop and the slam dust and were not recaptured (the HUD has changed since; a recapture would move every still); the drop and the dust are captured in the F01-SIM-TICKS evidence.

**AREA-HAZARDS-01B** ([evidence](../area-hazards-01b/README.md)) recoloured the acid to its own teal ramp (HZB-AD1: the rows are unchanged, only the ten palette entries moved) and backed the rings' outer band row with the dark tape `flame_00` (HZB-AD2). The `dfx_acid`, `dfx_ring_r48` and `dfx_ring_r64` folders, their review packs and every still and the GIF in `scene/` were regenerated with that art; every capture line keeps its window and headless state hashes, only the PNG checksums moved.

Everything is drawn from the committed grid sources by `scripts/author_danger_fx.py`, exported by `bash scripts/export_assets.sh` and rendered by the shared toolkit (`python3 -m scripts.pixelart evidence` and `review-pack`); the in-scene stills and the GIF in `scene/` are real windowed captures of `floor01_scene` (the Bevy presentation), each ending on the same state hash as its headless replay.

## Regenerate

```bash
python3 scripts/author_danger_fx.py --meta          # grids, per-grid configs, manifests
for g in assets/source/props/floor01/dfx_*.grid.json; do python3 scripts/pixel_grid_to_ase.py build "$g" "${g%.grid.json}.ase"; done
bash scripts/export_assets.sh                       # atlases under generated/art (assets/source stays clean)
for c in assets/source/props/floor01/dfx_*.pixelart.json; do
  n="$(basename "$c" .pixelart.json)"
  python3 -m scripts.pixelart evidence "$c" --out "docs/evidence/f01-danger-fx/$n"
  python3 -m scripts.pixelart review-pack "$c" --out "docs/evidence/f01-danger-fx/review_pack/$n"
done
cargo build --locked --bin floor01_scene
bash scripts/make_f01_danger_fx_evidence.sh         # scene/: stills at 640x360 and 1920x1080 (two passes), the dynamite GIF
```

## What is in this folder

| Folder | Content |
| --- | --- |
| `dfx_<sheet>/` (6) | per grid: `contact_sheet_x1.png`, `contact_sheet_x4.png` (every frame, key frames marked), `gif_<tag>_x1.gif` and `_x3.gif` at tick timing, `tell_attack_strip.png` (the state strip with tick windows), `scene_640x360.png` and `scene_1920x1080.png` (the frames on the lit Floor 1 lane next to Carl and Donut; a toolkit composite), `SHA256SUMS` |
| `review_pack/dfx_<sheet>/` (6) | the three-image review pack (contact sheet x4, lit scene x3, state strip) with the validator report (every sheet: PASS) |
| `scene/` | twelve in-scene stills at 640 x 360 and at 1920 x 1080, the dynamite beat GIF, `capture_lines.txt`, `SHA256SUMS` |

## Frames and the state-to-tag map

| Grid | Canvas | Pivot | Frames | Sim state -> tag (timing at 60 Hz) |
| --- | --- | --- | --- | --- |
| `dfx_acid` | 80 x 10 | (0, 9) | 6 | `AcidPatch` Full -> `acid_idle` (4 x 8 ticks, world tick + 5 x patch id); Draining -> `acid_drain` (2 x 6); cropped to the live rectangle from both caps |
| `dfx_ring_r48` | 103 x 103 | (51, 51) | 4 | Rot Sticker Warning with ticks left (HAZ-Q38 `t..t+23`) -> `ring_march` (2 x 4); last 6 ticks -> `ring_hot` (2 x 2) |
| `dfx_ring_r64` | 135 x 135 | (67, 67) | 4 | Carl's stick landed and burning -> `ring_march`; last 6 ticks of the fuse -> `ring_hot` |
| `dfx_wave_r48` | 103 x 103 | (51, 51) | 6 | detonated charge with a 48 px outer radius (Rot Sticker, Boiler drum) -> `blast_wave` once from `detonate_tick` (2, 2, 2, 6, 5, 5 ticks) |
| `dfx_wave_r64` | 135 x 135 | (67, 67) | 6 | detonated charge with a 64 px outer radius (thrown dynamite) -> `blast_wave` |
| `dfx_gate` | 28 x 224 | (0, 223) | 6 | boss gate Closed -> `gate_drop` (frames 2..5, 2 ticks each) for 8 ticks from the close tick, then `gate_closed` over `{0, floor, 28, 224}`; Open or Released -> `gate_raised` (24 rows under the lintel, behind the actors) |

Frozen keys (`tests/fixtures/f01-dfx-*_keys.json`): acid 0 (unchanged by HZB-AD1: the digest is of rows), both rings 0 (re-pinned for HZB-AD2), both waves 3 (the reach frame), gate 0.

## In-scene stills (`scene/`)

Room 4 (`--f01-room hazards --room-pilot haz_spectacular`): the slime under the door is defeated on tick 164 (its pool drains on 164..223), sticker 0 is provoked on 373 and blasts on 397, Carl's stick lands on 522 and blasts on 634, chaining sticker 1 (blast on 642). Room 5 (`--f01-room juicer --room-pilot juicer`): the gate closes on tick 21, the Juicer crashes into the left wall on 139.

| File (`_640x360.png`, `_1920x1080.png`) | Tick | Shows |
| --- | --- | --- |
| `acid_full` | 120 | the full acid pool beside the slime, Carl and Donut at the door |
| `acid_drain` | 190 | the pool draining after the slime's defeat (narrower, duller, fuming) |
| `rot_ring` | 385 | the warning sticker's 48 px marching reach ring |
| `rot_ring_hot` | 394 | the same ring solid hot, 3 ticks before the blast |
| `rot_wave_reach` | 404 | the Rot blast: the 48 px wave on its reach frame around the VFX-01 burst |
| `dyn_ring` | 560 | Carl's landed stick with its 64 px reach ring |
| `dyn_ring_hot` | 630 | the ring solid hot, 4 ticks before the blast |
| `dyn_wave_front` | 636 | the dynamite blast, the wave on its way out |
| `dyn_wave_reach` | 641 | the 64 px wave on its reach frame, the chained sticker's ring hot |
| `gate_raised` | 8 | room 5 before the party is in: the raised gate's teeth under the lintel |
| `gate_closed` | 30 | the closed gate over its collider (captured before F01-SIM-TICKS: a current run also shows the slam dust of the gate's impact on this tick, see below) |
| `gate_crash` | 140 | the Juicer crashed against the closed gate |

`dynamite_beat_640x360.gif`: ticks 516..670, every 2 ticks at game speed (landing, the burning ring, the hot ring, the 64 px wave, the chained sticker's ring and its 48 px wave).

`capture_lines.txt` holds one `DANGER_FX_STILL` line per still and size and one `DANGER_FX_GIF` line: every line ends `hash_match=yes` (`window_hash == headless_hash`) and every still `passes_byte_identical=yes` (two full capture passes, byte-identical PNGs).

## Simulation unchanged (read-only presentation)

- Every capture's window run and headless replay end on the same `COMBAT_STATE` hash (`capture_lines.txt`).
- `danger_fx_art::tests::drawing_never_changes_the_state_hash` draws every item on every tick of a hazard room with acid, a slime, a sticker and a thrown stick and compares the state hash and the event count with an undrawn twin.
- No golden, pinned hash, capture line or replay log was edited: `cargo test --locked --all-targets` and `bash scripts/ci_floor01_combat.sh headless` (every fixture replayed twice against the committed logs) pass unchanged on the PR head.

## Not covered

- The gate's drop is a snap (the simulation closes it on one tick and keeps no close tick). The spent sticker's drip is drawn by the AREA-HAZARDS-01B art now (the code-drawn stand-in remains only as the missing-atlas fallback). See the spec's follow-ups.
- The committed Boiler GIFs and the hazard first look were regenerated in this PR (two passes byte-identical each, capture lines and pilot hashes unchanged): the drum's chain clips and the first look now show the danger FX art.
- Human pad play, 2P captures and a real-GPU frame time were not run.
