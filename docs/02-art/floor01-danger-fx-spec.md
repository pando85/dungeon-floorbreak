# F01 danger FX: acid pool, blast reach rings, blast waves and the room 5 gate (design spec)

**Status:** design spec, 6 frozen keys, every frame, export, manifests, validators, the presentation-only wiring and the evidence are done. Independent top-tier art-director review passed (fix round applied); owner art and game-feel review pending. Evidence: [docs/evidence/f01-danger-fx](../evidence/f01-danger-fx/README.md).

**Scope.** Four Floor 1 danger visuals that were still code-drawn stand-ins in the scene: (1) the acid pool of the hazard rooms, (2) the blast reach ring (the Rot Sticker warning ring, and Carl's dynamite), (3) the dynamite blast size (the blast art must show the real 20 px core / 64 px ring reach), (4) the room 5 entrance gate (the Juicer's arena). Nothing here changes the simulation, a rectangle, a hash or a golden.

**Route.** Difficulty score 2 for each sheet (new design; acid, rings and waves carry VFX; the gate is 224 px tall), class Medium. Design and execution were done in one pass; the silhouette gate is folded into the independent review (adopted default DFX-AD9). Spec and keys never name who drew them; provenance is in the manifests and the PR.

**Authorities.** Sim state: acid `src/slime.rs` (`AcidPatch`, HAZ-Q18/Q21: 6 px volume, 32..80 px wide, symmetric 60-tick drain), Rot warning and ring `src/rot_sticker.rs` and `src/hazards_art.rs` (HAZ-Q37/Q38: ring on `t..t+23`, outer radius `hazq30_outer_radius_px` = 48), dynamite `src/thrown_charge.rs` (CP-Q20: core 20, outer 64; fuse 150 ticks), blast `src/blast.rs` (`BlastCharge::centre`, 8 px above the walking line), drum `src/combat_scene/env.rs` (outer = `blast_radius_px` 48), gate `src/boss.rs` (`Gate::{Open, Closed, Released}`; collider `boss_scene::gate_block`, x `0..solid_x` = 28, closing the whole height; the art covers it from the walking line up to the 224 px arena ceiling, the rectangle the stand-in slab drew). Design: [enemy-gameplay-slime-rot.md](../01-design/enemy-gameplay-slime-rot.md), [carl-dynamite-anim-03-spec.md](carl-dynamite-anim-03-spec.md) (open question 8: a blast above about 26 px needs native large art), [vfx-01-spec.md](vfx-01-spec.md) section 8, [art direction](art-direction.md), [antihero tone](../01-design/antihero-tone.md), the [Floor 01 kit palette](../../assets/source/terrain/floor01/floor01_catacomb_kit.json).

**Sources.** `assets/source/props/floor01/dfx_*.grid.json` (+ `.ase` built from them, one `*.pixelart.json` config and one `assets/manifests/f01-dfx-*.json` manifest per grid), all written by [scripts/author_danger_fx.py](../../scripts/author_danger_fx.py) (deterministic; `tests/test_art_danger_fx.py` re-derives every committed file byte for byte). Pure wiring [src/danger_fx_art.rs](../../src/danger_fx_art.rs), Bevy half `src/bin/floor01_scene/danger_fx_art.rs`.

## 1. Tone and readability rules

1. **A danger reads before it hurts.** The acid is always on (it is its own warning: lit liquid surface, bubbles, fumes). The blast reach is shown on the ground truth *before* the blast: the ring marches for the whole warning or fuse and closes into one solid hot band for the last 6 ticks (`RING_HOT_TICKS`). The gate shows its teeth under the lintel while it is still open.
2. **The art is the simulation's shape or it is not drawn.** Rings and waves are generated per sim radius (48 px and 64 px canvases); a radius without its own canvas keeps the code-drawn ring dots and the VFX-01 burst alone. The acid is cropped to the pool's live rectangle every tick, and drawn only for the 6 px volume height its canvas is made for (another `hazq18_volume_height_px` keeps the code-drawn strip). The gate canvas is exactly the closed gate's collider between the walking line and the arena ceiling.
3. **Grim, practical, no cartoon.** Hazard paint is the kit's abstract orange/yellow dashes on a soot line (no symbols, no text); the blast wave is fire cooling to warm catacomb dust, not stars or rings of sparkles; the gate is a riveted, rusting drop grate, not a castle portcullis with ornaments.
4. **Warnings are never hidden.** Rings draw on the tell layer above everything; the wave draws behind the actors (VFX-01 rule: the actors silhouette against the fire); the acid sits under the slime and the actors like the stand-in strip did.
5. Light from the upper left; dark outline (`iron_00`, slime `outline_murk`) on every edge that meets the background.

