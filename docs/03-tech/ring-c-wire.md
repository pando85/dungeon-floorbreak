# RING-C wire: the moving shutter of `ring_court`

Status: implemented and tested headless; game feel is pending the owner's review ("merged, game-feel review pending"). Every number below is data in `assets/levels/floor01/f01_ring_court.encounter.json` and a tuning knob.

Designed in `docs/07-agents/f01-playtest-integration-plan.md` section 15 (KR2, KR3, KR4, KR5, KR6, KR7, KR18, KR19, KR21). The machine (`Shutter`, `ShutterConfig`) is RING-B/RING-C core in `src/env_prop.rs`; the presenter frames (`floor01_ring_*`, `src/ring_shutter_art.rs`) are AREA-RINGS-01. This step runs the machine in the env step and reads it back in the presenter.

## Behaviour

Room 10, `ring_court` (`--scene f01_late --f01-room ring_court`, and every `--f01-run`), runs a **moving** shutter by default:

| Phase | Ticks | What happens |
| --- | --- | --- |
| open | until armed | no slice stands; the lane under the 80 px walkway is open |
| warning | 40 | armed: the tell; nothing moves |
| closing | 34 | the lower edge descends 2 px per tick (68 px); a slice (4 px, 17 in all) turns intact once the edge has passed its bottom |
| closed | 150 | all 17 slices stand under the walkway slab |
| opening | 34 | the edge rises 2 px per tick; slices turn non-intact from the bottom |
| open | for good | `cycles` is 1: one closure per attempt |

The unheld blocked window (closing, closed and opening) is 34 + 150 + 34 = 218 ticks, under the 300-tick stuck limit (KR19).

**Arm rule (KR18).** The warning arms on the first tick on which a living hero has crossed `arm_x` (1008, 192 px before the arch) **and** the trailing living hero is within `arm_trail_px` (320) of the shutter. A position bound, never a camera read. If the trailing hero stays back, the shutter stays open.

**Hold rule (KR4).** While a living body overlaps the band the edge would occupy next, the edge does not move: nobody is squeezed, no slice turns intact over a body, heroes are never hurt or pushed. The hold has no time bound. Carl (52 px) under the edge holds it; a goblin (38 px) or Donut under it can hold the lane shut for Carl, and the walkway over the arch is the crossing (KR3).

**Pin (KR4).** A `Neutral` or `Attack` goblin under the edge is staggered for `pin_ticks` (36) with no damage, once per closure (one token per enemy). A launched goblin that hits the shutter's face takes the existing CF02-B slam and spends its token. A pin never overwrites `Hurt`, `Launched`, `Downed`, `Airborne`, `Down` or `Corpse`.

**Retry (RING-10).** A restart or a checkpoint respawn resets the shutter to open with its cycle and tokens unspent; no slice stands in the world.

## Wiring

* `src/combat_scene/env.rs`: `EnvRuntime::new` builds a moving shutter as 17 `EnvSolidKind::ShutterSlice` blocks (`Material::Timber`, so the companion's nav stays incremental); a static one stays one `ShutterStack` block (RINGW-AD8). `EnvRuntime::step` arms and steps a moving shutter (hero centres, per-body states) and returns the pins in `EnvTick::pins`. `CombatState::step_env` maps goblin modes to body states and writes the pin to the public goblin fields. The KR6 sync rewrites each slice's `intact` from the machine every tick.
* `src/f01_ring.rs`: `GateMode` and `launch_mode`.
* `src/bin/floor01_scene.rs`: `--ring-static-gate`.
* The presenter (`env_prop_art::sync_ring_shutter`) already reads `phase` and `edge_px` from the env step through a shared reference; it needed no change, and the window state hash equals the headless hash (evidence).

## Switch for comparison

`--ring-static-gate` (a floor run keeps it through its relaunches) restores the RING-B static closed gate: one block, `cycles` 0. Libraries use `f01_ring::launch_mode(.., GateMode::Static)`. A PLAYTRACE recording made under the flag does not verify under the default build.

## Adopted defaults (RINGC-AD*)

| ID | Adopted value | Source |
| --- | --- | --- |
| RINGC-AD1 | the court's sidecar authors `initial: open` and `arm_x: 1008`; every other knob is the core default (warning 40, 2 px per tick, slice 4, cycles 1, pin 36, dwell 150, trail 320) | KR2, KR18, KR19; `arm_x` 192 px before the arch is the value the validator test of RING-B core already proves |
| RINGC-AD2 | timings 40 / 34 / 150 / 34 ticks; blocked window 218 | KR2, KR19 |
| RINGC-AD3 | one closure per attempt (`cycles` 1) | KR5 |
| RINGC-AD4 | the moving stack is 17 Timber slice solids; the static gate is one block | KR6, RINGW-AD8 |
| RINGC-AD5 | goblin `Neutral` and `Attack` are `Free`; `Hurt`, `Launched`, `Downed`, `Airborne` are `Busy`; `Down` and `Corpse` are `Gone`; llamas and slot enemies are `Busy`; a pin is `contact_stagger` with no push and the facing kept | KR4 |
| RINGC-AD6 | on by default in `--f01-run` and `--f01-room ring_court`; `--ring-static-gate` restores the static gate | owner rule: new behaviour on by default, with a switch for comparison |
| RINGC-AD7 | `LaunchedBody` gains `source` (the goblin's id) so the face hit can spend its token: a one-field edit of `contacts_cf02.rs`, where KR7 hoped for none; the alternative was matching a goblin by its box | KR4, KR7 |
| RINGC-AD8 | no new `CombatEvent` and no journal line; the phase and the tokens are hashed shutter state, and the env token of the state line names the phase | KR21 |

## Open questions (RINGC-Q*)

| ID | Question | Recommendation |
| --- | --- | --- |
| RINGC-Q1 | Does a single closure read as a threat, or should the shutter re-arm (`cycles` 2) once the party retreats? | keep 1; revisit after play |
| RINGC-Q2 | Is the 40-tick warning readable on a 640x360 window without an audio cue? | game-feel review; RING-D adds cues |
| RINGC-Q3 | Is the 150-tick closed dwell too long when a goblin pinned on the far side is the only reason to wait? | keep; the walkway is always open |
| RINGC-Q4 | Should the autonomous partner prefer the walkway while the shutter is armed or closing? | no companion code (KR19); she crosses either under the open arch or over the walkway through the nav |

## Tests

`src/ring_c_wire_tests.rs` (phase cycle and per-tick slice sync, the trailing-hero arm rule, the pin and the held edge, a hero under the edge, the court traversal for Carl, Donut and the companion at four seeds, the held-edge fixture for Carl and the partner within 300 ticks, retry, and the incremental nav equal to a fresh build through a full cycle), `src/f01_ring.rs` (both modes), `src/combat_scene/env.rs` (the solids of both modes), and the unchanged RING-B fixtures, which now run under `GateMode::Static` where they assert a closed shutter. Evidence: `docs/evidence/ring-c-wire/`.
