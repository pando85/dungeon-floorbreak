# AREA-FASTFOOD-01: the fast-food refuge kit of Floor 01 room 1, design spec

**Status:** executed kit (prototype art, independent review pending, not owner-approved). The design spec and the eight keys were frozen at the silhouette gate, and the gate's notes G1..G8 are applied below (the execution record is section 16). The keys are:
- the staff door, two frames: `ff_staff_door_closed`, `ff_staff_door_open`;
- the Light on Your Feet pickup, three frames: `ff_reward_box_full_a`, `_full_b`, `_empty`;
- three language samples: `ff_pass_ledge`, `ff_wainscot_a`, `ff_floor_checker_a`.

The gate sheets are in [`docs/evidence/area-fastfood-01/gate/`](../evidence/area-fastfood-01/README.md). The rest of the kit, the room dressing, the presenter and the Bevy wiring are executed (section 13, record in section 16). Nothing here is owner-approved. Launch target: `floor01_scene --scene f01_late --f01-room rest_refuge`. Room 1 of `--f01-run` launches the same room.

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
| The seam (F-A0) | 264..383 | the tile floor starts at the timber door with a broken edge (`ff_floor_checker_edge` at x 256, under the door's right edge). The bay A opening behind (264..303), and the gap inside its right pier (x 304..319), are bricked up with the composer's brick (`backwall_fill`, 264..319, FF-AD29) and carries the first wainscot. `ff_tile_seam` at x 376 marks the stone pier's right face where the clean tile wall begins |
| Dining room and play area (F-A1) | 384..831 | wainscot and drop ceiling from here to the staff door. The play structure dresses platforms 0 and 1 (deck skins, slide chute, ball pit, one static stray ball). The trash bin is at x 400 and the counter run at x 704..831 |
| The window (F-A1) | 832..1135 | bay B becomes a restaurant window: frame, mullions and a sill over the opening. The parallax behind it is replaced by raw rock (`ff_rock_fill`). A booth set stands under it on the back line |
| The box (F-A2) | 1136..1295 | plain tile wall, nothing tall: the gold box at x 1204 is the only warm object in a cool field |
| Shelf and staff door (F-A3) | 1296..1459 | the pass-through ledge skin on the shelf, the dark service hatch above it, a tiled under-panel below, and the staff door key in the lock |
| Exit vestibule (F-A4) | 1460..1600 | the composer's catacomb with the closed portal and the 1536 torch: back to ominous masonry |

## 4. Mapping onto the room solids (presentation only)

Every piece is presentation only:
- pieces never produce solids (only collidable tiles of the collision layer do);
- the dressing never adds, removes or moves a **collision** tile (`backwall_fill` only bricks up bay A's opening with the composer's own non-collidable backwall bricks, as the Broken Ring kit does);
- room, encounter and table hashes stay unchanged.

The door's solid belongs to the lock (`f01_rest::RestRun::door`). The door key only draws it. The box never collides (REST-AD16).

## 5. Piece list

All pieces live on `floor01-catacomb-v2` (section 7). Sizes are native px. "At" is the top-left in layout px. **Key** means drawn in the design step and frozen. Everything else is executed per this table.

| Id | Layer | Size | At | Notes |
| --- | --- | --- | --- | --- |
| `ff_staff_door_closed`, `_open`, `_rise_01.._12` | **presenter** (not in the dressing) | 24 x 228 | (1436, 352) | **Key** (closed, open). Section 6. The atlas holds 14 pre-baked frames (gate note G5, FF-AD15) |
| `ff_reward_box_full_a`, `_full_b`, `_empty` | **presenter** | 32 x 24 | (1204, 556) | **Key.** Section 7 |
| `ff_reward_sparkle_a..c` | presenter | 16 x 16 | centred on (1220, 562) | grant-tick burst, 3 frames x 4 ticks, `brass` 6 / `flame` 4..5 only, at most 10 px each |
| `ff_pass_ledge` | structure | 96 x 20 | (1296, 448) | **Key.** Covers the shelf's plank tile exactly; stand line on row 1 |
| `ff_wainscot_a`, `_b` | backwall_decal | 32 x 32 | y 528, x = 32 n over 288..1439 | **Key** `_a`. `_b` is the same field with a different chip pattern. Gate note G8: `_b` at every tile index n with n % 3 == 2, counted from x 288, every other tile `_a` (never two `_b` in a row) |
| `ff_wall_tile_a` | backwall_decal | 32 x 32 | y 368..496 in 32 px rows, x = 32 n over the plain brick of zones 384..831 and 1136..1439 (review F4, FF-AD20: the full grid, also under the hatch, the canopy and the under-panel, which are placed after the tiles and draw over them; the cell at x 1120 lies behind bay B's right pier and shows x 1136..1151) | pale field: `gal` 8/9 tiles with `gal` 6 grout and sparse `stone_cold` 4 highlights. **No pale white wall field**: whites are reserved (rule 10.2) |
| `ff_floor_checker_a` | structure (FF-AD26) | 32 x 12 | y 561, x = 32 n over 288..1439 | **Key.** |
| `ff_floor_checker_edge` | structure (FF-AD26) | 32 x 12 | (256, 561) | broken left edge: missing tiles and two loose tiles |
| `ff_floor_spill` | structure (FF-AD26) | 48 x 12 | (168, 561) | three loose cream tiles on the hot side, at most 18 px of tile |
| `ff_tile_seam` | structure | 8 x 192 | (376, 368) | grout bead and torn tile edge against the stone pier |
| `ff_ceiling_tile` | structure | 32 x 16 | y 336, x = 32 n over 288..1439 | acoustic drop ceiling: `gal` 7..9 panel faces, `gal` 4 T-bar. Its bottom row is the visible slab underside |
| `ff_soffit_stone` | structure | 32 x 16 | y 336 elsewhere | the slab in the catacomb's own stone (`stone` 3..5) |
| `ff_light_strip` | prop | 48 x 6 | y 352, x 400, 592, 784, 976, 1168, 1360 | fluorescent tube under a `gal` housing; tube `copper` 8 on 2 rows only. One light per strip (section 8; gate note G8: the values stay, no floor-projection entry) |
| `ff_counter_a` | prop | 128 x 36 | (704, 524) | order counter: `gal` stainless top at y 524..527 (**darker than every stand line**, rule 10.1), blue tiled front, a yellow kick strip. A blank register block with no screen glyphs |
| `ff_counter_canopy` | backwall_decal | 128 x 40 | (704, 400) | backlit blank panel over the counter: three abstract food-like blobs in `brass`/`copper` on a `gal` 9 field, no words |
| `ff_deck_skin` | structure | 96 x 16 | (464, 512) | covers platform 0's plank tile. Rows (gate note G8, as `ff_pass_ledge`): 0 `brass` 3; 1 `flame` 5 (the stand line); 2..3 `brass` 4; 4..11 the face, `brass` 5/4 by Bayer, at most `brass` 5; 12 `brass` 3; 13..14 `brass` 2; 15 `stone_cold` 0. End caps on columns 0 and w-1 are `brass` 3 (rows 2..14, FF-AD18). Nothing above row 0. Opaque over the whole tile |
| `ff_slide_top_skin` | structure | 128 x 16 | (560, 496) | same rows as the deck skin, platform 1 |
| `ff_play_posts` | structure | 224 x 68 | (464, 512) | two posts under each skin, a net panel between, all dimmer than the skins' faces (`gal` 5..7, `brass` 2..3). Review F3, FF-AD28: rows 16..67 are the first execution's 224 x 52 block (it started at y 528); the slide side's posts and net continue straight up to y 512 so the slide-top skin (y 496..511) stands on its posts; the columns under the deck skin are transparent there. Listed after the floor decals, so the posts' feet draw over the checker |
| `ff_slide_chute` | backwall_decal | 64 x 64 | (688, 496) | the slide curving down behind the counter end, `brass` 3..4 only, no `flame` 5 run (rule 10.1) |
| `ff_ball_pit` | prop | 96 x 12 | (560, 548) | the pit's rim plus a heap of balls under the slide top: `brass`, `gal` 8 and `brick` 6, single px balls in 2 x 2 clusters, rim at most 12 rows |
| `ff_stray_ball` | prop | 6 x 6 | (1012, 554) | one static yellow ball in the dining room (REST-Q5 option, section 15) |
| `ff_bin` | prop | 20 x 32 | (400, 528) | stainless bin with a swing flap, no text |
| `ff_window_frame` | structure | 160 x 160 | (900, 384) | frame, two mullions and a sill over bay B's opening. The rock shows through the panes |
| `ff_rock_fill` | backwall_decal (was "parallax replacement", FF-AD16) | 168 x 192 | (904, 352) | raw rock face, `stone` 2..5 and `stone_cold`, damp seams in `lichen`. Covers the gallery in bay B's opening only, behind the window and the arch's stone. The first 152 columns are the first execution's; 16 columns are appended so it also fills the gap inside the right pier (x 1056..1071, FF-AD29) |
| `ff_booth_a` | prop | 112 x 40 | (912, 520) | two benches and a table: blue vinyl `gal` 6..8, a `brass` table edge, `stone_cold` chrome legs. The table top is not brighter than `gal` 9 |
| `ff_service_hatch` | backwall_decal | 80 x 48 | (1304, 400) | dark kitchen opening over the ledge: `iron` 0..2 interior, one dim `copper` 5 heat-lamp glow row. **Never brighter than the ledge's stand line** |
| `ff_under_panel` | backwall_decal | 96 x 60 | (1296, 468) | tiled panel under the ledge (the counter's back face) |
| `ff_bell_idle`, `_ding_a`, `_ding_b` | presenter | 8 x 6 | (1380, 442) | the service bell at the ledge's right end, its bottom row on y 447 (gate note G8). `gal` 6..10 dome, `brass` 4 base, a 1 px `gal` 10 glint, no `flame` and no `copper`, no 8 px run brighter than 201. `ding_a` moves the plunger 1 px down; `ding_b` is `idle` plus two 1 px `gal` 10 ticks beside the dome on row 1. Played once on the door-open tick (section 6) |

## 6. The staff door: states

The lock's solid is 16 x 228 at (1440, 352). The key canvas is 24 x 228 at (1436, 352):
- canvas columns 4..19 are the solid;
- columns 0..3 and 20..23 are the jamb rails, present in **both** states (the frame stays, so the player always sees where the door was);
- row 0 is y 352 (the slab underside) and row 227 is y 579 (the walking line's last row).

| State | When | Drawing |
| --- | --- | --- |
| **Closed** (`ff_staff_door_closed`) | the lock is closed: every launch and fixture restart, until the open event | a full-height brushed stainless panel filling the solid exactly (3648 of 3648 px). It has vertical grain only: two section joints at rows 76 and 152, never horizontal rungs, so it never reads as a ladder. Features: the porthole at row 176 (Carl's head band), the push plate at rows 192..203, the kick plate from row 208 (the floor back line) to the walking line, and a **red** status lamp on the right jamb at rows 112..116 |
| **Rising** (presentation only) | the 12 ticks after the open event | crops of the closed key, **pre-baked** in the door atlas as `ff_staff_door_rise_01..12` (`door_raised(RISE[k-1])`, byte-identical to the crop, FF-AD15): the panel rises into the slab by the table below. The jamb rails and the lamp (now green) stay. **No drawn in-between frame** |
| **Open** (`ff_staff_door_open`) | the lock is open (latched for the attempt; kept on a checkpoint respawn) | rails only, the panel's 4 px kick-plate lip at rows 0..3 under the slab, a floor sill track at rows 226..227 and a **green** lamp. The lane rows 4..225 between the rails are fully transparent (0 of 3552 px) |

Rising table: the raise in px on ticks 1..12 after the open event (tick 0 is the event tick, drawn as tick 1) is 19, 37, 59, 85, 112, 139, 165, 188, 205, 217, 222, 224. The panel's bottom clears Donut on tick 2 (37 px) and Carl's 52 px with margin on tick 3 (59 px). The table eases out: it moves fast, then settles into the slab.

The sim opens the lane on the event tick itself, so the art is never behind the solid (rule 9.1). A fixture restart snaps the art back to closed on the restart tick. A checkpoint respawn with the lock open draws the open frame at once (no replay of the rise).

**Presenter selection (gate note G4): the lock's state is the source of truth, not a tick difference.** `door_frame(lock_open, open_tick, now)`: not `lock_open` is `Closed`; `lock_open` with `Some(t)` and `t <= now < t + 12` is `Rising(now - t + 1)` (ticks 370..381 give `Rising(1..12)`); anything else is `Open` (a checkpoint respawn, a rewound tick counter, a lock restored with no event). `bell_frame` has the same guard: `ding_a` on `t..t+3`, `ding_b` on `t+4..t+7`, `ding_a` on `t+8..t+11`, otherwise `idle`. `open_tick` is the tick of the attempt's latest `SurfaceLockOpened` event after its latest `Restart`.

The bell rings on the event tick. That is the only cue of the open beat besides the door itself (REST-AD17: no door notice). `ff_bell` plays `ding_a`, `ding_b`, `ding_a` for 4 ticks each, then idles. Optional audio is out of scope here.

## 7. The pickup (Light on Your Feet): states

The canvas is 32 x 24 at (1204, 556). Row 23 is the walking line's last row (y 579). Its centre column (canvas x 16, world x 1220) is the centre of the grant column 1200..1240. The pickup is drawn in the actor plane but **under** the actors (prop layer z, like the kit's props), so Donut walks through it.

| Frame | When | Drawing |
| --- | --- | --- |
| `full_a` / `full_b` | the boost is not held | a gilded box: a `brass` 3..5 body, two raised gold straps, a dark `iron` band with two rivets, a keyhole clasp, and a lid with a `brass` 6 band on rows 6..7 (luma 238) under a `flame` 4 top edge (luma 229). The box's brightest runs are therefore the lid's `brass` 6 band and its `flame` 4 edge, **both below the stand lines' 250** (gate note G1 corrects the earlier "brightest run, luma 229"). The frames alternate on a **presentation** timer, 24 ticks each: a 4-arm `flame` 5 glint on the lid's left corner (`full_a`), then a small one on the right corner with a strap highlight (`full_b`) |
| `empty` | the boost is held | the same box opened and spent. The lid stands up behind it, showing its shadowed underside. The dark empty inside shows over the lit front rim, the clasp hangs open, and the gold is one step duller. No glint (mean luma 85 against 137 full) |
| `ff_reward_sparkle_a..c` | the grant tick only | a 12-tick burst over the box, then `empty` |

**State source:** `empty` if and only if Donut holds Light on Your Feet in the running attempt. `box_frame(held, grant_tick, now, t_present)` (gate note G4): not `held` is `FullA` when `(t_present / 24) % 2 == 0`, else `FullB`; `held` with `Some(g)` and `g <= now < g + 12` is `Sparkle((now - g) / 4)`, drawn **over** the `empty` frame; otherwise `Empty`. `t_present` is the scene's tick counter (FF-AD19). The presenter reads the same state the `LIGHT FEET` notice and Donut's boosted jump read: the grant receipt `f01_safe_room_reward_found`, or the loadout flag on a launch that starts with it. Consequences:
- a launch with the boost already held (the `All` loadout) shows `empty` from the first tick;
- a launch without it shows `full`;
- a fixture restart that re-arms the grant shows `full` again.

The art never decides the grant.

## 8. Lighting (presentation only)

- The dressing removes the dining-room torches and their pools:
  - torches at (336, 448), (848, 176), (848, 448) and (1088, 448) via `remove_at`;
  - `torch_pool_a_m` (232, 364), `torch_pool_a` (744, 92), `torch_pool_a_m` (744, 364) and `torch_pool_a` (984, 364);
  - `grime_a` (600, 376): the restaurant is unexpectedly clean;
  - bay B's `arch_chain` (912, 384) (structure, 144 x 48): the catenary would hang across the restaurant window (review F2, FF-AD27). Bay A keeps its chain: that is the catacomb side.
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

1. **Stand lines are the brightest runs.** The three walkable tops are drawn in `flame` 5 (luma 250): play deck row 513, slide top row 497, shelf row 449. Within 64 px of each, no other horizontal run of 8 px or more is brighter than luma 201 (`stone` 10). **Metric (gate note G2):** "within 64 px" means the run's pixel rect intersects the stand-line segment's rect grown by 64 px on every side (Chebyshev distance); the segments are the play deck x 464..559 at y 513, the slide top x 560..687 at y 497 and the shelf x 1296..1391 at y 449. Under it the box (y 561 and below) is more than 64 px from the shelf line. Examples:
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
5. **The box is the only warm object between x 1136 and 1295.** Nothing gold, orange or yellow stands within 48 px of it. **Test (gate note G3, replacing a column test that a strip tube at x 1168 would fail by construction):** no piece other than the box and its sparkle whose placed rect intersects the box rect grown by 48 px (x 1156..1283, y 508..579) holds a `brass`, `copper`, `flame`, `gal_warm` or `brick` 7+ pixel. The checker's cream `stone` 9/10 is the floor tile and is exempt.

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
| FF-A8 | Palette: every piece on `floor01-catacomb-v2`, at most 63 colours per atlas, 1-bit alpha, at least one transparent px per piece, except pieces declared `opaque` (FF-AD17), which must be fully opaque | `validate_assets.py`, art unittests |
| FF-A9 | No text, glyph, logo or chain trade dress | review |
| FF-A10 | The window still at 640 x 360 shows the door state, the box state and the three stand lines legibly at 1x | owner review of the stills |

## 13. Execution list

1. Pieces of section 5 in `scripts/author_ff01.py` (new functions; keys untouched). Then the atlases, the pixel-grid and `.ase` sources, and the exports through `scripts/export_assets.sh`.
2. Contract `assets/source/terrain/floor01/floor01_fastfood_kit.json`. It extends the opening kit, holds the atlases and pieces, and has a `room_dressing.rest_refuge` block: `remove_at`, `backwall_fill`, `place` and an empty `parallax` block (FF-AD16). It also holds a `state_groups` entry for the door and the box (presenter-drawn, like the heat partition).
3. `src/fastfood_kit.rs`: dressing load and apply through `f01_boiler::compose_kit_hooked`, and the pure presenter frame selection (door, box, bell). Tests FF-A1, A3, A4 and A7.
4. Bevy: `f01_rest::launch` composes with the dressing. The binary replaces the greybox decor rectangles and the door rectangle (`spawn_rest_greybox`, `sync_rest_door`) with the presenter sprites. `f01_rest::decor` stays as data.
5. Evidence at `docs/evidence/area-fastfood-01/` with the window hash equal to the headless hash, docs and the art unittest `tests/test_art_ff01.py`.

## 14. Files

| Path | Content |
| --- | --- |
| `docs/02-art/area-fastfood-01-spec.md` | this spec |
| `scripts/author_ff01.py` | the authoring record (keys, every piece, atlases, manifests, contract, dressing, gate, measurements) |
| `assets/source/props/floor01/ff_door.grid.json`, `ff_pickup`, `ff_wall`, `ff_structure`, `ff_props`, `ff_rock` (+ `.ase`) | the six atlas sources |
| `assets/manifests/floor01_ff_*.json` | their manifests (`decor_atlas`, `prototype_fixture_unapproved`) |
| `assets/source/terrain/floor01/floor01_fastfood_kit.json` | the kit contract: pieces, state groups, `room_dressing.rest_refuge` |
| `src/fastfood_kit.rs` | the dressing and the pure presenter selection (door, box, bell) |
| `src/bin/floor01_scene/fastfood_art.rs` | the Bevy presenter sprites |
| `tests/test_art_ff01.py` | the art unittest |
| `scripts/make_ff01_evidence.sh`, `make_ff01_sheets.py`, `ff01_view.py` | the evidence capture, sheets and drawn-view measurement |
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
| FF-AD15 | The door atlas pre-bakes 14 frames (closed, the 12 `door_raised(RISE[k-1])` crops, open), 24 x 228 each; no runtime crop (gate note G5) | "every rising frame equals `door_raised`" becomes a byte test; the presenter picks an atlas rect |
| FF-AD16 | `ff_rock_fill` is a `backwall_decal` over bay B's opening at (904, 352), and the dressing's `parallax` block is empty | a `parallax` swap replaces the **whole** strip in every opening (bay A's included, where the catacomb must stay "as is") and the mid strip would still draw over it; a decal behind the window and the arch's stone covers bay B's opening only. Flagged as a design note to the top tier |
| FF-AD17 | A tile piece with no transparent pixel is declared `"opaque": true` in the contract, and `env_kit.py` checks it fully opaque instead of requiring one transparent pixel. These are `ff_wainscot_a` and `_b`, `ff_floor_checker_a`, `ff_wall_tile_a`, `ff_ceiling_tile`, `ff_soffit_stone`, both skins and the closed door | the frozen wainscot, checker and closed-door keys are fully opaque by design (a tile that tiles, a panel that fills the solid exactly), so the FF-A8 clause "at least one transparent px per piece" cannot hold for them. Every other piece keeps the clause. Flagged as a design note |
| FF-AD18 | The skins' end caps (`brass` 3) cover rows 2..14, so row 1 is `flame` 5 across the platform's full width | the stand line must span the platform the sim treats as walkable (spec 9.2, gate note G8) |
| FF-AD19 | The box's glint timer (`t_present`) is the scene's tick counter | deterministic stills (the window hash and the captures stay reproducible) with no extra clock |
| FF-AD20 | **Rewritten after the independent review (F4):** the wall tiles run under the hatch, the canopy and the under-panel; the three decals are placed after every tile, so they draw over them. The earlier default (skip any tile that touches one of the three rects) left 8..24 px brick frames | the tile grid is 32 px and the three rects are not; a continuous tile field is the room's identity |
| FF-AD26 | The checker floor, its broken edge and the spill are `structure` pieces, listed before `ff_play_posts` (review F1) | on `backwall_decal` (order 3) they sat under the collision floor deck (order 4) and never rendered; the HEAT kit's floor decals are `structure` for the same reason. A test asserts every placement in y 561..572 draws above the collision layer |
| FF-AD27 | `remove_at` also drops bay B's `arch_chain` (912, 384); bay A keeps its chain (review F2) | the catenary read as a dashed line across the restaurant window |
| FF-AD28 | `ff_play_posts` is 224 x 68 at (464, 512) (review F3) | removes the 16 px wall gap between the slide-top skin and its posts; rows 16..67 are byte-identical to the first execution |
| FF-AD29 | Both right piers are closed (review F5): `backwall_fill` reaches x 319 in bay A, `ff_rock_fill` is 168 wide in bay B, and one more wall tile cell sits at x 1120 | the piers' inner columns are transparent at y 416..527 and showed the gallery parallax; a `fastfood_kit` test asserts no background pixel in either bay |
| FF-AD30 | The box's glint period stays 24 presentation ticks (review item 6) | after F1 the full box's opaque pixels measure 84.9 mean luma against 52.1 for a 4 px ring around its rect on the checker floor: a difference of 32.7, not under 12, so FF-AD23's remedy is not applied. The spent box measures 53.2 against 53.8 (it is meant to recede). The olive hue comes from the floor-wide scene ambient (0.6, 0.62, 0.82); a lighting-identity remedy (the `actors` light group for the box, or a room ambient) is an owner call |
| FF-AD31 | G7 closed as accepted (review item 1): panel 47.8 against wall 72.3 (difference 24.5, ratio 0.66) under the scene ambient, the rail outline unbroken on 160 of 160 rows; FF-AD16 and FF-AD17 accepted | the target 25 was an unlit estimate ("about 60 against 100"); the ratio stays strong |
| FF-AD21 | FF-Q1 answered: no palette extension in this work order; cream `stone` 9/10 and pale `gal` stay (the drift from "blue/white" is known and recorded) | a neutral white above luma 201 would break rule 10.1 within 64 px of the stand lines, most of the room; the route to whiter is a floor-wide palette PR (cool-neutral ramp capped at luma 190, used outside the G2 envelopes). Adopted under the owner's go-with-all authorization (gate recommendation) |
| FF-AD22 | FF-Q2 answered: cool steady strip light in the refuge is acceptable; FF-AD7 and FF-AD8 stand | atlas row 04 itself lists light strips; row 09's gym-only rule binds the gym's high-contrast accent light, and the refuge strips are steady, single-light, unprojected and under the gym tube's values (gate recommendation) |
| FF-AD23 | FF-Q3 answered: no glow on the box (FF-AD6 stands) | if play finds the box lost at 1x, the first remedy is a shorter glint period (24 down to 16 presentation ticks), not an emitter (gate recommendation) |
| FF-AD24 | FF-Q4 answered: the stray ball stays static (FF-AD13 stands) | the roll waits for the owner's game-feel review; if adopted: a bounded non-collidable roll ending at (1012, 554), presentation only (gate recommendation) |
| FF-AD25 | FF-Q5 answered: no paw-switch frames in this work order | the lock opens on standing, not on a press; if ANIM-INTERACT-01 is wired later, the diegetic target is the bell at x 1380 (a presentation-only paw tap on the open tick) (gate recommendation) |

| Id | Open question | Owner of the answer |
| --- | --- | --- |
| FF-Q1 | Should a neutral white/grey ramp be added to the floor palette in a palette PR, for a brighter "unexpectedly clean" restaurant? Answered for this work order by FF-AD21 (no); the owner may reopen it | owner art review |
| FF-Q2 | Is cool steady strip light in the refuge acceptable next to atlas row 09's gym-only accent rule? Answered by FF-AD22 (yes) | owner art review |
| FF-Q3 | Should the box gain a faint glow, if it gets lost at 1x in play? Answered by FF-AD23 (no, shorten the glint first) | owner game-feel review |
| FF-Q4 | REST-Q5: animate the stray ball rolling in (a bounded, non-collidable loop)? Answered by FF-AD24 (static) | owner game-feel review |
| FF-Q5 | Should Donut's paw-switch frames (ANIM-INTERACT-01) play on the shelf moment? Answered by FF-AD25 (not in this work order) | owner, once wired |
| FF-Q6 | **Owner note (gate note G9, non-blocking):** the door's open lamp (dim olive `lichen`) is darker than its closed lamp (`flame` 1/2), and the orange/olive pair is weak for colour-blind players. The keys stay frozen; the panel (a solid steel column against two rails) carries the state and the lamp is a tertiary cue. Should the lamp pair change in a follow-up key edit? | owner art review |

**Horizon fit:** the kit changes no move, ability or gate. It dresses the existing `hero_on_surface` beat and the Light on Your Feet grant of the [progression horizon](../01-design/character-progression-horizon.md) and still fits it.

## 16. Execution record

The checks below were run on the execution head, with their actual results. Everything is prototype art (`prototype_fixture_unapproved`); nothing is approved.

**What was executed.** Every non-key row of section 5 in `scripts/author_ff01.py` (new functions only), six atlases (`floor01_ff_door` with 14 pre-baked frames, `_pickup`, `_wall`, `_structure`, `_props`, `_rock`), their manifests, the contract `floor01_fastfood_kit.json` (112 pieces, three state groups, `room_dressing.rest_refuge`), `src/fastfood_kit.rs` (dressing and the pure presenter selection), the scene's presenter (`fastfood_art.rs`) and the evidence. `f01_rest::launch` dresses only `RestRoom::Refuge`; the camp and the aftermath keep `compose_kit` (a test compares their pieces and strips).

**Gate notes applied.** G1 and G2 (text corrections in sections 7 and 10.1), G3 (the 2D box test, section 10.5 and the unit test), G4 (presenter functions read the lock first), G5 (14 pre-baked door frames, FF-AD15), G6 (`gate_overview.png` kept; the README says it is not regenerated), G7 (measured, below), G8 (skin rows, wainscot cadence, bell, strip lights), G9 (FF-Q6). FF-Q1..Q5 are answered by FF-AD21..25.

**Pins.** The eight key pins are unchanged (`key_sha()` equals `gate/measurements.txt`; the key grids and `.ase` are byte-identical to the committed ones). `MANIFEST_HASH` did not move (the new atlas manifests are not tracked by it). `REFUGE_ROOM_HASH`, `REFUGE_ENCOUNTER_HASH`, `TABLE_HASH`, every golden, replay, fixture hash and `COMBAT_STATE` hash did not move: the full suite is green with the pins as they were. No art allowlist needed a new entry (the export and validators are manifest-driven). One validator was extended: `scripts/env_kit.py` accepts `"opaque": true` on a piece and then requires it fully opaque (FF-AD17).

**Checks (actual results).**

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --locked --all-targets -- -D warnings` | clean |
| `cargo test --locked --all-targets --no-fail-fast` | 3913 passed, 0 failed, 36 ignored (after the rebase onto ACT1-A) |
| `fastfood_kit` tests (21, one ignored evidence helper) | FF-A1 (launched room identical with and without the dressing), FF-A2 (the `COMBAT_STATE` hash equal on every tick of the Story solo-Donut crossing and of a Carl walk), FF-A3 and FF-A4 (frame selection over the `rest_refuge_solo_donut` ticks, a fixture restart, a checkpoint respawn, the `All` loadout), FF-A7 (light budget, net minus 8 plus 6), the `remove_at` and placement rules, the floor decals above the collision layer (F1), no background in either bay (F5) |
| `python3 scripts/env_kit.py validate --contract ...floor01_fastfood_kit.json` | VALID (112 pieces) |
| `python3 scripts/validate_assets.py` | exit 0, 0 invalid |
| art unittests (`tests/test_art_*.py`, `tests/test_pixelart_*.py`, including `tests/test_art_ff01.py`, 27 tests, and `tests/test_art_env_kit.py`) | 1242 and 111 tests OK |
| `bash scripts/export_assets.sh` twice | tree clean after the second run |
| `python3 scripts/check_docs.py` | passed |
| `scripts/ci_floor01_combat.sh headless` and `render` | `F01_LATE_HEADLESS_OK` and `CAMERA_RENDER_OK` |
| `scripts/check_f01_owner_commands.sh` | exit 0 |
| `scripts/make_ff01_evidence.sh` | 33 stills (11 views x 3 sizes), every one twice byte-identical, `hash_match=yes` for all 33, and equal to the REST-WIRE hash for the 9 stills at a REST-WIRE tick (`threshold` 20, `door_closed` 560, `door_rising_376`) |

**Gate note G7 (door against the wall tile; accepted by the review as FF-AD31).** In the runtime `door_closed` still (drawn view left 960, top 292) the door panel's interior (x 1441..1454, y 368..527) has mean luma 47.8 and the wall field beside it (x 1408..1435) 72.3: a difference of 24.5 (ratio 0.66) against an unlit-estimate target of 25. The rail's `stone_cold` 0 outline is unbroken (darker than both neighbours on all 160 rows). No key change.

**Rule 10.1 at runtime (G2 metric).** In every still where a stand line is visible it is the brightest 8 px run of its 64 px envelope, by 44 (deck, slide top; the checker floor is now the nearest rival) and 77 (shelf) luma steps in the lit scene (`measurements.txt`). The pixel-model test passes.

**Independent review (PASS-with-fixes, F1..F5) and its fixes.** F1 (the checker floor was invisible on `backwall_decal`), F2 (bay B's chain), F3 (the 16 px post gap), F4 (brick slivers) and F5 (the gallery showing through both right piers) are applied as FF-AD26, FF-AD27, FF-AD28, FF-AD20 (rewritten) and FF-AD29. New tests: `test_f1`, `test_f3`, `test_f4` and `test_f5` in `tests/test_art_ff01.py`, `the_floor_decals_draw_above_the_collision_layer` and `no_background_shows_in_either_bay` in `fastfood_kit`, and `test_opaque_option_is_opt_in_and_never_weakens_the_alpha_check` in `tests/test_art_env_kit.py`. G7 is closed as accepted (FF-AD31); the glint period stays (FF-AD30). The evidence was re-captured on the fixed head.

**Owner notes (non-blocking, no action in this work order).**
- FF-Q6: the door's lamp pair is weak for colour-blind players; the panel carries the state.
- FF-Q3 (extended): the "gilded" box and the concept's yellow accents (skins, chute, kick strip) drift to olive under the floor-wide scene ambient (0.6, 0.62, 0.82). Candidate remedies, not adopted: draw the box and its sparkle in the `actors` light group, or give the refuge a room-level scene ambient. Both change the lighting identity.
- The booth under the window is blue on blue and low-contrast at 1x; the ball pit reads as a dark tray with olive specks; the hatch reads like a dark monitor; the rock's `stone_cold` blobs read a little like holes and its cracks like thin vector squiggles; the play structure reads as two khaki planks over a dark tray (acceptable within the palette and rule 10.1).
- At runtime `door_closed` (tick 560) the box is already spent (the companion crossed it at tick 369), so the gate composite's "box untouched" was not the real state.
