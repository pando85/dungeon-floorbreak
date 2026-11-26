# AREA-FASTFOOD-01: the fast-food refuge kit of Floor 01 room 1, design spec

**Status:** design spec plus eight keys, frozen for the silhouette gate. The keys are:
- the staff door, two frames: `ff_staff_door_closed`, `ff_staff_door_open`;
- the Light on Your Feet pickup, three frames: `ff_reward_box_full_a`, `_full_b`, `_empty`;
- three language samples: `ff_pass_ledge`, `ff_wainscot_a`, `ff_floor_checker_a`.

The gate sheets are in [`docs/evidence/area-fastfood-01/gate/`](../evidence/area-fastfood-01/README.md). The rest of the kit, the room dressing, the presenter and the Bevy wiring are not executed yet (section 13). Nothing here is owner-approved. Launch target: `floor01_scene --scene f01_late --f01-room rest_refuge`. Room 1 of `--f01-run` launches the same room.

**Route:** difficulty score 2. The kit is a new environment family. It has two state-keyed hero keys: a door whose state gates the exit lane, and a pickup with a collected state. The route is therefore **B + silhouette gate** ([routing by difficulty](../07-agents/pixel-art-pipeline.md#routing-by-difficulty)), as for the Broken Ring and the scorched corral. The art audit (row V5, pickup row P2) proposed C plus a gate. The state-keyed keys are the reason for the upgrade. The design step stops at this spec and the keys. Work order: AREA-FASTFOOD-01 (pipeline order batch 4).

**Authorities:**
- **Design intent:** [quiet set pieces, section G](../01-design/floor-01-quiet-setpieces-and-transitions.md):
  - beats F-A0..F-A4;
  - G.7, the inputs of this kit;
  - REST-AD5/6: the shelf opens a 16 x 228 staff door, latched;
  - REST-AD16: the box is non-collidable decor over the grant column;
  - REST-AD17: no door notice, so the door's own art carries the beat;
  - REST-Q5: the rolling ball gag.
- **Geometry:** `assets/levels/floor01/f01_rest_refuge.room.json` and its `.encounter.json` sidecar (pinned hashes, read-only here), `src/f01_rest.rs`, and the composer `src/camera_lab/layout.rs` via `f01_boiler::compose_kit`.
- **Mood only:** [atlas panel 04](../../assets/concept/floor-01/atlas-v1/04-fast-food-refuge.png) and [atlas row 04](floor-01-art-atlas-v1.md) ("1990s family restaurant dropped into an implausible underground context; keep it recognizable and unexpectedly clean"). No pixel is cropped, traced or resampled from the panel.
- **Environment language:** the [ENV-01 kit](floor-01-catacomb-kit.md) contract and palette `floor01-catacomb-v2`.
- **Precedents:**
  - the [scorched corral kit](area-heat-01-spec.md): a `room_dressing` pass through `f01_boiler::compose_kit_hooked`;
  - the [Broken Ring kit](area-rings-01-spec.md): a state-keyed prop drawn by a presenter, its moving frames crops of the key;
  - the [gym kit](area-gym-01-spec.md): piece lights inside the light budget.
- **Pipeline and tone:** [art direction](art-direction.md), [asset pipeline](asset-pipeline.md), [antihero tone](../01-design/antihero-tone.md).

## 1. Canon and tone constraints

- **What the room is:** an ordinary 1990s family restaurant dropped into the catacomb. It is spotless, abandoned and lit by cold strips. The joke is the dissonance: after the scorched corral, nothing here is burnt, and nothing wants to kill you. The room is **not** a theme park:
  - no mascots;
  - no faces on props;
  - no prices, menu boards with words, or tray liners with print.
- **No text, glyphs, logos, brands or recognisable chain trade dress.** Shapes that would normally carry text (the backlit panel over the counter, the bin flap, the staff door) carry blank fields, abstract food-like blobs or nothing. Colour pairs that point at a real chain are out:
  - no red-and-yellow scheme;
  - no golden arches shape;
  - no crown, star or clown motif.
  The room's identity is **blue and white tile with yellow accents**.
- **Engine ids are IP-neutral:**
  - pieces: `ff_*`;
  - atlases: `floor01_ff_*`;
  - contract: `floor01_fastfood_kit.json`;
  - code: `src/fastfood_kit.rs`.

  `AREA-REFUGE-01` is Mordecai's guild room, a different room, and its `refuge_*` ids are not reused.
- **The pickup is a found object, not loot fanfare.** It is a small gilded box on the floor that Donut walks through. There is no pedestal, no beam of light, no rotating icon.

## 2. Scale and coordinates

Carl is 52 px tall, about 1.8 m. All coordinates are **room layout pixels** (top-left origin, y down), as composed for `rest_refuge` by `f01_rest::launch`.

| Line | y | Note |
| --- | --- | --- |
| Walking line `W` | 580 | room `floor` 580 |
| Floor back line `FB` | 560 | props stand here (their bottom row is 559) |
| Deck surface | 560..579 | floor decals only on its back half, 561..572 |
| Base course | 544..559 | |
| Slab (ceiling) | solid 336..351, full width | `ceiling_px` 228; jump ceiling y 352. The launch pushes the slab solid; no kit piece draws it today |
| Actor head and torso band | 524..560 | wall art stays mid-value here (rule 10.3) |
| Deck stand lines | play deck 516, slide top 500, staff shelf 452 | the platform tops of the sim |
| Kit plank tiles over them | grid rows 512, 496, 448 (16 px, opaque, lit row at tile row 1) | collidable tiles of the collision layer: **never removed**, only covered (rule 9.2) |
| 640 x 360 view | top 292 | the HUD plates cover about y 292..352 at both corners |

| `rest_refuge` (1600 wide) | Rect (x, y, w, h) or x | Note |
| --- | --- | --- |
| Protected spawn | 16, 484, 208, 96 | Carl spawns at x 112, Donut at x 80 |
| Bay A (composer) | `pier_l` 80..151, opening 152..303, `pier_r` 304..383, y 304..575 | the catacomb entry arch; `door_timber` at (216, 488) stands in it |
| Bay B (composer) | `pier_l` 832..903, opening 904..1055, `pier_r` 1056..1135 | the restaurant's window onto raw rock (F-A1) |
| Plain brick at floor level | x 384..831, 1136..1439, 1456..1600, plus 264..303 once bricked up | wall dressing ground |
| Play deck (platform 0) | 464, 516, 96, 16 | gate `carl`, 64 px |
| Slide top (platform 1) | 560, 500, 128, 16 | gate `carl`, 80 px |
| Checkpoint | 1104, 484, 48, 96 | |
| Reward box (decor) and grant column | x 1200..1240 | grant `f01_safe_room_reward_found` once, any y |
| Staff shelf (platform 2) | 1296, 452, 96, 16 | gate `donut_boosted`, 128 px; the lock's surface |
| Lock `staff_door` | solid 1440, 352, 16, 228 | latched open on the first tick Donut stands on the shelf |
| Exit | 1504, 484, 64, 96 | goal x 1528; `guild_portal_closed` at (1480, 444) |
| Torches (composer) | (48, 432), (96, 448), (336, 448), (848, 176), (848, 448), (1088, 448), (1536, 432) | the dressing keeps 48, 96 and 1536 (section 8) |

## 3. Room layout (zones, left to right)

| Zone (beat) | x | Dressing |
| --- | --- | --- |
| Threshold, hot side (F-A0) | 0..263 | the composer's catacomb as is: bay A, the two threshold torches, `door_timber`. Only a few loose cream floor tiles reach out from the door (`ff_floor_spill`) |
| The seam (F-A0) | 264..383 | the tile floor starts at the timber door with a broken edge (`ff_floor_checker_edge` at x 256, under the door's right edge). The bay A opening behind (264..303) is bricked up with the composer's brick (`backwall_fill`) and carries the first wainscot. `ff_tile_seam` at x 376 marks the stone pier's right face where the clean tile wall begins |
| Dining room and play area (F-A1) | 384..831 | wainscot and drop ceiling from here to the staff door. The play structure dresses platforms 0 and 1 (deck skins, slide chute, ball pit, one static stray ball). The trash bin is at x 400 and the counter run at x 704..831 |
| The window (F-A1) | 832..1135 | bay B becomes a restaurant window: frame, mullions and a sill over the opening. The parallax behind it is replaced by raw rock (`ff_rock_fill`). A booth set stands under it on the back line |
| The box (F-A2) | 1136..1295 | plain tile wall, nothing tall: the gold box at x 1204 is the only warm object in a cool field |
| Shelf and staff door (F-A3) | 1296..1459 | the pass-through ledge skin on the shelf, the dark service hatch above it, a tiled under-panel below, and the staff door key in the lock |
| Exit vestibule (F-A4) | 1460..1600 | the composer's catacomb with the closed portal and the 1536 torch: back to ominous masonry |

## 4. Mapping onto the room solids (presentation only)

Every piece is presentation only:
- pieces never produce solids (only collidable tiles of the collision layer do);
- the dressing never adds, removes or moves a tile;
- room, encounter and table hashes stay unchanged.

The door's solid belongs to the lock (`f01_rest::RestRun::door`). The door key only draws it. The box never collides (REST-AD16).

## 5. Piece list

All pieces live on `floor01-catacomb-v2` (section 7). Sizes are native px. "At" is the top-left in layout px. **Key** means drawn in the design step and frozen. Everything else is executed per this table.

| Id | Layer | Size | At | Notes |
| --- | --- | --- | --- | --- |
| `ff_staff_door_closed`, `_open` | **presenter** (not in the dressing) | 24 x 228 | (1436, 352) | **Key.** Section 6 |
| `ff_reward_box_full_a`, `_full_b`, `_empty` | **presenter** | 32 x 24 | (1204, 556) | **Key.** Section 7 |
| `ff_reward_sparkle_a..c` | presenter | 16 x 16 | centred on (1220, 562) | grant-tick burst, 3 frames x 4 ticks, `brass` 6 / `flame` 4..5 only, at most 10 px each |
| `ff_pass_ledge` | structure | 96 x 20 | (1296, 448) | **Key.** Covers the shelf's plank tile exactly; stand line on row 1 |
| `ff_wainscot_a`, `_b` | backwall_decal | 32 x 32 | y 528, x = 32 n over 288..1439 | **Key** `_a`. `_b` is the same field with a different chip pattern; never two `_b` in a row |
| `ff_wall_tile_a` | backwall_decal | 32 x 32 | y 368..496 in 32 px rows, x = 32 n over the plain brick of zones 384..831 and 1136..1439 (not over the hatch) | pale field: `gal` 8/9 tiles with `gal` 6 grout and sparse `stone_cold` 4 highlights. **No pale white wall field**: whites are reserved (rule 10.2) |
| `ff_floor_checker_a` | backwall_decal | 32 x 12 | y 561, x = 32 n over 288..1439 | **Key.** |
| `ff_floor_checker_edge` | backwall_decal | 32 x 12 | (256, 561) | broken left edge: missing tiles and two loose tiles |
| `ff_floor_spill` | backwall_decal | 48 x 12 | (168, 561) | three loose cream tiles on the hot side, at most 18 px of tile |
| `ff_tile_seam` | structure | 8 x 192 | (376, 368) | grout bead and torn tile edge against the stone pier |
| `ff_ceiling_tile` | structure | 32 x 16 | y 336, x = 32 n over 288..1439 | acoustic drop ceiling: `gal` 7..9 panel faces, `gal` 4 T-bar. Its bottom row is the visible slab underside |
| `ff_soffit_stone` | structure | 32 x 16 | y 336 elsewhere | the slab in the catacomb's own stone (`stone` 3..5) |
| `ff_light_strip` | prop | 48 x 6 | y 352, x 400, 592, 784, 976, 1168, 1360 | fluorescent tube under a `gal` housing; tube `copper` 8 on 2 rows only. One light per strip (section 8) |
| `ff_counter_a` | prop | 128 x 36 | (704, 524) | order counter: `gal` stainless top at y 524..527 (**darker than every stand line**, rule 10.1), blue tiled front, a yellow kick strip. A blank register block with no screen glyphs |
| `ff_counter_canopy` | backwall_decal | 128 x 40 | (704, 400) | backlit blank panel over the counter: three abstract food-like blobs in `brass`/`copper` on a `gal` 9 field, no words |
| `ff_deck_skin` | structure | 96 x 16 | (464, 512) | covers platform 0's plank tile. Row 1 is `flame` 5; the yellow plastic face is `brass` 4..5 with `gal` posts below |
| `ff_slide_top_skin` | structure | 128 x 16 | (560, 496) | same language as the deck skin, platform 1 |
| `ff_play_posts` | structure | 224 x 52 | (464, 528) | two posts under each skin, a net panel between, all dimmer than the skins' faces (`gal` 5..7, `brass` 2..3) |
| `ff_slide_chute` | backwall_decal | 64 x 64 | (688, 496) | the slide curving down behind the counter end, `brass` 3..4 only, no `flame` 5 run (rule 10.1) |
| `ff_ball_pit` | prop | 96 x 12 | (560, 548) | the pit's rim plus a heap of balls under the slide top: `brass`, `gal` 8 and `brick` 6, single px balls in 2 x 2 clusters, rim at most 12 rows |
| `ff_stray_ball` | prop | 6 x 6 | (1012, 554) | one static yellow ball in the dining room (REST-Q5 option, section 15) |
| `ff_bin` | prop | 20 x 32 | (400, 528) | stainless bin with a swing flap, no text |
| `ff_window_frame` | structure | 160 x 160 | (900, 384) | frame, two mullions and a sill over bay B's opening. The rock shows through the panes |
| `ff_rock_fill` | parallax replacement | 152 x 192 | over the bay B opening | raw rock face, `stone` 2..5 and `stone_cold`, damp seams in `lichen`. Replaces the gallery strip in this opening only |
| `ff_booth_a` | prop | 112 x 40 | (912, 520) | two benches and a table: blue vinyl `gal` 6..8, a `brass` table edge, `stone_cold` chrome legs. The table top is not brighter than `gal` 9 |
| `ff_service_hatch` | backwall_decal | 80 x 48 | (1304, 400) | dark kitchen opening over the ledge: `iron` 0..2 interior, one dim `copper` 5 heat-lamp glow row. **Never brighter than the ledge's stand line** |
| `ff_under_panel` | backwall_decal | 96 x 60 | (1296, 468) | tiled panel under the ledge (the counter's back face) |
| `ff_bell` | prop | 8 x 6 | (1380, 442) | the service bell at the ledge's right end. Frames `idle`, `ding_a`, `ding_b` play once on the door-open tick (section 6) |

## 6. The staff door: states

The lock's solid is 16 x 228 at (1440, 352). The key canvas is 24 x 228 at (1436, 352):
- canvas columns 4..19 are the solid;
- columns 0..3 and 20..23 are the jamb rails, present in **both** states (the frame stays, so the player always sees where the door was);
- row 0 is y 352 (the slab underside) and row 227 is y 579 (the walking line's last row).

| State | When | Drawing |
| --- | --- | --- |
| **Closed** (`ff_staff_door_closed`) | the lock is closed: every launch and fixture restart, until the open event | a full-height brushed stainless panel filling the solid exactly (3648 of 3648 px). It has vertical grain only: two section joints at rows 76 and 152, never horizontal rungs, so it never reads as a ladder. Features: the porthole at row 176 (Carl's head band), the push plate at rows 192..203, the kick plate from row 208 (the floor back line) to the walking line, and a **red** status lamp on the right jamb at rows 112..116 |
| **Rising** (presentation only) | the 12 ticks after the open event | crops of the closed key: the panel rises into the slab, by `ceil` per tick from the table below. The jamb rails and the lamp (now green) stay. **No drawn in-between frame** |
| **Open** (`ff_staff_door_open`) | the lock is open (latched for the attempt; kept on a checkpoint respawn) | rails only, the panel's 4 px kick-plate lip at rows 0..3 under the slab, a floor sill track at rows 226..227 and a **green** lamp. The lane rows 4..225 between the rails are fully transparent (0 of 3552 px) |

Rising table: the raise in px on ticks 1..12 after the open event (tick 0 is the event tick, drawn as tick 1) is 19, 37, 59, 85, 112, 139, 165, 188, 205, 217, 222, 224. The panel's bottom clears Donut on tick 2 (37 px) and Carl's 52 px with margin on tick 3 (59 px). The table eases out: it moves fast, then settles into the slab.

The sim opens the lane on the event tick itself, so the art is never behind the solid (rule 9.1). A fixture restart snaps the art back to closed on the restart tick. A checkpoint respawn with the lock open draws the open frame at once (no replay of the rise).

The bell rings on the event tick. That is the only cue of the open beat besides the door itself (REST-AD17: no door notice). `ff_bell` plays `ding_a`, `ding_b`, `ding_a` for 4 ticks each, then idles. Optional audio is out of scope here.

## 7. The pickup (Light on Your Feet): states

The canvas is 32 x 24 at (1204, 556). Row 23 is the walking line's last row (y 579). Its centre column (canvas x 16, world x 1220) is the centre of the grant column 1200..1240. The pickup is drawn in the actor plane but **under** the actors (prop layer z, like the kit's props), so Donut walks through it.

| Frame | When | Drawing |
| --- | --- | --- |
| `full_a` / `full_b` | the boost is not held | a gilded box: a `brass` 3..5 body, two raised gold straps, a dark `iron` band with two rivets, a keyhole clasp, and a lid whose top edge is `flame` 4 (the box's brightest run, luma 229, below the stand lines' 250). The frames alternate on a **presentation** timer, 24 ticks each: a 4-arm `flame` 5 glint on the lid's left corner (`full_a`), then a small one on the right corner with a strap highlight (`full_b`) |
| `empty` | the boost is held | the same box opened and spent. The lid stands up behind it, showing its shadowed underside. The dark empty inside shows over the lit front rim, the clasp hangs open, and the gold is one step duller. No glint (mean luma 85 against 137 full) |
| `ff_reward_sparkle_a..c` | the grant tick only | a 12-tick burst over the box, then `empty` |

**State source:** `empty` if and only if Donut holds Light on Your Feet in the running attempt. The presenter reads the same state the `LIGHT FEET` notice and Donut's boosted jump read: the grant receipt `f01_safe_room_reward_found`, or the loadout flag on a launch that starts with it. Consequences:
- a launch with the boost already held (the `All` loadout) shows `empty` from the first tick;
- a launch without it shows `full`;
- a fixture restart that re-arms the grant shows `full` again.

The art never decides the grant.

## 8. Lighting (presentation only)

- The dressing removes the dining-room torches and their pools:
  - torches at (336, 448), (848, 176), (848, 448) and (1088, 448) via `remove_at`;
  - `torch_pool_a_m` (232, 364), `torch_pool_a` (744, 92), `torch_pool_a_m` (744, 364) and `torch_pool_a` (984, 364);
  - `grime_a` (600, 376): the restaurant is unexpectedly clean.
- The threshold torches (48, 96) and the vestibule torch (1536) stay. The hot→cold contrast therefore happens in the light as well as in the art.
- Each `ff_light_strip` carries one light entry. Colour `#d8e4ff`, radius 150, intensity 0.24, group `scene`, glow radius 14, additive 0.08. There is no flicker (FF-AD7).
- Net light count change: minus 8 (four torches with a flame and a floor pool each), plus 6. The room stays well inside `MAX_LIGHTS` 64. A test proves it at the worst view (section 12).
- Light colours are not palette pixels; the pixels stay on the palette.
- **Cool light is not the gym's signature (FF-AD8).** Atlas row 09 reserves "fluorescent/accent light" for the gym. The refuge's strips are soft, steady and cool. They light a clean room. They are not the gym's high-contrast accents.
- The box has **no** emitter (FF-AD6).

## 9. Collision and state

1. **The lane is never drawn closed when it is open, nor open when it is closed.** On the open event tick the presenter switches from the closed frame to rising frame 1. The lane between the rails is clear above both heroes within 3 ticks (Donut within 2). Before the event, the closed frame covers the solid's every pixel.
2. **Collidable tiles are covered, never removed.** The deck, slide-top and shelf skins sit on the tile's grid row (platform y minus 4). Their rows 0..15 are opaque across the tile's width. Row 1 (the tile's own lit row) is the stand line. The skins add no pixel above the tile, so no false walkable appears above a real one.
3. **Nothing else in the lane is tall and bright.** The counter's top (y 524), the booth table and the bin are darker than every stand line. They stand on the back line and are not platforms.

## 10. Readability rules

1. **Stand lines are the brightest runs.** The three walkable tops are drawn in `flame` 5 (luma 250): play deck row 513, slide top row 497, shelf row 449. Within 64 px of each, no other horizontal run of 8 px or more is brighter than luma 201 (`stone` 10). Examples:
   - the checker's pale squares are `stone` 10 (201);
   - the wainscot's top trim is `gal` 10 (135);
   - the light strips' tubes sit 96 px above the shelf and are 2 rows.
2. **Whites are reserved.** The palette has no neutral white. Warm whites (`flame` 5, `copper` 8) appear only:
   - on stand lines;
   - on the strip tubes;
   - in the box's glint and sparkle.
   The clean "white" of the restaurant is carried by pale cool `gal` 8..10 fields and the cream checker (`stone` 9..10).
3. **Actor band:** wall art in y 524..560 stays mid-value (`gal` 5..9), so Carl's dark jacket and Donut's orange both separate from it. The wainscot sample measures `gal` 3..10 with most of its field at 7..8.
4. **Door state is unambiguous at 1x.** Closed is a solid steel column, the only tall cool-bright vertical in the right third. Open is two thin rails with nothing between them.
5. **The box is the only warm object between x 1136 and 1295.** Nothing gold, orange or yellow stands within 48 px of it.

## 11. The keys

`python3 scripts/author_ff01.py` writes the three grids and their `.ase` sources. `python3 scripts/author_ff01.py gate DIR` writes the gate sheets. Every pixel comes from rules: ramps, a Bayer threshold and a hash noise. The SHA-256 pins of the eight keys' pixel lists are in `gate/measurements.txt`. The execution keeps them byte-identical. A key change is a design change.

- `assets/source/props/floor01/ff_staff_door.grid.json`: one 24 x 228 canvas, frames closed and open.
- `assets/source/props/floor01/ff_reward_box.grid.json`: one 32 x 24 canvas, frames `full_a`, `full_b` and `empty`.
- `assets/source/props/floor01/ff_samples.grid.json`: `ff_pass_ledge` (96 x 20), `ff_wainscot_a` (32 x 32) and `ff_floor_checker_a` (32 x 12), packed in a row with 8 px gaps.

The rising frames are crops of the closed key (`door_raised(d)` in the authoring script), not separate keys.

## 12. Acceptance targets

| Id | Target | How it is checked |
| --- | --- | --- |
| FF-A1 | Solids, spawns, exit, views, walking line and lock rect are identical with and without the dressing | `fastfood_kit` unit test on the composed layout (the gym/heat pattern) |
| FF-A2 | Room, encounter and TABLE hashes, the state hash and every golden unchanged | existing pins; the window `COMBAT_STATE` hash equals the headless hash at every captured tick |
| FF-A3 | The door presenter shows closed before the event, rising for 12 ticks, then open. A fixture restart shows closed. A checkpoint respawn with the lock open shows open | headless frame-selection test over the `rest_refuge_solo_donut` fixture's ticks (event tick 370) |
| FF-A4 | The box presenter shows full before the grant, sparkle on the grant tick, then empty. The `All` loadout shows empty from tick 0 | headless frame-selection test (grant tick 326) |
| FF-A5 | Closed key covers 3648 of 3648 solid px. Open key has 0 opaque px in lane rows 4..225 | `measurements.txt`, repeated by the art unittest |
| FF-A6 | Each stand line is `flame` 5 on its tile's row 1. No rival run brighter than luma 201 within 64 px | art unittest on the composed layout (pixel model) |
| FF-A7 | Light count of the worst 640 px view stays at or under `MAX_LIGHTS` | test as `the_gym_lights_stay_inside_the_light_budget` |
| FF-A8 | Palette: every piece on `floor01-catacomb-v2`, at most 63 colours per atlas, 1-bit alpha, at least one transparent px per piece | `validate_assets.py`, art unittests |
| FF-A9 | No text, glyph, logo or chain trade dress | review |
| FF-A10 | The window still at 640 x 360 shows the door state, the box state and the three stand lines legibly at 1x | owner review of the stills |

## 13. Execution list

1. Pieces of section 5 in `scripts/author_ff01.py` (new functions; keys untouched). Then the atlases, the pixel-grid and `.ase` sources, and the exports through `scripts/export_assets.sh`.
2. Contract `assets/source/terrain/floor01/floor01_fastfood_kit.json`. It extends the opening kit, holds the atlases and pieces, and has a `room_dressing.rest_refuge` block: `remove_at`, `backwall_fill`, `place` and `parallax` for the bay B opening only. It also holds a `state_groups` entry for the door and the box (presenter-drawn, like the heat partition).
3. `src/fastfood_kit.rs`: dressing load and apply through `f01_boiler::compose_kit_hooked`, and the pure presenter frame selection (door, box, bell). Tests FF-A1, A3, A4 and A7.
4. Bevy: `f01_rest::launch` composes with the dressing. The binary replaces the greybox decor rectangles and the door rectangle (`spawn_rest_greybox`, `sync_rest_door`) with the presenter sprites. `f01_rest::decor` stays as data.
5. Evidence at `docs/evidence/area-fastfood-01/` with the window hash equal to the headless hash, docs and the art unittest `tests/test_art_ff01.py`.

## 14. Files

| Path | Content |
| --- | --- |
| `docs/02-art/area-fastfood-01-spec.md` | this spec |
| `scripts/author_ff01.py` | the authoring record (keys, gate, measurements) |
| `assets/source/props/floor01/ff_staff_door.grid.json`, `.ase` | key: the staff door |
| `assets/source/props/floor01/ff_reward_box.grid.json`, `.ase` | key: the pickup |
| `assets/source/props/floor01/ff_samples.grid.json`, `.ase` | keys: the ledge, the wainscot and the checker |
| `docs/evidence/area-fastfood-01/` | README, gate sheets, measurements, `SHA256SUMS` |

## 15. Adopted defaults and open questions

| Id | Adopted default | Reason |
| --- | --- | --- |
| FF-AD1 | Route B + silhouette gate (score 2), up from the audit's C | state-keyed door and pickup, as the ring shutter and the heat partition |
| FF-AD2 | The door is a full-height stainless **sliding panel that rises into the slab**. No header or drum hangs in the lane; the mechanism is implied by the slab | the solid runs floor to slab (REST-AD6). Anything drawn below y 352 outside the solid would read as an obstacle |
| FF-AD3 | Jamb rails stay in both states, the lamp turns red to green, and the rise is 12 ticks of key crops | the open beat has no notice (REST-AD17): the frame, the lamp and the motion carry it |
| FF-AD4 | The pickup is a small gilded box on the floor that opens and stays open when spent | REST-AD16 (decor over the grant column); a found object fits the antihero tone better than a floating icon |
| FF-AD5 | Box state reads Donut's held boost, not a separate flag | one source of truth with the `LIGHT FEET` notice and the jump |
| FF-AD6 | No emitter on the box | the rule-10.5 contrast already makes it the focal point; a glow would compete with the shelf's stand line in the same view |
| FF-AD7 | Six steady cool strip lights replace the four dining-room torches; the threshold and vestibule torches stay | the hot→cold beat (F-A0, F-A4) in the light itself; no flicker keeps the room calm |
| FF-AD8 | Cool strips in the refuge do not breach atlas row 09's "fluorescent/accent light only here" (the gym) | the refuge's strips are soft and steady, not the gym's high-contrast accents; flagged as FF-Q2 |
| FF-AD9 | No palette extension; whites are warm and reserved (rule 10.2); "white" tile is pale `gal` and cream `stone` | GYM-AD3, HEAT-AD4; the palette has no neutral grey or white ramp |
| FF-AD10 | Skins cover collidable plank tiles on the tile's grid row (16 opaque rows, stand line on row 1) | the tiles cannot be removed; the kit's planks already draw their lit row at tile row 1 (Donut's feet end on y 451 on the shelf) |
| FF-AD11 | The counter is placed at x 704..831 (behind no platform), not across the greybox `counter` rect (320..768) | the greybox rect overlaps the play decks; a prop drawn over a deck skin would hide its stand line |
| FF-AD12 | The hot→cold seam is at the timber door (x 264) with loose tiles on the hot side; bay A's stone pier stays | F-A0 ("scorched masonry shifts to white tile") inside the composer's real arch |
| FF-AD13 | One static stray ball, no rolling gag in this work order | REST-Q5 stays a review item; a static ball is free and readable |
| FF-AD14 | The service bell rings once on the open tick (presentation only) | a diegetic cue for REST-AD17's no-notice beat |

| Id | Open question | Owner of the answer |
| --- | --- | --- |
| FF-Q1 | Should a neutral white/grey ramp be added to the floor palette in a palette PR, for a brighter "unexpectedly clean" restaurant? | owner art review |
| FF-Q2 | Is cool steady strip light in the refuge acceptable next to atlas row 09's gym-only accent rule? | owner art review |
| FF-Q3 | Should the box gain a faint glow, if it gets lost at 1x in play? | owner game-feel review |
| FF-Q4 | REST-Q5: animate the stray ball rolling in (a bounded, non-collidable loop)? | owner game-feel review |
| FF-Q5 | Should Donut's paw-switch frames (ANIM-INTERACT-01) play on the shelf moment? | owner, once wired |

**Horizon fit:** the kit changes no move, ability or gate. It dresses the existing `hero_on_surface` beat and the Light on Your Feet grant of the [progression horizon](../01-design/character-progression-horizon.md) and still fits it.
