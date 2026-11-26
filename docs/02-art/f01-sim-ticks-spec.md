# F01-SIM-TICKS: Boiler landing dust, room 5 gate drop and slam dust (design spec)

**Status:** design spec, one frozen key (the slam dust's rolling head, `slam_dust` frame 1), every frame, the sources, manifests, validators, the two simulation ticks, the presentation-only wiring and the window captures are done. Evidence: [docs/evidence/f01-sim-ticks](../evidence/f01-sim-ticks/README.md). Independent art-director review pending; owner art and game-feel review pending. Not owner-approved.

**Scope.** Two moments of weight that snapped without any impact in the Floor 1 scene:

1. the Boiler's hanging weight (`FallingMass`, room 2) lands and the floor shows nothing;
2. the room 5 entrance gate snaps from raised to closed on one tick (the [danger FX spec](floor01-danger-fx-spec.md) adopted default DFX-AD7, kept only because the simulation kept no close tick).

The work order adds the two ticks to the simulation as **unhashed observer fields** and draws, from them, the landing dust, the gate drop and the gate's slam dust. No collider, rectangle, state hash, golden or pin moves.

**Route.** Difficulty score 1 to 2 (one small new VFX sheet of five frames and four extra frames of an existing sheet that are a rigid shift of its frozen closed frame), class Easy, route **C** collapsed ([routing by difficulty](../07-agents/pixel-art-pipeline.md#routing-by-difficulty)): this spec plus one frozen key (the dust's rolling head); the remaining dust frames and the drop frames were authored in the same pass because they follow mechanically from the key and the closed frame. Spec and keys never name who drew them; provenance is in the manifests and the PR.

**Authorities.**

- **Sim state:**
  - Weight: `src/env_prop.rs` (`FallingMass::step_at`, `landed_tick()`, `remnant()`; the landing is the `MassEdge::Landed` edge, logged as `CombatEvent::PropPhase { phase: Landed }`), called from `src/combat_scene/env.rs` with the combat step tick.
  - Gate: `src/boss.rs` (`Boss::gate_closed_tick()`, set in `step_gate` on the tick it closes the gate and logs `CombatEvent::BossEngaged`; collider `boss_scene::gate_block`, x `0..solid_x` = 28).
- **Current art:** `dfx_gate` ([danger FX spec](floor01-danger-fx-spec.md) section 2.4) and the Goblin Workshop remnant (`env_prop_art`, 64 x 32 over the landed weight's rectangle).
- **Pure wiring:** `src/danger_fx_art.rs` (`landing_dust`, `gate`, `gate_dust`, `dust_frame`, `Layer::Dust`); Bevy half `src/bin/floor01_scene/danger_fx_art.rs`.
- **Hazard and VFX family:** [area-hazards-01b-spec.md](area-hazards-01b-spec.md) (binary alpha, kit palette, presenters hash-neutral, one shot per event, no glow on inert matter).
- **Tone:** [antihero tone](../01-design/antihero-tone.md) and [art direction](art-direction.md).

**Horizon check.** Presentation plus two unhashed observer fields: no move, ability, unlock or terrain-combat rule changes, so the [living horizon](../01-design/character-progression-horizon.md) needs no update.

## 1. Tone and readability rules

- **Dirt, not magic.** The dust is old stone grit and mortar dust kicked off a dirty floor: the kit's warm stone ramp, no glow, no sparkle, no colour shift. It says "that was heavy", nothing more.
- **Low and fast, then slow.** The plume hugs the floor and runs away from the impact, curls into a rolling head, puffs up, breaks into separate puffs and settles as a haze line. It never rises above 20 px; it is a floor event, not an explosion.
- **The gate is dropped, not lowered.** The grate falls with gravity (accelerating), hits the floor and stops dead; the dust comes on the hit, not on the close.
- **Never hide the fight.** The dust draws over the props and under the actors and their tells, so a hero, an enemy or a ring stays on top.

## 2. Simulation ticks (unhashed observer fields)

| Field | Set | Value | Hashed |
| --- | --- | --- | --- |
| `FallingMass::landed_tick` | once, by `step_at(tick, ..)` on the step whose edge is `MassEdge::Landed`; cleared by a reset | the combat step tick, the same tick as the `Landed` event and the scene's `world.tick` after that step | no (excluded from `mix_into`, like `Boss::charge_started`) |
| `Boss::gate_closed_tick` | once, in `step_gate` with `Gate::Closed` | `ctx.tick`, the tick of `BossEngaged` | no |

Both are copies of a tick taken at a hashed edge, so they add no information the hashed state does not already order; a replay reproduces them exactly.

## 3. Sheets

### 3.1 Slam dust (`dfx_slam_dust`): new

Canvas 44 x 20, pivot (4, 19) (the impact edge on the floor row; the plume rolls to the right, mirrored for the left side). Tag `slam_dust`, once, five frames:

| Frame | Ticks | Shape |
| --- | --- | --- |
| 0 burst | 2 | a flat, fast jet hugging the floor (to x about 28), the first grit flying ahead |
| 1 roll (**KEY**) | 3 | the front curls up into a rolling head about 9 px high, grit arcing over it |
| 2 billow | 4 | puffed up and slowing (to about 15 px), the grit down on the floor ahead, darker body |
| 3 thin | 5 | risen, darker, torn into separate puffs |
| 4 settle | 6 | three high wisps fading and a haze line on the floor |

20 ticks in all. Colours: lit `stone_07`/`stone_06`/`stone_05` for frames 0 and 1, then one step darker per phase down to `stone_04`/`stone_03`; grit chunks in `stone_03`, `iron_03` and `iron_01`. The floor row 19 is painted in every frame and every frame reaches back to the pivot column (x at most 4), so the plume always starts at the impact edge.

### 3.2 Gate drop (`dfx_gate`, tag `gate_drop`): four frames added

Frames 2..5 of the gate sheet, 2 ticks each: the closed grate (frame 0) lifted 192, 168, 128 and 72 px (an accelerating fall from the raised height of 200 px, a rigid shift of its rows so the rust and the grime ride with the bars), with the jamb channel in place from the first drop frame. Frames 0 (closed) and 1 (raised) are unchanged, so the frozen gate key (frame 0) is unchanged.

## 4. Presenter rules (presentation only, hash-neutral)

- **Boiler landing dust.** For every `FallingMass` with a `landed_tick` and a remnant, two plumes from the landing tick for 20 ticks: one rolling right from the remnant's right edge (`x + w`), one mirrored rolling left from its left edge (`x - 1`), both on the remnant's floor. Age 0 is the landing tick itself (the first presented frame after the landing step).
- **Gate drop.** While the gate is closed, its age is `tick - gate_closed_tick`: ages 0..7 draw `gate_drop` frames 2, 2, 3, 3, 4, 4, 5, 5 on the gate layer; from age 8 the closed frame. Without a close tick (a state built without stepping) the closed frame draws at once. The collider is solid from the close tick, as before; only the picture falls.
- **Gate slam dust.** From age 8 (the grate's impact) for 20 ticks, one plume rolling into the arena from the gate's arena face (x 28) on the boss floor.
- **Layer.** A new `Layer::Dust`, after `Floor` and before `Splash`; in Bevy `actor_z - 0.03`: over the props (`actor_z - 0.05`), under the actors.
- **Read-only.** `draws` reads shared references only; the existing test that draws every tick and compares the state hash with an undrawn twin covers the new effects, and every capture ends `window_hash == headless_hash`.

## 5. Palette

Named entries of the Floor 01 kit palette, values exact: `stone_03`..`stone_07`, `iron_01`, `iron_03` for the dust; the gate drop reuses the closed frame's pixels. Binary alpha in every source.

## 6. Acceptance targets

1. The frozen key is byte-identical to `tests/fixtures/f01-dfx-slam-dust_keys.json`; no isolated pixels; binary alpha; palette as section 5 (toolkit validators pass on both configs).
2. Every drop frame is a pure vertical shift of the closed grate, its lowest row accelerating toward the floor, the jamb identical to the closed frame's (`tests/test_art_danger_fx.py`).
3. Each tick is set exactly once, at the right tick, deterministically, and leaves every state hash unchanged (unit tests in `src/env_prop.rs` and `src/boss.rs`).
4. The landing dust spawns on the landing tick, both plumes on the remnant's floor at its edges, and is gone at age 20; the gate draws raised before the close, the drop on ages 0..7, the closed grate and its dust from age 8, the dust gone at age 28 (`src/danger_fx_art/tests.rs`).
5. At 1x (640 x 360) in the Boiler and in room 5: the dust reads as floor grit thrown by a heavy impact, not a magic effect or smoke; the gate reads as a grate dropped on the party.
6. No state hash, golden or pin moves (`window_hash == headless_hash` on every capture; the full test suite and `ci_floor01_combat.sh headless` pass with no golden edits).
7. No text, logos or third-party designs; no concept traced.

## 7. Adopted defaults

| ID | Default | Why |
| --- | --- | --- |
| ST-AD1 | The two ticks are unhashed observer fields (`landed_tick`, `gate_closed_tick`), excluded from the state hash like `Boss::charge_started` | Copies of a tick taken at a hashed edge; no golden, hash or pin moves |
| ST-AD2 | The gate collider stays solid from the close tick; only the picture drops, over 8 ticks (four frames of 2 ticks, an accelerating fall) | Presentation may not change outcomes; 8 ticks reads as a drop at 60 Hz without delaying the lock-in visibly |
| ST-AD3 | The jamb channel is drawn from the first drop frame | The jamb is part of the wall, not the grate; it should not pop in on the impact |
| ST-AD4 | The dust draws on its own layer, over the props and under the actors (`actor_z - 0.03`) | The dust rolls over the remnant's base and the drum but never hides a hero or an enemy |
| ST-AD5 | The Boiler landing throws two mirrored plumes, one from each edge of the remnant | The weight lands flat; dust escapes on both sides |
| ST-AD6 | The gate's dust starts on the impact (close + 8), from the gate's arena face, rolling into the arena only | The grate seals the doorway; nothing rolls into the closed corridor the camera does not show |
| ST-AD7 | The slam dust plays once over 20 ticks (2, 3, 4, 5, 6); frame 1 is the frozen key | Fast start, slow settle; the rolling head is the most characteristic silhouette |
| ST-AD8 | Kit stone colours, no glow | Inert matter in the hazard/VFX family has no light of its own |
| ST-AD9 | DFX-AD7 (the gate has no drop animation) is superseded | The simulation now keeps the close tick |

## 8. Open questions

- Should the landing or the gate impact also shake the camera or play a sound? Out of scope here (camera and audio are separate work orders); the ticks now exist for them.
- The right Boiler plume rolls over the drum's base, which stands right next to the remnant; the owner's art review decides whether it should be shortened there.
