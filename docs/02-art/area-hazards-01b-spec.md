# AREA-HAZARDS-01B: acid palette, entry splash, corrosion and Rot marks (design spec)

**Status:** design spec, the palette decisions (HZB-AD1, HZB-AD2) and two frozen keys (the acid entry splash crown and the floor-face corrosion sample). The 1x readability gate passed on the room 4 and room 6 backgrounds (section 9). Executed: the remaining frames, the sources, manifests and fixtures, the validators and tests, the two pure presenters with their Bevy halves, and the room 4 and room 6 window captures are in [the evidence folder](../evidence/area-hazards-01b/README.md); execution defaults HZB-AD9 to HZB-AD16 are recorded in section 10. Review follow-ups: the wall trail anchor (HZB-AD17), the redrawn corrosion key (HZB-AD18) and the drip z are design changes still to apply in execution. Owner art review pending. Not owner-approved.

**Route.** Difficulty score 1 (small effects and decals, one new animated effect), class Easy, route **C**: this spec plus one hero key, the splash ([routing by difficulty](../07-agents/pixel-art-pipeline.md#routing-by-difficulty)). One corrosion sample is also frozen so that the decal language is fixed before execution. Work order AREA-HAZARDS-01B is the follow-up to [AREA-HAZARDS-01](../07-agents/floor-01-area-kits-plan.md) (kit plan line 21) and the open findings F1 and F5 of the [danger FX spec](floor01-danger-fx-spec.md) (backlog `F01-FU-B10`, `DANGER-FX-RING`). The work appears in rooms 4 (`f01_hazards`) and 6 (`mastery`).

**Authorities.**

- **Sim state:**
  - Acid: `src/slime.rs` (`AcidPatchSpec` `x0`, `x1`, `floor_y`; `AcidPatch::rect()`; `ACID_SOURCE_ID` 0x4143; `acid_source(patch_id)`).
  - Rot Sticker: `src/rot_sticker.rs` (`RotSticker::detonate_tick()`, `state()`, `is_spent()`, `track().orientation`, `dome(t)`).
  - Combat events: `CombatEvent::HeroHurt` with an acid source; `CombatEvent::EnvironmentHurt` with `class: HazardClass::Acid`, behind the `fu.acid_hurt_event` flag.
- **Room data:**
  - Room 4: `assets/levels/floor01/f01_hazards.encounter.json` (patches 248..312 and 856..920 at `floor_y` 580; ceiling stickers r1 on track 512..600 and r2 on track 760..816).
  - Room 6: `assets/levels/f01_late/mastery.encounter.json` (patch 320..360; ceiling sticker r1 on track 912..928).
  - The room 4 layout note in [enemy gameplay: slime and Rot](../01-design/enemy-gameplay-slime-rot.md) section 15 ("a cosmetic corrosion stain at x about 150, teach the look").
- **Current art:**
  - The pieces: `dfx_acid`, `dfx_ring_r48` and `dfx_ring_r64`, written by `scripts/author_danger_fx.py`.
  - The stand-ins: `src/hazards_art.rs` (`Tone::Acid`, `Tone::AcidLip`, `Tone::Drip`, `DRIP_PX`, `fn drip`, "spent drip (stand-in)").
  - The pure wiring: `src/danger_fx_art.rs` (`draws`, `Layer`).
- **Characters:** SLIME-01 ([spec](slime-01-spec.md), `slime_01.grid.json`) and ROT-01 ([spec](rot-01-spec.md), `rot_01.grid.json`). ROT-01 places the "wall and ceiling tells, drip stains and decals" here, and SLIME-01 places the "puddle / acid trail decals and corrosion marks on stone" here.
- **Mood only:** [atlas panel 07](../../assets/concept/floor-01/atlas-v1/07-slimes-rot-stickers.png) shows acid streaks on the walls, slime footprints and dark star-cracked Rot marks. No pixel is cropped, traced or resampled from it.
- **Tone:** [antihero tone](../01-design/antihero-tone.md) and [art direction](art-direction.md) (cold blue-violet stone, magenta magic, hot orange explosions).

**Horizon check.** This is presentation only: no move, ability, unlock or terrain-combat rule changes, so the [living horizon](../01-design/character-progression-horizon.md) needs no update.

## 1. Tone and readability rules

- **Grimy, practical and chemical.** The acid is a dirty industrial fluid that eats mortar. It is not a glowing fantasy potion: there are no sparkles, no glow halo and no bloom. The splash is a short, ugly slop around the boots.
- **Every effect is a different substance:**
  - The slime is a green gel body, the enemy.
  - The acid is a teal chemical sheet, the floor hazard (HZB-AD1).
  - The goblins are green skin.
  - Donut's magic is magenta.
  - Rot is soot-black shell with olive rot and dead red.

  No two of these share a hue at 1x.
- **Decals never lie about danger.** Corrosion and Rot marks are dry and dark: they carry no lit acid colour, no sheen and no fizz, and they never sit on the walking surface next to a live pool. A player must never mistake a stain for a pool, or a nest mark for a sticker.
- **Warnings stay on top.** The rings stay on the Tell layer above everything. The splash and the decals never cover a ring.
- **No text, logos or third-party designs.** Every pixel is drawn for this project.

## 2. Coordinates and anchors

All placements use layout coordinates as in `AcidPatchSpec` (x right, y down; `floor_y` is the walking line). The Bevy half converts them exactly as it converts the acid strip today. "Pivot row on `floor_y`" means the same rule as `dfx_acid`, whose pivot row 9 is drawn on the patch's walking line and whose surface row (row 4) is 5 px above it.

## 3. Elements

| Element (grid) | Canvas, pivot | Frames and timing (60 Hz ticks) | Anchor | Palette | Layer and z |
|---|---|---|---|---|---|
| Acid pool recolour (`dfx_acid`, existing) | 80 x 10, (0, 9), unchanged | unchanged (`acid_idle` 4 x 8 ticks, `acid_drain` 2 x 6 ticks) | unchanged | HZB-AD1 ramp, a 1:1 remap of the ten entries in their current order | unchanged (`floor_z`) |
| Ring tape (`dfx_ring_r48`, `dfx_ring_r64`, existing) | unchanged | unchanged | unchanged | + `flame_00` (kit) for the outer band row (HZB-AD2) | unchanged (`tell_z`) |
| **Acid entry splash** (`dfx_acid_splash`, new) | 32 x 24, (16, 23) | `splash` 5 frames: 2, 3, **3 (KEY)**, 4, 4 ticks; played once (16 ticks) | x = the actor's x at the event, clamped (section 4.1); pivot row on the patch `floor_y`; the surface row is row 18 | HZB-AD1 ramp, all ten entries | new `Layer::Splash` in `src/danger_fx_art.rs` (`FxLayer` in the scene): above the actors, below the rings (`Tell`) (HZB-AD10) |
| **Corrosion, floor face** (`dfx_corr_face`, new) | 32 x 10, (16, 0) | 1 static frame (+ 1 mirrored use) | row 0 is the first solid row under the pool's bottom row; the tiles cover the patch's **authored** `x0..x1` | dry subset: `acid_outline`, `acid_0_deep`, `acid_1_shade`, `fume_dark`, `fume_lit` | floor decal: over the floor solid, under the acid (`floor_z - 0.002`) |
| Corrosion, floor top (`dfx_corr_top`, new) | 32 x 6, (16, 5) | 1 static frame | room 4 only, x 150, pivot row on `floor_y` (a const table, section 4.3) | dry subset | as `dfx_corr_face` |
| Corrosion, wall trail (`dfx_corr_wall`, new) | 12 x 24, (0, 23) | 1 static frame | one per patch at `x0 + 6`; the pivot row sits on the **backwall base**, the last backwall row above the deck's back edge, at `floor_y - DECK_TOP_PX - 1` (HZB-AD17) | dry subset | backwall decal z + 0.01 (over the brick, behind everything else) |
| **Rot nest, ceiling** (`dfx_rot_nest`, new) | 32 x 12, (16, 0) | 1 static frame | row 0 on the ceiling surface's underside line, x = the track's `start` | ROT-01 entries only (HZB-AD5) | `sticker_z - 0.002` (under the shell, over the solid) |
| Rot track scuff (`dfx_rot_nest`, tag `trail`) | 16 x 3, (8, 0) | 1 static frame (alternate uses mirrored) | every 16 px from `a` to `b` along the underside, skipped within 16 px of the nest | ROT-01 entries only | as the nest |
| **Spent drip** (`dfx_rot_drip`, new; replaces the HAZ-Q38 stand-in) | 8 x 16, ceiling (4, 0), wall (0, 0) | `drip_run_c` / `drip_run_w`: 4 x 6 ticks once from `detonate_tick`; then `drip_drop_c` / `drip_drop_w`: 3 x 20 ticks, looping | the stand-in's anchor: ceiling at the dome x on the underside; wall at the face x (authored `WallLeft`, mirrored for `WallRight`) at the dome height | ROT-01 entries only | `sticker_z - 0.001`: over the nest (`sticker_z - 0.002`), under the Rot shell |

No other element is in scope. The Rot explosion's own splash belongs to the hazard-VFX work order, not here.

### 3.1 Acid entry splash: frame plan

The key (frame 2, `crown`) is frozen in `assets/source/props/floor01/hz01b_keys/acid_splash_key.grid.json`.

- **Design.** The crown is drawn for a pop toward +x: the right sheet is taller and its beads fly toward +x. The two sheets frame an empty centre 6 px wide (columns 13..18 above the rim), where the actor's boots and shins sit. The low displaced lip is rows 16..18, and its centre hole stays dark (`acid_0_deep` and `acid_outline`). Rows 19..20 hold the trough under the entry, inside the pool volume.
- **Lighting.** Inner faces (toward the hole) are shade. Outer faces are lit. The tips are sheen.

| Frame | Ticks | Content |
|---|---|---|
| 0 `dimple` | 2 | The surface dents: a dark trough 8 px wide on rows 17..19 and a low lip of rows 16..17 on both sides, 1 px higher on the +x side. Two 2 px fizz beads (`acid_fizz` over `acid_4_lit`) just above the lip |
| 1 `rise` | 3 | The key's two sheets at about half height: the right tip near row 10, the left near row 13. The lip as in the key. No detached drops yet |
| **2 `crown` (KEY)** | 3 | Frozen key: full sheets, beads breaking off the tips, five drops in the air (+x spray), one fume wisp over the hole |
| 3 `break` | 4 | The sheets collapse to stubs (the right top near row 12, the left near row 14). Every drop of the key travels on along its arc: +2..3 px outward, the high ones still rising by 1 px, the low ones falling 2 px. The wisp rises 2 px and turns `fume_dark` |
| 4 `settle` | 4 | No sheets. The lip sinks to rows 17..18 and its lit ends move 2 px outward (the ripple). Three or four drops fall back onto rows 14..16. The wisp is gone |

Every frame follows these rules:

- No isolated pixel: every drop is a 2 px chunk (a bead plus a tail).
- No colour outside the key's ten entries.
- Nothing outside the 32 x 24 canvas.
- Rows 19..23 never extend past the key's rim width.

The mirrored use (a -x pop) flips the whole frame about the pivot column.

### 3.2 Corrosion: the decal language

The floor-face sample is frozen in `hz01b_keys/corrosion_key.grid.json`. It shows:

- a broken stain band along the floor edge (rows 0..1);
- three eaten bites of **different width, depth and spacing** (`acid_outline` cores with `acid_0_deep` rims): a wide ragged bite 3 rows deep at the left, a narrow 1-row nick in the middle, a medium bite 2 rows deep on the right. The gaps between them differ (10 and 8 px) and the right quarter is almost clean;
- uneven 1 px trails that do not hang from the centre of every bite: one long kinked trail of 5 rows, one short one, two hairlines;
- one `fume_lit` crust speck on the band.

The edge must read as eaten material, not as a regular trim. No two bites share a shape, and the stamping (alternately mirrored tiles) never lines up equal bites at a fixed period. The first key's five evenly spaced V pits tiled into a sawtooth fringe and were redrawn (HZB-AD18).

`dfx_corr_top` and `dfx_corr_wall` are drawn in the same language:

- **`dfx_corr_top`** is a flat, blotchy stain on the floor-top band. Its darkest pixels sit in the middle, it has no outline at its ends, and it is never lit.
- **`dfx_corr_wall`** is two or three thin, uneven runs climbing at most 20 px up the backwall from its base, thickest at the base. It has one eaten-mortar notch and is never lit. It lies wholly on the backwall, never on the deck's top surface (HZB-AD17).

No corrosion frame uses `acid_2_mid`, `acid_3_light`, `acid_4_lit`, `acid_sheen` or `acid_fizz`. That is the dry rule.

### 3.3 Rot marks: the language

The Rot marks use ROT-01's palette entries and add no new colours:

- `outline_soot`, `shell_0_deep`, `shell_1_shade` and `shell_2_mid` for the crust;
- `rot_olive` for the gunk;
- `rib_dead` for one or two dark red flecks that echo the ribs;
- `rim_cold` only as the single wet glint on a drip bead.

No `glow_*`, `spike_tip`, `rib_red` or `flash_white` appears in any mark: the marks are spent or dormant, and they never read as a live warning.

- **Ceiling nest.** A crusted, star-cracked blotch on the underside. It is 20..26 px wide, with three or four short gunk stalactites of 2..5 px. It reads as "something lives here", not as a second sticker: it has no dome shape and no rim highlight.
- **Track scuff.** A broken 1..2 px line of `shell_0_deep` and `rot_olive` specks along the underside.
- **Spent drip, ceiling, `drip_run_c`:**
  - Frame 0: a 6 px olive splat on rows 0..1.
  - Frame 1: a strand of 4 px.
  - Frame 2: a strand of 9 px with a bead.
  - Frame 3: a strand of 13 px with a bead at its foot.
- **Spent drip, ceiling, `drip_drop_c`:** the bead swells (rows 13..14), then detaches as a 2 px chunk at row 15, then the strand hangs alone.
- **Wall variants.** These are a 2 px smear running down the face from the dome height, with the same four and three beats.

## 4. Presenter rules (presentation only, hash-neutral)

These rules apply to every new element:

- They are drawn by pure functions of the combat state, the tick, the room's authored data and the event log.
- They are never written into the simulation.
- They are never part of the state hash.
- They must not change any golden or replay.

The pattern to copy is `src/blink_cues.rs`:

- `step(tick, combat, ...)` scans only new events through `combat.dropped_events + combat.events.len()` and its own `seen_events`.
- It resets on a backward seek.
- `view(tick)` is pure.

The Bevy half pre-spawns its sprites, as `src/bin/floor01_scene/pierce_spark.rs` does.

### 4.1 Splash trigger and placement

- **Triggers:**
  - Every `CombatEvent::HeroHurt` whose `source & 0xFFFF == ACID_SOURCE_ID`; the patch is `source >> 16`.
  - Every `CombatEvent::EnvironmentHurt` with `class: HazardClass::Acid`; the patch is the event's `prop`.

  An invulnerable entry produces no event, so it produces no splash. That is accepted.
- **Goblin coverage.** `EnvironmentHurt` exists only while `fu.acid_hurt_event` is on. Without it, goblins make no splash, which is acceptable and recorded. If a lethal landing produces only an `env_defeat` defeat event that carries the acid class, it triggers the splash too. If no event identifies the acid, the gap is recorded in the PR.
- **x position.** x is the hurt actor's x at the event tick, clamped to `[left + 6, right - 6]` of the patch's live `rect()`. A patch with no live rect draws no splash.
- **Facing.** The splash is mirrored when the actor's x is below the patch centre (`(x0 + x1) / 2`). This is the sign of the sim's pop, which is + when `x >= centre`.
- **Rim crop.** Rows 16..23 of every frame are cropped to the live rect's x range, so the lip never spills onto dry floor. Rows 0..15 (the sheets and drops) are not cropped.
- **Slots.** There are 4 splash slots.
  - A new event for an actor that already has a playing splash restarts that splash.
  - When the slots are full, the oldest is replaced.
  - Two events with the same tick, patch and actor make one splash.

### 4.2 Draw order

The draw order, from back to front:

1. the backwall
2. `dfx_corr_wall`
3. the floor solid
4. `dfx_corr_face` and `dfx_corr_top`
5. the acid pool
6. the ceiling solid, then `dfx_rot_nest` and its trail
7. the spent drip (`sticker_z - 0.001`)
8. the Rot shell (`sticker_z`)
9. the actors
10. `dfx_acid_splash`: above Carl, Donut and idle goblins. Swinging goblins and globs draw above it, which is accepted
11. the rings (Tell)

The blast layer draws under the actors in the scene, so it is not a bound on the splash (HZB-AD10).

The splash covers the actor's boots inside the crown, which is intended. If the 40-sprite danger FX pool is shared, splashes are dropped before rings. Warnings are never hidden (see F2 of the danger FX spec).

### 4.3 Decal placement

Decals are static per room. They are computed once at room load by a new pure module (suggested name `src/hazard_decals.rs`) from the authored `AcidPatchSpec`s, the sticker tracks and one const table that holds room 4's entry stain at x 150. They use no data pin and no JSON change.

- **`dfx_corr_face`.** It covers the authored `x0..x1`, not the live rect, because the stain outlives the drain. It is drawn as `ceil((x1 - x0) / 32)` tiles centred on the patch. Every second tile is mirrored, and the tiles are cropped to `[x0 - 4, x1 + 4]`.
- **`dfx_corr_top`.** It sits only where the const table says. It is never within 24 px of a patch's authored ends.
- **`dfx_corr_wall`** (HZB-AD17).
  - It is one trail per patch at `x0 + 6`. Its pivot row (row 23) sits on the backwall base: `y = floor_y - DECK_TOP_PX - 1` in layout px (y down).
  - `DECK_TOP_PX` is the deck's visible top surface above the walking line. In the catacomb kit (`assets/source/terrain/floor01/floor01_catacomb_kit.json`) that is one `deck_back` row (16 px) plus the front tile's `walk_line_y` (4 px), so 20 px (ADR-0007 "deep floor"). The reviewer measured 19 to 20 px in the room 4 and room 6 captures.
  - Derive the value from the kit, never retype it per room, and pin it with a test against a captured pixel column.
  - The trail's 24 rows then lie wholly on the backwall brick, behind the deck tiles, and never on the walking surface. The trail art itself is unchanged.
- **`dfx_rot_nest`.**
  - It is drawn for ceiling tracks only.
  - Wall nests are deferred (HZB-AD6), because no authored room has a wall sticker. A wall track simply gets no nest.
  - The drip keeps both orientations.
- **The drip's age.** The drip's age is `tick - detonate_tick` while `is_spent()`. The run plays on ages 0..23, then the drop loop runs. The stand-in `Tone::Drip` piece and its label are removed when the art is present. The stand-in remains only as the fallback when the atlas is missing, as for the other danger FX.

## 5. Palette

### 5.1 HZB-AD1: the teal acid ramp (finding F5)

| Entry (dfx_acid order and key) | Old (SLIME-01) | New | CIELAB L* |
|---|---|---|---|
| `#` acid_outline (was `outline_murk`) | #121a0b | **#061418** | 6 |
| `$` acid_0_deep (was `gel_0_deep`) | #222f14 | **#0d3034** | 18 |
| `%` acid_1_shade (was `gel_1_shade`) | #3a5219 | **#124c50** | 29 |
| `&` acid_2_mid (was `gel_2_mid`) | #5d861c | **#197a7e** | 46 |
| `'` acid_3_light (was `gel_3_light`) | #8eba24 | **#2eb2a8** | 66 |
| `(` acid_4_lit (was `gel_4_lit`) | #bddb37 | **#78e2d4** | 84 |
| `)` acid_sheen (was `sheen_torch`) | #dcef7c | **#c6f8ee** | 94 |
| `*` acid_fizz (was `acid_hot`) | #f0e61e | **#eafffa** | 99 |
| `+` fume_lit (was `bone_gel_lit`) | #b4be62 | **#a4c4c0** | 77 |
| `,` fume_dark (was `bone_gel_dark`) | #7d893c | **#587a78** | 48 |

**Why teal.** The slime's body stays green, and the acid it leaves behind is a different, chemical substance: verdigris-teal, the colour of mortar and copper eaten by acid.

**Why the hue is free.** Teal is unused in the world palette:

- the goblins and slime are green;
- Donut's magic is magenta;
- the stone is blue-violet, much darker and less saturated;
- the explosions are orange.

**Contrast.** Teal is also the complement of the lit orange brick, so the pool reads near torches.

**Measurements.** CIEDE2000 between the slime ramp and the acid ramp, entry by entry from shade to sheen:

| Vision | Range |
|---|---|
| normal | 23.5 .. 30.6 |
| protan | 24.1 .. 31.9 |
| deutan | 26.5 .. 34.8 |
| tritan | **10.4** .. 22.0 |

The protan, deutan and tritan rows use the Machado 2009 matrices at severity 1.0.

- **Tritan.** Tritan is the only weak axis: a blue-yellow confusion pulls green and teal together. It stays at ΔE00 ≥ 10. On top of that, the read relies on structure: the slime is a tall mound with an outline and a skull, while the pool is a flat 6 px sheet.
- **Backgrounds.** Against the floor (#4f312e) the lit surface `acid_3_light` measures a contrast ratio of 4.4, and `acid_4_lit` 7.5.

**Geometry.** The acid geometry does not change: every `dfx_acid` row stays byte-identical, and only the palette values and entry names change. The frozen key digest is rows-only, so `tests/fixtures/f01-dfx-acid_keys.json` stays unchanged if the author script keeps the key characters in entry order. If it cannot, re-pin it explicitly and list the moved pin. The stand-ins `Tone::Acid` and `Tone::AcidLip` become `acid_2_mid` and `acid_4_lit` (with their current alphas).

This supersedes the acid half of DFX-AD1. The acid no longer borrows SLIME-01 entries: `author_danger_fx.py` gets its own `ACID_RAMP`.

### 5.2 HZB-AD2: the dark tape backing (finding F1)

- **`ring_march`.** In both frames of r48 and r64, every band pixel of the outer row (`r <= d < r + 1` in `ring_march`) becomes `flame_00` (#8a2a12), whatever its dash colour. The inner row keeps `flame_02` and `brass_05`, and the teeth and soot lines are unchanged.
- **`ring_hot`.** The outer row is `flame_00` in both flicker frames. The inner row flickers `flame_04` to `flame_05`, as the inner row does today.
- **Contrast on lit brick.** `flame_00` against `brick_09` and `brick_10` measures a contrast ratio of 3.3..4.5 (ΔE00 36..46). That is where `flame_02` measured 1.1..1.3 (ΔE00 5..10).
- **Contrast on dark brick.** On the dark brick of room 6 the inner row still carries the read.
- **Moved pins.** These are the only pixel changes to the rings. The r48 and r64 frozen-key fixtures move and must be re-pinned explicitly.

### 5.3 Atlas budget

The new teal entries and `flame_00` (already a kit entry) keep every danger FX atlas under the 63-colour rule. The Rot marks add no colour.

## 6. Source-of-truth convention (HZB-AD7)

- **Hand-drawn keys.** The keys live in `assets/source/props/floor01/hz01b_keys/*.grid.json` (`pixel-grid-v1`, one frame, layer `body`).
- **Author script.** The author script (`scripts/author_danger_fx.py`, extended) loads the keys verbatim as the frozen frames. It draws every other frame and writes the production grids `assets/source/props/floor01/dfx_acid_splash.grid.json`, `dfx_corr_face`, `dfx_corr_top`, `dfx_corr_wall`, `dfx_rot_nest` and `dfx_rot_drip`. Each production grid gets an `.ase`, a `*.pixelart.json` config and an `assets/manifests/f01-dfx-*.json` manifest, as the existing danger FX do.
- **Frozen-key fixtures.** `tests/fixtures/f01-dfx-acid-splash_keys.json` pins frame 2, and `tests/fixtures/f01-dfx-corr-face_keys.json` pins frame 0. Each pin is byte-identical to its key grid (rows-only digest).

## 7. Acceptance targets

1. **F5.** At 1x in the room 4 and room 6 window captures, the slime's base outline separates from its pool along the whole contact line, and the pool, the slime and a goblin are three different colours. The ΔE00 table of section 5.1 holds (normal, protan and deutan ≥ 20 per entry, tritan ≥ 10).
2. **Acid unchanged in shape.** The `dfx_acid` rows are byte-identical, and the frozen key is unchanged or explicitly re-pinned.
3. **F1.** The march and hot rings read at 1x on the room 4 torch wall and on the room 6 dark brick, in the window. The r48 and r64 fixtures are re-pinned and listed.
4. **Splash, frames.**
   - Frame 2 is byte-identical to the key.
   - There are 5 frames over 16 ticks, played once.
   - No frame has an isolated pixel or uses a colour outside the key's ten.
5. **Splash, behaviour.**
   - It is visible at Carl's boots at 1x in both rooms, in both facings.
   - It is never drawn over a drained patch.
   - No more than 4 play at once.
   - A goblin landing makes one when `fu.acid_hurt_event` is on.
6. **Corrosion.**
   - The dry rule holds (no lit acid entries).
   - No stain lies on the walking surface within 24 px of a patch's ends.
   - The `dfx_corr_face` frame is byte-identical to its key.
   - Every authored patch in rooms 4 and 6 has its face stain and wall trail, and room 4 has its entry stain at x 150.
   - Every opaque pixel of a wall trail lies on the backwall, above the deck's back edge. A test derives `DECK_TOP_PX` from the catacomb kit and checks it against a captured pixel column in both rooms.
   - The face edge shows no regular period: no two bites in one tile share a width and depth.
7. **Rot.**
   - There is a nest at every ceiling track's start, with a scuff along `a..b`.
   - The spent drip art replaces the stand-in, the label is gone, and the run starts on `detonate_tick`.
   - No `glow_*`, `rib_red` or `spike_tip` appears in any mark.
8. **Hash-neutral.**
   - Every state hash, golden and replay line of the simulation is unchanged.
   - A test runs a hazard fixture with the presenters and without them and compares the state hashes.
   - The capture hash of the window equals the headless capture hash.
   - The render replay lines of rooms 4 and 6 are refreshed and listed as moved pins, because the pixels change.
9. **Validators.** The pixel-art validators pass on every new grid. The `max_isolated_per_frame` setting is 0 everywhere. Orphan budgets are set from the drawn frames and recorded. Spray drops and stain trails are two-colour 2 px chunks, so they count as orphans by design: the splash key measures 26 orphans and the corrosion key 15, against the default limit of 9. Set `max_orphans_per_frame` to the largest frame's measured count plus 2 in each element's config, at most 32. Existing prop configs go up to 40.
10. **Evidence.**
    - Contact sheets at 6x or 8x.
    - The splash as a GIF at 1x and 4x.
    - Window stills at 640 x 360 of rooms 4 and 6: the slime on its pool, a splash at Carl's boots, a ring on lit brick, a nest and a spent drip.
    - A CVD strip.
    - A `SHA256SUMS` file.

## 8. Execution list (done; evidence in the evidence README)

1. Recolour `dfx_acid` (HZB-AD1) and the stand-in tones.
2. Apply the ring backing (HZB-AD2) and re-pin the ring fixtures.
3. Draw the remaining splash frames, the two corrosion variants, the nest and its trail, and the drip from this spec and the keys.
4. Export the grids, manifests, `.ase` files and checksums.
5. Write the validators, the unit tests, the presenter modules and the Bevy halves.
6. Record the captures and refresh the replay lines.
7. Collect the evidence and update the docs.

## 9. Readability gate (design step)

The gate was run with `scripts/hz01b_key_gate.py`. The background stills are:

- **Room 4:** `docs/evidence/f01-danger-fx/scene/acid_full_640x360.png` (committed).
- **Room 6:** a 640 x 360 capture of the room 6 open scenario at tick 700. It is not committed; its SHA-256 is in the [evidence README](../evidence/area-hazards-01b/keys/README.md).

The composites approximate the scene light with a flat multiply: 0.9 on the dim floor, 1.0 near the torch. The images are in [docs/evidence/area-hazards-01b/keys](../evidence/area-hazards-01b/keys/README.md).

**Verdict: PASS**, with one note to carry into execution.

- **F5: pass.** At 1x the teal pool reads as a separate substance under the green slime in both rooms. The old green pool merges into its slime. The protan and deutan views separate olive from grey-blue. The tritan view separates them by saturation and structure.
- **Splash: pass.** At 1x it reads as liquid thrown up around Carl's boots on the dark floor of room 4 and in front of the dark door of room 6.
  - **Note for execution:** at 1x the crown is small next to a 52 px hero. Frames 1 and 3 must keep the full sheet height range. Do not shrink the sheets.
- **Corrosion: pass.** Under the pool it reads as old stains and trails, never as liquid.
- **F1: pass.** With the dark backing, the dashes and the hot band keep an edge on the lit brick around the torch, and the bright inner row still carries the read on dark brick.

## 10. Adopted defaults

| ID | Decision | Reason |
|---|---|---|
| HZB-AD1 | The acid becomes the teal ramp of section 5.1, a 1:1 remap of the ten `dfx_acid` entries. The slime keeps its green. This supersedes the acid half of DFX-AD1 | Finding F5: the slime merged into its pool at 1x. Teal is a free hue and the complement of the lit brick. ΔE00 ≥ 20 against the slime ramp for normal, protan and deutan vision, ≥ 10 for tritan |
| HZB-AD2 | The ring's outer band row becomes `flame_00` in `ring_march` (both dash colours) and `ring_hot`. Nothing else changes | Finding F1: `flame_02` disappeared on lit brick (contrast 1.1). A dark backing reads on lit and dark brick without new colours or geometry |
| HZB-AD3 | The splash presents existing hurt events only: no sim entry tick, 4 slots, facing by the pop sign, the rim cropped to the live rect | Presentation only. It stays exactly in sync with the damage the player takes |
| HZB-AD4 | Corrosion uses the dry palette subset. Stains follow the authored patch extents, plus one const entry stain in room 4 at x 150 | Decals never look like a live pool, and the stain outlives the drain. The room 4 layout note asks for a "teach the look" stain |
| HZB-AD5 | The Rot marks use ROT-01 entries only, with no glow, live red or spike colours | Spent or dormant marks must never read as a warning |
| HZB-AD6 | Ceiling nests only; wall nests are deferred. Both drip orientations are drawn | No authored room has a wall sticker, but the drip stand-in already supports walls |
| HZB-AD7 | Hand-drawn keys live in `hz01b_keys/*.grid.json`, are loaded verbatim by the author script and are pinned by frozen-key fixtures | The keys stay the single source for the frozen frames |
| HZB-AD8 | Corrosion and nest marks are permanent from room load. They are dressing, not state | No sim coupling; deterministic from authored data |

### 10.1 Execution defaults

| ID | Decision | Reason |
|---|---|---|
| HZB-AD9 | The Rot nest and its track scuff are two grids, `dfx_rot_nest` (32 x 12) and `dfx_rot_trail` (16 x 3), not two tags of one grid | An atlas is one row of equal cells, so one canvas size per grid. The tags are `nest` and `trail` as in section 3 |
| HZB-AD10 (approved in review) | The splash z is a hair above Carl and Donut (`actor_z + 0.105`) and under the rings. It is above the actors and below the rings, not "below Blast". Swinging goblins (+0.15) and globs (+0.2) draw above it, which is accepted | In the scene the blast layer lies 0.455 **under** the actors, so a z next to it would hide the splash behind Carl's boots. The blast wave still draws under the actors as before |
| HZB-AD11 | The splash and the decals have their own sprite pools (8 for the splash: a crown and a cropped lip per slot; one per static decal and one per sticker for the decals), not the 40-sprite danger FX pool | A splash can never push a warning ring out of the shared pool, and the rim crop of rows 16..23 needs two sprites per splash |
| HZB-AD12 | An acid `EnvironmentKill` (the lethal landing) triggers the splash like `EnvironmentHurt` | The defeat event carries the acid class and the patch, so a goblin killed by the pool splashes whatever `fu.acid_hurt_event` says. The only remaining gap is an invulnerable entry, which produces no event |
| HZB-AD13 | The scuff skips a position when it is **16 px or less** from the nest (the nest is 26 px wide, so a scuff at exactly 16 px would sit under its edge). Room 6's 16 px track has a nest and no scuff | Keeps the scuffs clear of the nest. The room 4 tracks get 5 and 2 scuffs |
| HZB-AD14 | The const entry stain is keyed by the combat fixture's name (`f01_hazards`), on the walking line of the room's first acid patch | The decals need no room data and no new JSON; room 4's layout note asks for the stain at x 150 |
| HZB-AD15 | In frames 1 and 3 the sheet tips are one row taller than the table's "near" rows (right tip row 9 and left row 12 in frame 1; right row 11 and left row 13 in frame 3), and the drops of frame 3 clamp at the canvas edge | The readability gate's note: the crown must not shrink next to a 52 px hero. The table says "near" and these stay within one row |
| HZB-AD16 | The wall drip sheet is authored with the ceiling pivot (4, 0) in the grid; the manifest records the wall pivot (0, 0) on its `_w` frames and the presenter applies it | A grid has one pivot; the wall pivot is per tag in the manifest |
| HZB-AD17 | The wall trail is anchored on the backwall base (`floor_y - DECK_TOP_PX - 1`, with `DECK_TOP_PX` = 20 from the catacomb kit), not on the walking line | Review finding F1: on the walking line the 24 px trail sat entirely on the 20 px deck top behind the live pool and never reached the backwall. That broke the section 1 rule that decals never sit on the walking surface next to a live pool |
| HZB-AD18 | The corrosion key is redrawn with three bites of different width, depth and spacing and offset trails. The stamping rule is unchanged (alternately mirrored tiles) | Review finding F3: the first key's five evenly spaced V pits tiled into a sawtooth or icicle fringe. With three unequal bites per tile, a mirrored pair never repeats one shape at a fixed period. The redrawn key has 13 orphan pixels, within the current budget of 17 |

## 11. Open questions

None block execution. Owner game-feel notes to collect after merge:

- whether the splash should also fire on a drained patch's last tick (it does not now);
- whether the slime should leave footprint decals (not in scope; it needs a trail state).
- **Small splash crown (review F5).** It reads at 1x on the left pool entry but gets lost in clutter on the right entry and in room 6. It is kept as drawn (the sheets are already at the full height range of HZB-AD15). Whether to enlarge it is the owner's call in play.
- **Rot nest contrast in room 6 (review F6).** The nest is dormant by design and uses only ROT-01's dark entries, so it is low-contrast on room 6's dark brick. It is readable on the room 4 slab and pillar. Whether a dormant nest should read more strongly is the owner's call; any lift must stay out of the warning colours (HZB-AD5).
