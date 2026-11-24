# AREA-GOBLINWORKSHOP-01: Goblin Workshop prop kit (Boiler encounter), design spec

**Status:** design spec, 13 frozen keys, execution frames (in-betweens, state loops, tells), export, manifests, validators and the Bevy wiring are done; the two gate-approved key edits are applied and re-frozen (section 8). Independent top-tier review passed (PR 256, no design defect); its non-blocking follow-ups are applied (section 9: weight silhouette re-frozen under the review's approval, rig run, plume, shadow tell, scorch ember fade), their review is pending; owner review pending. Evidence: [docs/evidence/f01-gwkit](../evidence/f01-gwkit/README.md).

**Route:** difficulty score **4** (new design; boiler native height 208 px; rig with moving parts; venting, telegraph and blast states), class Hard, route **B + silhouette gate** ([routing by difficulty](../07-agents/pixel-art-pipeline.md#routing-by-difficulty)). No separate draft existed, so the design step drew every key that defines a silhouette or a state read and froze them; execution draws in-betweens, loops, tells and VFX only after the gate passes. Work order row: [floor-01-area-kits-plan.md](../07-agents/floor-01-area-kits-plan.md) (`AREA-GOBLINWORKSHOP-01`: boiler, tools, pipes, gears, parked-vehicle props; no stationary factory boss).

**Authorities:** encounter roles and timings [floor-01-goblin-boiler-encounter.md](../01-design/floor-01-goblin-boiler-encounter.md) (P1–P4); geometry [f01_boiler.encounter.json](../../assets/levels/floor01/f01_boiler.encounter.json); the present stand-in [src/env_prop_art.rs](../../src/env_prop_art.rs). Mood concept [06-goblin-workshop.png](../../assets/concept/floor-01/atlas-v1/06-goblin-workshop.png) for **mood only** (riveted copper vertical boiler with dome, gauge and glowing firebox; red hazard drums; a hook on a chain from an overhead beam): no pixel is cropped, traced or resampled from it. Pipeline: [art direction](art-direction.md), [asset pipeline](asset-pipeline.md), [pixel-art pipeline](../07-agents/pixel-art-pipeline.md), [antihero tone](../01-design/antihero-tone.md).

**Sources:** `assets/source/props/floor01/gw_*.grid.json` (+ `.ase` built from them, one `*.pixelart.json` config and one `assets/manifests/f01-gw-*.json` manifest per grid), authored by [scripts/author_gwkit_keys.py](../../scripts/author_gwkit_keys.py) (the key functions) and [scripts/author_gwkit_frames.py](../../scripts/author_gwkit_frames.py) (every frame, the configs and the manifests; deterministic, `tests/test_art_gwkit.py` re-derives the committed files byte for byte). The tells are one grid per canvas: `gw_tell_sputter`, `gw_tell_plume`, `gw_tell_shadow`, `gw_tell_ring`.

## 1. Canon and tone

- A goblin boiler room is **industrial service gone wrong**: theatrical, deadly, held together by bodges. Every prop shows one improvised fix: a brick wired under the safety lever, a crooked patch plate, a timber post wedged with a painted pin, a welded patch on the weight.
- Nothing heroic or ornate: no gilding, banners or skulls. Grime is soot and condensation, not gore.
- No text, logos, numbers or readable writing. Hazard paint is abstract yellow/black stripes.
- IP-neutral data: ids are `gw_*` and the encounter's `valve`, `weight`, `drum`; no character names.

## 2. Readability rules (the kit's core decision)

1. **Yellow/black marks a trap part.** The `brass_05` / `iron_00` stripe accent appears only on trap parts. The three at strike height are the hit targets: the valve handwheel, the brace wedge pin and the drum band. The two elevated ones mark where a trap delivers danger and are not targets: the weight chevron band (the weight is dropped by hitting the brace) and the steam-outlet collar (the steam lane).
2. **Interactive steel is cold.** Valve, brace hardware, weight, rig and hoops use the `stone_cold_00–04` ramp so they separate from the warm brick wall at 1x.
3. **The boiler is background.** Its copper sits one step darker (`copper_00–04` body, `copper_05` only on the single specular streak and rim catches, `copper_06` on two or three glints). Its focal point is the firebox glow (`flame_00–04`), the only hot light in the room besides the drum telegraph.
4. Light comes from the upper left; every prop has a 1 px `iron_00` silhouette outline (soot marks on the boiler are kept off the outline).
5. Foreground props (valve, brace, weight, drum) are brighter and higher contrast than the boiler they stand in front of.

## 3. Scale and placement (layout px, y down, floor at y 580, 60 Hz ticks)

Carl is 52 px (6 ft), so 8.7 px per foot. The boiler shell is 12 ft tall to the rim, 20 ft to the stack cap: a room-sized machine, not a kettle.

| Grid | Canvas w x h | Pivot | Layout anchor (canvas top-left) | Matches (must not change) |
| --- | --- | --- | --- | --- |
| `gw_boiler` | 192 x 208 | (96, 207) | (496, 372) | bottom 176 rows = landmark {496, 404, 192, 176}; top 32 rows are dome and stack (presentation only, replaces the stand-in 12 px dome) |
| `gw_valve` | 32 x 48 | (16, 47) | (464, 532) | bottom 40 rows = valve target {464, 540, 32, 40}; top 8 rows: handwheel |
| `gw_brace` | 32 x 40 | (16, 39) | (744, 540) | weight target (brace) {744, 540, 32, 40}; chain eye at local x 15–16 |
| `gw_weight` | 64 x 48 | (32, 47) | (mass.x, mass.y − 16) | bottom 32 rows = `mass_rect()` (hanging {784, 420, 64, 32}); top 16 rows: shackle and fall streaks |
| `gw_remnant` | 64 x 32 | (32, 31) | (784, 548) | remnant {784, 548, 64, 32} |
| `gw_drum` | 36 x 36 | (18, 35) | (846, 544) | body 32 x 32 at local (2, 4) = drum {848, 548, 32, 32}; 2 px margins for bulge and hiss |
| `gw_scorch` | 48 x 8 | (24, 7) | (844, 572) | stand-in scorch {t.x − 4, t.bottom − 3, t.w + 8, 3}, raised to 8 rows |
| `gw_rig` | 112 x 24 | (0, 0) | (744, 352) | sheave chain columns at local x 15–16 (brace, layout 759–760) and 71–72 (weight, layout 815–816) |
| `gw_chain` | 4 x 8 | (0, 0) | tiled at x 758 and x 814 | vertical repeat tile, link centre at local x 1.5 |

Placement facts the art relies on: the valve side pipe (local y 18–25) meets the boiler inlet pipe (boiler local y 178–185), both at layout y 550–557. The boiler nozzle mouth is boiler local x 191 = layout 687, the left edge of the steam lane {688, 484, 80, 96}; its bore is layout y 515–528. The weight hangs on the right sheave; the brace tethers the left one; the return chain sags between the sheaves under the beam.

## 4. Props, keys and states

Keys marked **F** are frozen: execution must not change a pixel of them (grid frame index given). Frames marked **E** are drawn in execution from the description, inside the same canvas, after the silhouette gate.

### P4 `gw_boiler` (background landmark)

Silhouette: wide riveted copper cylinder on an iron plinth, shallow dome cap, a small steam dome with a lever and ball on the left, a stack on the right, an outlet pipe with a flared nozzle on the right edge and a short inlet stub on the left. Details: brass gauge (upper left), cool sight glass (upper right), two iron hoop bands, three rivet seams, patch plate, barred arched firebox door.

| Frame | Kind | Content | Timing |
| --- | --- | --- | --- |
| 0 `boiler_idle` | **F** | intact key | idle loop frame 1 |
| 1–3 | E | idle loop: firebox brightens one ramp step (f1), gauge needle +1 px (f2), lever ball bobs 1 px and firebox dims back (f3); silhouette identical to frame 0 except the ball | 4 frames x 8 ticks |
| `boiler_strain` | E | 2 frames while the valve is Warning: needle in the red arc, ball lifted 2 px, 3–4 px hiss puff (`gal_10`, `stone_10`) at the steam dome, firebox at full `flame_04`; frame 2 shifts the body 1 px right (rattle) | 2 x 4 ticks (= `FLASH_TICKS`) |
| `boiler_damaged` | **F** | damaged key: torn dome flap, lever snapped and hanging (ball and brick gone), stack kinked, gauge smashed, upper hoop sprung out left, soot streaks, torn patch corner, scorched nozzle collar, firebox dead (cold iron grate, grey ash bed, a few last embers), thin tapered soot smears | damaged loop frame 1 |
| damaged 2 | E | ember flicker in the firebox, one soot drip 1 px longer; silhouette identical | 2 x 10 ticks |

Shown damaged when the stand-in shows "broken" today (valve Venting or Spent, or mass Landed). The boiler never moves the sim.

### P1 `gw_valve` (hittable, steam trigger)

Silhouette: squat cold-steel valve on a bolted floor plate: riser, round cast body, bonnet, brass stem and a yellow handwheel seen slightly from above; a copper side pipe runs right into the boiler inlet.

| Frame | Kind | ValveState | Content | Timing |
| --- | --- | --- | --- | --- |
| 0 `valve_intact` | **F** | Intact | intact key | static |
| `valve_warning` | E | Warning | 2 frames: handwheel spokes rotated 45° (frame 2), 2–3 px packing puff at the bonnet, body 1 px right on frame 2 | 2 x 4 ticks |
| `valve_venting` | E | Venting | spent body plus a short white jet spitting from the torn side pipe and the neck; 3-frame loop | 3 x 4 ticks |
| 1 `valve_spent` | **F** | Spent | bonnet and wheel blown off, bent stem stub, side pipe split into curled petals, cracked body, drip | E wisp loop 3 x 10 ticks: the key, then two wisp frames rising from the torn neck |

Broken art persists (encounter doc P1).

### P2 `gw_brace`, `gw_weight`, `gw_remnant`, `gw_rig`, `gw_chain` (falling-weight trap)

Silhouettes: a timber post with a diagonal kicker strut, an iron floor shoe, a top cleat with the chain eye, a yellow wedge pin through the post (the hit target). The weight is a stack of three cast-iron slabs (60 px wide, vertical sides, dark notches between the slabs) on two tie rods whose nuts stand on top, under a square lifting lug, with bolts, a welded patch and a yellow/black chevron band on the middle slab (section 9: it replaced a flared trapezoid that read as a lamp shade). The rig is a ceiling-hung I-beam with two sheave blocks and a sagging return chain. The remnant is the weight sunk and tilted into heaved floor slabs with rubble and a coiled chain on top.

| Frame | Kind | MassState | Content | Timing |
| --- | --- | --- | --- | --- |
| brace 0 `brace_supported` | **F** | Supported | supported key | static |
| brace 1 `brace_weakened` | **F** | Weakened | upper post sheared 1 px left above a 1 px dark split, pale splinters out of both sides, wedge pin pulled 3 px further out with its outer end dropped 1 px, strut cracked | **required** E creak loop: 2 x 6 ticks (wedge jiggles 1 px, 2 dust grains); frame 1 is the key |
| brace 2 `brace_stump` | **F** | Released, Falling, Landed | snapped stump with splintered tips, strut fallen flat, wedge on the floor | static |
| weight 0 `weight_hang` | **F** | Supported, Weakened, Released | hanging key; Released keeps the code sway of ±1 px every `SWAY_TICKS` (6) | static |
| `weight_fall` | E | Falling | same body, three 1 px speed-streak columns in rows 0–15, chain gone, shackle pin straight up | 1 frame |
| remnant 0 `remnant` | **F** | Landed | remnant key; the walkable top is rows 0–3 across local x 6–57 (collision is the rect, unchanged) | static |
| rig 0 `rig` | **F** | Supported, Weakened | rig key | static |
| `rig_run` | E | Released onward | return chain gone from under the beam; its 10–12 px loose end leaves the weight sheave's rim on the return (left) side and hangs down the sheave block's left cheek, swung out (frame 1), then straight (frame 2) | 2 x 6 ticks, then hold frame 2 |
| chain 0 `chain` | **F** | — | 4 x 8 tile | — |

Chain drawing (code, presentation only): brace chain from layout y 376 down to the brace eye (y 540) while the brace is whole (Supported, Weakened); weight chain from y 376 down to the weight eye (mass top) while Supported, Weakened, Released; none while Falling or Landed.

### P3 `gw_drum` and `gw_scorch` (hittable, blast)

Silhouette: an upright red fuel drum: steel lid with a brass bung, two rolling hoops, a yellow/black diagonal stripe band, a dent and a fuel stain. Scorch: a black oval splat with a blackened hoop ring, two twisted shards and an ember.

| Frame | Kind | DrumState | Content | Timing |
| --- | --- | --- | --- | --- |
| drum 0 `drum_armed` | **F** | Armed | armed key | static |
| `drum_telegraph` | E | Telegraph | seams along both hoops glow `flame_03`; frame 2 bulges the lid 1 px into the top margin and the body 1 px into each side margin; 2–3 px hiss at the bung | 2 x 4 ticks |
| scorch 0 `scorch` | **F** | Detonated | scorch key; drum hidden | E ember fade: the key, a dimmer frame, the cold frame; 3 x 10 ticks from the drum charge's `detonate_tick`, then the cold frame holds |

The explosion itself stays `blast_art`; this kit adds no blast frames.

## 5. Tells and VFX (execution, new grid `gw_tells`)

All tells are presentation only, drawn on the Tell layer, and keep the stand-in rects as their footprint:

| Tag | Canvas | Rect | Content | Timing |
| --- | --- | --- | --- | --- |
| `steam_sputter` | 16 x 16 | at the nozzle mouth (layout 688, 512) | 2–5 px puffs leaving the nozzle (telegraph) | 3 x 4 ticks |
| `steam_plume` | 80 x 96 | steam lane {688, 484, 80, 96} | one continuous jet from the nozzle bore spreading right and slightly up, built of overlapping billows on a continuous core (about 55 % coverage), billows travelling 4 px a frame; a ground fog at actor height; light from the upper left: `gal_10` shadow, `stone_10` body, near-white `flame_05` core, `gal_08` only on the fog's edge; Bevy applies the stand-in alpha (0x90) | 4 x 4 ticks loop |
| `fall_shadow_tell` | 64 x 8 | {784, 572, 64, 8} | `iron_00` ellipse with a solid 1 px rim and a dithered inside, hazard ticks at both ends; inside dither and ticks swap phase at `FLASH_TICKS`; Bevy alpha 0x70 | 2 x 4 ticks |
| `fall_shadow` | 64 x 8 | same | solid ellipse (Bevy alpha 0x70) | static |
| `drum_blast_tell` | 96 x 8 | floor of {816, 500, 96, 80} | dithered red/yellow floor ring blinking with the drum telegraph | 2 x 4 ticks |
| landing dust | — | — | reuse the existing `shore_dust` burst at the remnant top; no new art | — |

Palette budget: the keys use 58 kit colours (the plume core adds `flame_05`: 59); the whole kit, tells included, must stay at 63 or fewer kit colours, all named and valued exactly as in the [Floor 01 kit palette](../../assets/source/terrain/floor01/floor01_catacomb_kit.json).

## 6. Wiring contract (presentation only)

- Replace every code-drawn piece in `src/env_prop_art.rs` with atlas frames from the generated kit atlas(es), following the atlas binding pattern of `src/boss_art.rs` (pure draw list, placeholder fallback when the atlas is missing).
- Remove the stand-in labels (`src/env_prop_art/labels.rs` and the `PropTag` sprites) once the art draws.
- The encounter simulation, its rects, the state hash and every pinned capture line and pilot hash stay byte-identical (`docs/evidence/f01-boiler/capture_lines.txt`, `pilot_hashes.txt`).
- Layer order is unchanged: rig, chain and boiler on Back; valve, brace, weight, remnant, drum and scorch on Prop; tells on Tell.

## 7. Acceptance targets

1. Every frozen key is byte-identical to the frame listed here; every E frame keeps its prop's canvas, pivot and bottom row (bottom-row drift 0 within a prop, except `gw_weight` streak rows above the body and drum bulge margins).
2. No isolated pixels in any frame; palette within section 5; alpha is binary in the sources.
3. At 1x in the lit room (640 x 360) a viewer can name, without labels: the boiler, the valve, the brace, the hanging weight, the drum, and which three things can be hit (yellow accents at strike height).
4. The boiler never reads brighter than the valve, brace, weight or drum standing in front of it; the firebox is the brightest boiler pixel.
5. Damaged and spent states read as broken from silhouette alone (boiler dome flap and hanging lever; valve missing its wheel; brace stump).
6. The steam plume covers the lane rect and reads as dangerous at 1x without hiding an actor completely (alpha as specified).
7. No text, logos or third-party designs; mood concept not traced.

## 8. Silhouette gate record and open for execution

The silhouette gate found no design defect and prescribed key fixes; it is the design authority for them. Applied and re-frozen in the design sources:

- `gw_weight` 0: the chevron band is now drawn (an argument-order bug had dropped it, so the weight read as a lamp shade).
- `gw_boiler` 1: dead firebox (no `flame_03/04`, 26 `flame_00/01` and 2 `flame_02` ember pixels) and soot as thin tapered dithered smears in `iron_01/02` instead of hard bars.
- `gw_brace` 1: stronger weakened read (sheared upper post, visible split, splinters, pin pulled out); creak loop made required.
- Rule 1 and acceptance 3 reworded (trap parts vs hit targets).

**Execution record** (the pre-approved edits; the touched keys are re-frozen in `tests/fixtures/area-gw-*_keys.json`, the other 11 keys are byte-identical to the gate head):

- `gw_valve` 1 `valve_spent`: **applied.** The split-pipe petals are 5 px thick bands with `copper_02/03` inside the outline and `copper_04/05` catches, so they read as torn copper (grid frame 6).
- `gw_remnant` 0: **applied.** Each visible side slants out 1 px per 3 rows, at most 4 px (rows 1 to 13), over the heaved slabs; rows 0 to 3 over x 6 to 57 (the walkable top) are unchanged.
- `gw_drum` 0 and `drum_telegraph`: **checked, no change.** At 1x the telegraph (two `flame_03` / `flame_02` seam rows across both hoops, the lid and body bulge, the hiss) is far brighter and wider than the armed key's single `flame_02` highlight column, so the armed highlight stays.
- `gw_boiler` 0: **not applied** (an owner call, not a default change).

Frame order in the grids follows the tags (a tag is a contiguous run), so a key's grid index is not its spec order: boiler keys at frames 0 and 6, valve 0 and 6, brace 0, 1 and 3. Deviations from the tables above, all execution choices inside the frozen canvases (listed for the independent review):

- `rig_run`: the 24 row canvas leaves 2 rows below the weight sheave, so the loose chain end (10 to 12 px of path) hangs out of the right side of the sheave block and swings (frame 1), then settles beside the lead-out (frame 2). The simulation keeps no "ticks since release", so the presenter plays the two frames on the tick while the weight is Released and rests on frame 2 once it falls or lands.
- Ticks-driven loops: every loop plays on the world tick (`tick / ticks per frame`), as the stand-in's flash and sway did; no state timer is added to the simulation.
- `steam_sputter` has 3 frames (puff grows from 2 to 14 px) and the sputter, plume and shadow are drawn on the Tell layer; the steam warning area and edge and the drum's blast area outline stay as code-drawn footprints (they are taller than the art), the weight footprint rectangles are replaced by the shadow sprite.
- Optional frames skipped: `scorch` ember fade (the simulation has no detonation timer to drive it) and the landing dust reuse (a one shot needs the landing tick). The ember fade was added later (section 9).

- The parked-vehicle, tools, pipe and gear dressing named in the work-order row are not needed by the Boiler encounter and are out of this kit's first pass; they extend `gw_*` later without touching these keys.

## 9. Review follow-ups (after the PR 256 review)

The independent review of PR 256 passed with no design defect; these are its non-blocking follow-ups. Per the [fix-loop rule](../07-agents/pixel-art-pipeline.md#routing-by-difficulty), the review is the design authority for the key change it asked for.

- **`gw_weight` 0 re-frozen (follow-up 3).** The flared trapezoid with a ring on top read partly as a lamp shade, worse with a backdrop torch under it. The review asked for a bulkier or taller mass: the weight is now a stack of three cast slabs, 60 px wide with vertical sides and notches, on two tie rods (their nuts stand on top), under a square lifting lug; the shackle pin, the chain column (local x 30–33) and the chevron band are kept. The body fills rows 7–31 of the mass rect (it filled rows 8–31 before, tapering from 64 to 48 px). `weight_fall` follows from the same body. The fixture `tests/fixtures/area-gw-weight_keys.json` is re-frozen; the other 12 keys are unchanged. The remnant key already shows a straight-sided block, so it now matches better. The torch and the encounter x stay as they are (a level-composition call).
- **`rig_run` (follow-up 1).** The review's alternative, a rig canvas 12 rows taller, would change the canvas of the frozen `rig` key (acceptance 1: every frame keeps its prop's canvas), so the frames are redrawn inside the frozen 112 x 24 canvas instead: the return chain fed the weight sheave from the left, so its loose end now leaves the pulley rim on that side and hangs down the sheave block's left cheek (an 11 px path to the canvas bottom), clear of the weight chain below the lead-out. Frame 1 swings it out to the left, frame 2 hangs it straight in face-on and edge-on links like the `gw_chain` tile; the old draft's 2 px links were turned into solid outline by the silhouette pass and are gone.
- **`steam_plume` (follow-up 2).** Redrawn as one continuous jet (section 5), about 55 % coverage in every frame instead of about 28 %, with a paler near-white core (`flame_05`, one new kit colour, 59 of 63).
- **`fall_shadow_tell` (follow-up 4).** A solid 1 px rim around the dithered inside, and the shadow sprite's alpha raised from 0x58 to 0x70 (adopted default GWP-AD3; the code-drawn stand-in tone is unchanged).
- **`valve_spent` wisp (follow-up 6).** The art has a 3 x 10 tick loop (the key and two wisp frames); the spec table now says so.
- **`scorch` ember fade (follow-up 7).** Added: it needs no simulation change, because the drum's `BlastCharge` keeps its own `detonate_tick` (the blast wave of the [danger FX](floor01-danger-fx-spec.md) reads the same tick). The presenter reads it through the scene's charges; with no known tick the cold frame holds. The landing dust stays skipped: the falling mass keeps no landing tick, so it would need a simulation change.
- **Not changed:** the boiler brightness and dominance (follow-up 5, an owner call), the small sputter and the remnant's square hazard blocks (follow-up 7, the footprint carries the warning).