## 2. Sheets

Coordinates: canvas px, origin top-left. Ticks at 60 Hz, `duration_ms = round(ticks * 1000 / 60)`.

| Grid | Canvas | Pivot | Tags (frames x ticks) | Frozen key |
| --- | --- | --- | --- | --- |
| `dfx_acid` | 80 x 10 | (0, 9) | `acid_idle` 4 x 8 loop, `acid_drain` 2 x 6 loop | 0 |
| `dfx_ring_r48` | 103 x 103 | (51, 51) | `ring_march` 2 x 4 loop, `ring_hot` 2 x 2 loop | 0 |
| `dfx_ring_r64` | 135 x 135 | (67, 67) | `ring_march` 2 x 4 loop, `ring_hot` 2 x 2 loop | 0 |
| `dfx_wave_r48` | 103 x 103 | (51, 51) | `blast_wave` 6 frames, 2, 2, 2, 6, 5, 5 ticks, once | 3 (reach) |
| `dfx_wave_r64` | 135 x 135 | (67, 67) | `blast_wave` 6 frames, 2, 2, 2, 6, 5, 5 ticks, once | 3 (reach) |
| `dfx_gate` | 28 x 224 | (0, 223) | `gate_closed` 1 held, `gate_raised` 1 held | 0 |

A ring or wave canvas is `2 * (R + 3) + 1` px: odd, so its centre is one pixel, placed on the world pixel the code-drawn ring dots are centred on (the dome centre, the blast centre).

### 2.1 Acid pool (`dfx_acid`)

**Sim state it draws:** each `AcidPatch` with a volume: `Full` plays `acid_idle` (offset by the patch id, so two pools do not pulse together), `Draining` plays `acid_drain`, `Removed` draws nothing. The volume is 6 px tall; canvas rows 4..9 are the volume, rows 0..3 are bubbles and fumes standing above it.

**Design.** A shallow pool of the slime's own gel: lit surface line (`gel_3_light`) with rolling 3 px sheen streaks (`sheen_torch`, 2 px per frame), horizontal liquid layers below it (`gel_2_mid`, `gel_1_shade`, `gel_0_deep` runs of 4 px, not a dither), an `outline_murk` bottom and side wall, a 2 px meniscus at each end, sparse `acid_hot` sparks under the surface, five bubble sites cycling rise, swell, pop (a fume wisp `bone_gel_lit`/`bone_gel_dark`). Draining: the surface one step duller, no sparks, the gel darker, more fumes rising (it is boiling off), four bubble sites.

**Crop rule (footprint exact).** For a live rect `x..x + w`, the presenter draws the canvas columns `0..ceil(w/2)` at `x` and the columns `80 - floor(w/2)..80` right after them, so both meniscus ends sit exactly on the volume's ends at every drain step (a pool wider than 80 px, not authored today, is filled from middle columns, never stretched).

### 2.2 Blast reach ring (`dfx_ring_r48`, `dfx_ring_r64`)

**Sim state it draws:** the Rot Sticker in `Warning` on every tick with at least one tick left (HAZ-Q38, `t..t+23`; a chained sticker shows its last 8), centred on the dome, radius `hazq30_outer_radius_px` (48); Carl's stick once it has **landed** and burns, until its detonation tick, centred on `charge.centre()`, radius the profile's outer radius (64). The frame is `ring_hot` when `remaining <= 6`, else `ring_march`, both on the world tick.

