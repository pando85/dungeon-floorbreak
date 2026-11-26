# F01-SIM-TICKS evidence: Boiler landing dust, room 5 gate drop and slam dust

Prototype art (`review_status: prototype_fixture_unapproved`); nothing here approves art, tuning or game feel. Spec: [f01-sim-ticks-spec.md](../../02-art/f01-sim-ticks-spec.md). Independent art-director review pending; owner art and game-feel review pending.

The simulation now records two unhashed observer ticks, `FallingMass::landed_tick()` and `Boss::gate_closed_tick()`; the presentation draws the dust and the gate drop from them. The stills and GIFs in `scene/` are real windowed captures of `floor01_scene` (the Bevy presentation) at gameplay scale (640 x 360, `--scale 1`), one frame per tick. Each sequence was captured twice: every frame of the two passes is byte-identical, and each run's final window state hash equals its headless replay's (`capture_lines.txt`).

## Regenerate

```bash
python3 scripts/author_danger_fx.py --meta          # grids, per-grid configs, manifests
bash scripts/export_assets.sh                       # .ase from the grids, atlases under generated/art
c=assets/source/props/floor01/dfx_slam_dust.pixelart.json
python3 -m scripts.pixelart evidence "$c" --out docs/evidence/f01-sim-ticks/dfx_slam_dust
python3 -m scripts.pixelart review-pack "$c" --out docs/evidence/f01-sim-ticks/review_pack/dfx_slam_dust
cargo build --locked --bin floor01_scene
bash scripts/make_f01_sim_ticks_evidence.sh         # scene/: stills, GIFs, capture_lines.txt, SHA256SUMS
```

The gate's toolkit evidence (with the new `gate_drop` GIFs) is in [f01-danger-fx/dfx_gate](../f01-danger-fx/dfx_gate) and its review pack in [f01-danger-fx/review_pack/dfx_gate](../f01-danger-fx/review_pack/dfx_gate).

## What is in this folder

| Folder | Content |
| --- | --- |
| `dfx_slam_dust/` | `contact_sheet_x1.png`, `contact_sheet_x4.png` (the key frame marked), `gif_slam_dust_x1.gif` and `_x3.gif` at tick timing, `tell_attack_strip.png`, `scene_640x360.png` and `scene_1920x1080.png` (a toolkit composite on the lit lane), `SHA256SUMS` |
| `review_pack/dfx_slam_dust/` | the three-image review pack with the validator report (PASS) |
| `scene/` | the in-scene stills and one GIF per effect (every tick), `capture_lines.txt`, `SHA256SUMS` |

## In-scene captures (`scene/`)

| Sequence | Run | Ticks | Window hash = headless hash | Passes byte-identical |
| --- | --- | --- | --- | --- |
| `boiler_landing` | `--f01-room boiler --room-pilot chain` | 693..716 | `ad46d76a2e42ee3e` = `ad46d76a2e42ee3e` | yes |
| `gate_drop` | `--f01-room juicer --room-pilot juicer` | 19..50 | `c82e0527d3da7e80` = `c82e0527d3da7e80` | yes |

In the `chain` plan the weight starts falling on tick 688 and lands on tick 695. In the `juicer` plan the gate closes on tick 21, the grate hits the floor on tick 29 and the dust plays on 29..48.

| Still (`_640x360.png`) | Shows |
| --- | --- |
| `boiler_landing_t694` | the weight one tick before it lands, no dust |
| `boiler_landing_t695` | the landing tick: the remnant down, both plumes on their burst frame |
| `boiler_landing_t697` | the rolling head (the key frame) on both sides |
| `boiler_landing_t700`, `_t704`, `_t710` | billow, thin, settle |
| `boiler_landing_t715` | the last dust tick (settle haze) |
| `boiler_landing_t716` | the dust gone |
| `gate_drop_t20` | the gate raised, one tick before the close |
| `gate_drop_t21` | the close tick: the jamb in place, the grate on its first drop frame (still under the lintel and the HUD) |
| `gate_drop_t23`, `_t25`, `_t27` | the grate falling, faster every frame |
| `gate_drop_t29` | the impact: the closed grate and the slam dust's burst into the arena |
| `gate_drop_t31` | the rolling head (the key frame) |
| `gate_drop_t34`, `_t38`, `_t44` | billow, thin, settle |
| `gate_drop_t49` | the dust gone |

The GIFs `boiler_landing_640x360.gif` and `gate_drop_640x360.gif` play the full sequences at one frame per tick.

## Tests

- `src/env_prop.rs`: `step_at_records_the_landing_tick_once_unhashed_and_reset_clears_it`.
- `src/boss.rs`: `the_gate_records_its_close_tick_once_unhashed`.
- `src/danger_fx_art/tests.rs`: `the_boiler_weight_throws_its_landing_dust_on_the_landing_tick`, `the_gate_drops_from_its_close_tick_and_slams_dust_on_the_impact`, `the_slam_dust_plays_once_and_mirrors_about_its_pivot`, the tag table and `drawing_never_changes_the_state_hash`.
- `tests/test_art_danger_fx.py`: the sources re-derived byte for byte, `test_the_gate_drop_falls_from_the_raised_height_to_the_floor`, `test_the_slam_dust_stands_on_the_floor_and_rolls_away_from_the_impact`.