**Design.** Hazard tape drawn on the reach: a 2 px band at `R-1..R+1`, 8 of every 12 px painted, alternating `flame_02` and `brass_05` dashes, every yellow dash carrying a 2 px tooth pointing at the centre ("inside is the blast"), and a 1 px `iron_00` soot line on both sides of the paint so it holds on warm brick and dark stone alike. Frame 2 shifts the dashes half a dash (it marches, like the Boiler drum's floor ring). `ring_hot`: the dashes close into one solid band (`flame_03`/`flame_04`, flickering to `flame_04`/`flame_05`), outlined.

### 2.3 Blast wave (`dfx_wave_r48`, `dfx_wave_r64`): the dynamite blast size

**Sim state it draws:** every detonated charge whose profile's outer radius has a wave canvas, from its own `detonate_tick` (age 0) for 22 ticks, centred on `charge.centre()`: the thrown dynamite (core 20, outer 64), the Rot Sticker (outer 48), the Boiler drum (outer 48). The VFX-01 `burst` (48 x 48, the fireball of the 20 px core) still plays unchanged on top; the wave is the native large art the dynamite spec asked for above 26 px.

**Design.** One shock front per frame, cooling as it travels: f0 white-hot ring leaving the core at 0.42 R; f1 0.64 R yellow with flame tongues behind; f2 0.84 R orange, first soot; **f3 the reach (key, 6 ticks):** the bright `flame_03` edge ends exactly on `R` (pixels at `R-2..R`), a 1 px `iron_00` soot rim on `R..R+1`, tongues of `flame_02..flame_00` and soot behind; f4 the front breaks into rolled catacomb dust clouds (`stone_06`/`stone_04`) and embers on the reach; f5 thin settling dust. No pixel is ever drawn beyond `R + 1`. Broken bands use a fixed integer hash (no randomness), so every run draws the same pixels.

The wave is a full circle: the simulation tests a circle (bodies are hit by distance), so the art shows the whole circle, also where it crosses the floor face (adopted default DFX-AD6).

### 2.4 Room 5 entrance gate (`dfx_gate`)

**Sim state it draws:** `gate_closed` over `{0, floor, 28, 224}` (the gate collider up to the arena ceiling) while `Boss::gate_closed()`; `gate_raised` (rail and teeth hanging 24 rows under the top of the doorway, behind the actors) while the gate is `Open` (before the party is in) and once `Released` (after the defeat).

**Design.** A drop grate: five 3 px cold-steel bars (`stone_cold_04` lit, `stone_cold_03`, `stone_cold_01` shade) with 2 px gaps, four riveted cross straps and a heavier bottom rail, every bar ending in a tapering spike that bites the floor on row 223, a jamb channel on the arena side (x 24..27, warm `iron_*`, a bolt every 32 rows), short rust runs (`copper_03`/`copper_02`) under some rivets and grime darkening toward the floor. Cold steel separates it from the warm brick at 1x (the Goblin Workshop kit's "interactive steel is cold" rule); it reads as a lock-in, not a door.

## 3. Palette

All kit sheets use named entries of the Floor 01 kit palette, values exact: 23 distinct kit colours across the five kit sheets (at most 63). The acid uses ten named entries of the SLIME-01 gel ramp (`outline_murk`, `gel_0_deep` .. `gel_4_lit`, `sheen_torch`, `acid_hot`, `bone_gel_lit`, `bone_gel_dark`), values exact as in `slime_01.grid.json` (adopted default DFX-AD1: the kit has no acid green, only the dark `lichen` ramp, and the pool must be the same fluid as the slime that stands in it). Alpha is binary in every source.

## 4. What stays code-drawn

- **Hazard footprints stay exact to the sim rect.** The acid art *is* the footprint (cropped to the live rectangle); no sprite is scaled to fit.
- The Boiler's steam warning area and edge and the drum's blast area outline (the Goblin Workshop kit's footprints) are untouched.
- The spent Rot Sticker's drip stays the code-drawn stand-in (out of this scope).
- A ring for a radius without a canvas (the Rot tuning range is 40..56) keeps the code-drawn ring dots; a wave for another radius is not drawn (the VFX-01 burst alone, as before). A gate collider other than 28 px keeps the slab.
- `--debug-combat` still draws the dynamite's dotted debug ring at the profile radius (it is a debug overlay, also during the flight).
- A missing atlas (`scripts/export_assets.sh` not run) prints one `DANGER_FX missing ...` line and that visual falls back to its stand-in.

## 5. Wiring contract (presentation only)

- `danger_fx_art::draws(state, layout_h, tick)` is a pure function of the combat state and the tick through shared references; the loops run on the world tick, the wave reads the charge's `detonate_tick`; no simulation timer is added, nothing is hashed or mixed. A unit test steps a hazard room with acid, a slime, a sticker and a thrown stick with and without drawing and compares the state hash on every tick.
- The Bevy half draws each item as a source rectangle of its atlas (one pool of 40 sprites): acid at the hazard pieces' z, the wave on the blast layer just under the VFX-01 burst, the closed gate at the boss slab's z, the raised gate behind the actors, the rings on the tell layer. The hazard presenter skips the stand-in acid strip, lip and ring dots, and the boss presenter skips the gate slab, only when the art for them loaded.

## 6. Acceptance targets

1. Frozen keys byte-identical to `tests/fixtures/f01-dfx-*_keys.json`; no isolated pixels; binary alpha; palette as section 3 (toolkit validators V01..V15 pass).
2. The drawn reach is the simulation's: every ring paint pixel lies in `R-3..R+1`, the wave never draws beyond `R+1`, the wave's reach frame has its bright edge on `R-1..R` and a soot rim on `R` (test `the_rings_and_waves_end_on_the_sim_radius`).
3. The acid covers exactly the live volume's x range at full width and at every drain tick, both caps on the ends (test `the_acid_covers_exactly_the_live_volume`).
4. The Rot ring shows on exactly the ticks the stand-in ring did; the dynamite ring only while the landed stick burns; both turn hot for the last 6 ticks; the wave plays 22 ticks from the detonation tick, once.
5. At 1x in the lit rooms (640 x 360): the acid reads as a corrosive pool, not a floor stripe; the ring reads as "the blast reaches here" before the blast; the 64 px wave visibly reaches farther than the VFX-01 fireball; the closed gate reads as a barrier and the raised teeth read as a gate that can drop.
6. The simulation, its rectangles, every state hash and every golden are unchanged (`window_hash == headless_hash` on every capture; the full test suite and `ci_floor01_combat.sh headless` pass with no golden edits).
7. No text, logos or third-party designs; no concept traced.

## 7. Adopted defaults

| ID | Default | Why |
| --- | --- | --- |
| DFX-AD1 | The acid uses the SLIME-01 gel ramp (named, exact), not the kit ramps | The kit has no acid green; the pool is the slime's own fluid and must match it at 1x |
| DFX-AD2 | Rings and waves are drawn natively per sim radius (48, 64); another radius keeps the stand-in | Integer art never stretches; the art must match the sim's reach or not claim it |
| DFX-AD3 | Carl's landed, burning stick shows its 64 px reach ring in normal play (before: only the `--debug-combat` dots); not while it flies | The danger must read before it hurts; the landing point is not settled in flight. Game-feel call for the owner |
| DFX-AD4 | The ring turns solid hot for the last 6 ticks (`RING_HOT_TICKS`), from the ticks left the sim already keeps | Escalation without a new timer |
| DFX-AD5 | The wave plays for every charge whose outer radius has a canvas: the dynamite (64), the Rot Sticker and the Boiler drum (48) | One rule: blast art matches the sim radius everywhere |
| DFX-AD6 | The wave is a full circle, also across the floor face | The sim's test is a circle; hiding the lower half would understate the reach on a ledge |
| DFX-AD7 | The gate has no drop animation; it snaps between raised and closed | The sim closes it on one tick and keeps no close tick; an animation needs one (follow-up) |
| DFX-AD8 | Gate in the kit's cold steel ramp with rust from the copper ramp | Separates the barrier from the warm brick at 1x |
| DFX-AD9 | Design and execution in one pass; the silhouette gate is folded into the independent review | Orchestrator brief; difficulty 2 per sheet |
| DFX-AD10 | The acid canvas is 80 px (the HAZ-Q18 maximum) and is cropped from both caps | Exact footprint at every width without scaling |

## 8. Follow-ups (gaps, not done here)

- A gate drop animation (needs the closing tick from the simulation, or a render-local memory that a replay seek would not reproduce).
- The spent sticker's drip and an acid entry splash are still code-drawn or absent.
- Done in this PR: the committed Boiler GIFs (`docs/evidence/f01-boiler`) and the hazard first look were regenerated with the wave and ring art (two byte-identical passes, state hashes unchanged).
