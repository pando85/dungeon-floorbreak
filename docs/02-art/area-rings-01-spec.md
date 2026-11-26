# AREA-RINGS-01: the Broken Ring kit of Floor 01 rooms 4a and 4b, design spec

**Status:** design spec and three keys frozen (`ring_arc_wall_a` hero key, `ring_shutter_closed`, `ring_camp_a`). Silhouette gate sheets are in [`docs/evidence/area-rings-01/gate/`](../evidence/area-rings-01/gate/), and the gate verdict is pending. Execution of the rest of the kit, the room dressing and the Bevy wiring is pending (section 13). Independent review is pending. Not owner-approved. Launch: `floor01_scene --scene f01_late --f01-run`, rooms 4a `ring_court` and 4b `ring_split`.

**Route:** difficulty score 2 (a new environment family on the approved ENV-01 kit language, plus a state-keyed shutter), route **B + silhouette gate** ([routing by difficulty](../07-agents/pixel-art-pipeline.md#routing-by-difficulty)). The design step stops at this spec and the three keys. Work order: AREA-RINGS-01 ([playtest integration plan](../07-agents/f01-playtest-integration-plan.md) section 15, row `AREA-RINGS-01`; KR2, KR9, KR14, KR16 and KR17). It also covers the drawing half of RING-D: shutter, soffit and landmark drawing.

**Authorities:**
- **Design intent:** owner draft [PR #189](https://github.com/pando85/dungeon-floorbreak/pull/189) (`docs/01-design/floor-01-broken-ring-labyrinth.md`, `docs/07-agents/floor-01-broken-ring-workorder.md`; not on main and never pushed to). This spec cites it. Every divergence from it is an adopted default in section 15.
- **Geometry:** `assets/levels/floor01/f01_ring_court.room.json` and `f01_ring_split.room.json` with their `.encounter.json` sidecars (pinned hashes, read-only here), `src/f01_ring.rs` and `src/env_prop.rs` (`Shutter`).
- **Mood only:** [atlas panel 08](../../assets/concept/floor-01/atlas-v1/08-circular-maze-survivor-camp.png). No pixel is cropped, traced or resampled from it.
- **Environment language:** the [ENV-01 kit](floor-01-catacomb-kit.md) contract and palette.
- **Precedents:** the [gym kit](area-gym-01-spec.md) (dressing pass and gate) and the [room exit spec](floor01-room-exit-spec.md) (`dfx_exit` owns the 64 x 96 exits of 4a and 4b).
- **Pipeline and tone:** [art direction](art-direction.md), [asset pipeline](asset-pipeline.md), [antihero tone](../01-design/antihero-tone.md).

## 1. Canon and tone constraints

- **What the rooms are:** the inside of a broken, half-buried concentric stone ring. Someone built it to impress, and nobody has maintained it in a long time. Old dressed stone is set into the catacomb brick:
  - three concentric carved courses (two voussoir bands with a notched band between them);
  - an off-axis missing segment;
  - hairline cracks, chipped arrises, a few spots of old damp lichen;
  - weathered chain stubs.
  It is impressive by scale and wear, not by ornament.
- **The camp is the first sign of living people:** salvage crates, a torn canvas lean-to, a rolled bedroll, a tied cloth bundle, one warm lantern. All of it is improvised, cheap and used. **No figure stands in the room**:
  - the survivors appear only as anonymous distant silhouettes in the 4b backdrop (KR14, KR16);
  - no named survivor; no Odette;
  - no faces or identifiable features.
- Mostly stone and gold torchlight (#189). The parallax tiers are cold blue and carry small warm lamp dots. **No ultraviolet, no neon, no audience.** The kit must not read as the Ball of Swine arena: no stands of spectators, no pig emblems, no banners with marks.
- **No text, glyphs, logos or heraldry.** The crowned keystone and the three-notch shield are plain carving: blunt points and V notches. They are not a real emblem or a franchise mark.
- **Engine ids are IP-neutral:** `ring_*` pieces, `floor01_ring_*` atlases, `floor01_rings_kit.json`, `survivor_silhouette_a..d`.

## 2. Scale and coordinates

Carl is 52 px, about 1.8 m. All coordinates are **room layout pixels** (top-left origin, y down), as composed by `f01_boiler::compose_kit` for the two rooms. `W` is the walking line.

| Line | y (both rooms) | Note |
| --- | --- | --- |
| Walking line `W` | 580 | room `floor_y` 576 on the 16 n + 4 deck line |
| Floor back line `FB` | 560 | `W - 20`: props stand here |
| Soffit band | 336..352 | `W - 244 .. W - 228`; the jump ceiling is its underside (the ceiling slab solid `f01_ring::launch` pushes) |
| 640 x 360 view | 292..652 | the HUD plates cover about y 292..352 at both corners |
| 640 x 400 view | 272..672 | |
| Visible strip above the soffit (360 view) | 292..336 | 44 px, mostly under the HUD in the corners |

| 4a `ring_court` (2048 wide) | Rect (x, y, w, h) | Note |
| --- | --- | --- |
| Entry door (`door_timber`, composer) | 216, 484, 48, 88 | kept; the crowned keystone sits over it |
| Protected spawn / checkpoint | x 16..224 / x 176 | |
| Step | 1056, 516, 64, 16 | |
| Stone walkway | 1120, 500, 192, 16 | top `W - 80`, slab underside 512 (`W - 68`), KR2 |
| Shutter target | 1200, 512, 32, 68 | static closed (`initial: closed`, `cycles` 0) |
| Donut shelf | 1536, 452, 96, 16 | |
| Exit (`dfx_exit`) | 1968, 484, 64, 96 | |
| Goblins (roster) | x 1424, 1824 | |

| 4b `ring_split` (1152 wide) | Rect (x, y, w, h) | Note |
| --- | --- | --- |
| Overhead route | 336, 516, 384, 16 | |
| R3 view ledge | 896, 516, 96, 16 | |
| Exit (`dfx_exit`) | 1072, 484, 64, 96 | |
| Goblins (roster) | x 560, 848 | |

## 3. Room layout (zones, left to right)

**4a, the Shutter Court.** The court reads as one walk *into* the broken ring:
1. **Entry (x 0..1012).** The composer's ground arch bays and the gallery stay. The **crowned keystone** sits over the entry door: the first landmark, "you are entering something built".
2. **The great arc (x 1012..1420).** `ring_arc_wall_a` frames the step, the walkway and the shutter. Its centre x 1216 is the shutter's centre. Its missing segment opens at about x 1300..1380, over the right end of the walkway. Two **fallen voussoirs** lie on the floor back line under the break. The **shutter** fills the arch under the walkway, between two **guide jambs** and under a **slot housing**. The **three-notch shield** is carved on the wall under the walkway, right of the shutter. Fight-relevant reading: the shutter is the darkest, most solid mass in the frame, and its iron shoe is the brightest line in it.
3. **The full ring (x 1656..1864).** `ring_relief_landmark` is the one complete (broken) ring, entire in a 640 x 360 frame from the floor (KR9). The second goblin fights in front of it. The Donut shelf passes in front of its left side.
4. **Exit (x 1864..2048).** Brick to the `dfx_exit` door.
5. **Overhead, full width:** the **soffit band** at 336..352 and the **upper arcs** in the 44 px strip above it.

**4b, the Split.** It reads as the ring opening toward the camp:
1. **Entry (x 0..324).** The composer's arches stay. `door_timber` stays.
2. **The second arc (x 324..732).** `ring_arc_wall_b`, the mirror-free variant with the break on the left, frames the overhead route. The route's ledge front passes in front of it.
3. **The camp outskirts (x 744..888).** `ring_camp_a` on the floor back line, between the route and the R3 ledge. The goblin at 848 fights in front of it.
4. **R3, the view (x 896..1072).** Over the R3 ledge, `ring_breach_r3` is a hole broken through the ring wall. It is framed by fractured voussoirs and shows the far camp: cloth over a carved stone gate, two lantern dots, crates and two anonymous silhouettes. The parallax behind it is `ring_camp_far`.
5. **Exit (x 1048..1152).** The **shared marker** `ring_marker_a` (the torn cloth strip and warm lantern, KR14) stands just before the `dfx_exit` door. REST-B places the same piece at its entrance.
6. **Overhead:** the soffit and the upper arcs, as in 4a.

## 4. Mapping onto the room solids (presentation only)

- **Mechanism (`RNG-AD1`):** the kit is a **dressing pass**, the gym precedent.
  - `floor01_rings_kit.json` is an `env-kit-v2` contract that extends the opening kit.
  - It carries per-room `room_dressing` blocks (`ring_court`, `ring_split`), each with `remove`, `place[{layer, piece, x, y}]` and `parallax` (the piece swap of `parallax_far` and `parallax_mid`).
  - A pure `src/ring_kit.rs` applies a block to the composed layout before `EnvKit::build`.
  - `f01_ring::launch` calls it through a `compose_kit` variant with a dressing hook.
  - No room or encounter JSON changes.
- **Never a collider.** No piece adds, moves or removes a solid, the soffit slab included: the soffit art is drawn exactly over the slab solid `launch` already pushes. These stay byte-identical with and without the dressing:
  - solids, spawn, exit, views, walking line, the blink barriers;
  - every ring fixture hash and the pinned room and sidecar hashes;
  - the `COMBAT_STATE` hash on every tick.
- **Ledge fronts** are `structure` pieces laid exactly on the platform rects:
  - a 32 x 16 middle tile, plus 8 x 16 left and right end caps that overlap the first and last 8 px;
  - they replace the thin brick ledge look and do not change the collision tiles (`compose_kit` already clears the platform face rows).
- **Parallax (`RNG-AD2`):** shows only where the composer already clears the backwall (the ground arch bays). No backwall tile is edited. The R3 view is an opaque `backwall_decal` with the far camp painted inside its frame.
- **Arcs are `backwall_decal`.** Their bottom rows sit at `W - 16`, so the legs dive 4 px behind the deck's back row. That reads as "the ring continues below the floor", and an arc never closes into an arch opening.

## 5. Piece list

`K` = frozen key drawn with this spec. Every other row is executed to this spec. Layers are the kit's: `parallax_far` 0, `parallax_mid` 1, `backwall_decal` 3, `structure` 5, `prop` 6, `foreground` 8.

| Piece | Atlas | Size | Layer | Placed at (x, y) | Design |
| --- | --- | --- | --- | --- | --- |
| `ring_arc_wall_a` **K** | `floor01_ring_wall` | 408 x 164 | backwall_decal | 4a: 1012, 400 | Three concentric courses of a ring whose centre is 48 px below the piece bottom. Bands, from outside in: outer voussoirs r 184..208 (joints every 7.5°, keystone 4 px proud); groove 3 px; notch band r 167..181 (a 4 px recess every 5°); groove 3 px; inner voussoirs r 148..164 (joints every 9°); cast shadow r 143..148 on the brick. The break is at +31..+51° from the apex, stepped per band, with fracture faces and a ragged mortar rim. The brick shows through the break. Two chain stubs hang at -21° and +17.5°; hairline cracks; lichen only in the bottom 34 rows. |
| `ring_arc_wall_b` | `floor01_ring_wall` | 408 x 164 | backwall_decal | 4b: 324, 400 | The same geometry, executed with its own parameters: the break at -48..-30° (left), its own crack and chip seeds, one chain stub at +20°. Not a mirror of `a` (the key light stays upper left). |
| `ring_relief_landmark` | `floor01_ring_wall` | 208 x 208 | backwall_decal | 4a: 1656, 356 | The one full ring: the key's band profile scaled to r 104 (outer voussoirs 86..104, notch band 74..84, inner voussoirs 60..72). It encloses a recessed inner disc of `stone_cold` 1..2 with one concentric groove and a 10 px centre boss in `stone` 4..6. The missing segment is off-axis at the upper right (+40..+68°); three fallen blocks lie at its foot in the piece's lower right, inside the band. Top at y 356 = `W - 224`, under the soffit and below the KR9 limit of `W - 288`. |
| `ring_arc_upper_a`, `_b` | `floor01_ring_wall` | 256 x 44 | backwall_decal | both rooms: x = 256 n, y 292 (a, b alternating) | The strip above the soffit: segments of two larger rings (r about 400) crossing as shallow arcs, outer voussoirs and notch band only, values one step darker than the key (in the soffit's shadow). Transparent elsewhere (brick shows). |
| `ring_keystone_crowned` | `floor01_ring_wall` | 48 x 40 | backwall_decal | 4a and 4b: 216, 440 | Entry landmark (#189 landmark i): a tall keystone with a carved crown of three blunt points. The right point is snapped off and shows a fracture face. Two short side voussoirs. No emblem. |
| `ring_shield_notched` | `floor01_ring_wall` | 32 x 40 | backwall_decal | 4a: 1256, 524 | #189 landmark ii: a plain shield shape in relief, with three V notches cut in its top edge, carved into a dressed panel on the wall under the walkway, right of the shutter. Stone 4..7, lit on its upper left arrises. |
| `ring_breach_r3` | `floor01_ring_wall` | 176 x 112 | backwall_decal | 4b: 896, 380 | An opaque hole broken through the ring wall, framed by fractured voussoirs on the left and top. Inside, the far camp: a carved stone gate draped with cloth (stone 6..8), two lantern dots (`gal_warm` 5/6 cores), two stacked crates, and `survivor_silhouette_a` and `_b` (flat `gal` 0..1 figures, 14..20 px tall, one seated, one standing; no face, no detail). The view is cold `gal` 2..4 beyond. |
| `ring_soffit` | `floor01_ring_structure` | 32 x 16 | structure | both rooms: x = 32 n, y 336 | The flat underside of the ring's walkway course (KR9): a dressed-stone fascia. The underside row (y 351) is lit warm from below (stone 7), the face is stone 3..5 with a joint every 32 px, and the top row is dark mortar. It tiles seamlessly. |
| `ring_ledge_mid` | `floor01_ring_structure` | 32 x 16 | structure | every platform: x = rect.x + 32 n, rect.y | Ring-stone ledge front: a lit top arris, a dressed face and a 4 px shadowed underside. |
| `ring_ledge_end_l`, `_r` | `floor01_ring_structure` | 8 x 16 | structure | platform ends | Rounded return ends. They overlap the outer 8 px of the run. |
| `ring_shutter_jamb_l`, `_r` | `floor01_ring_structure` | 8 x 68 | structure | 4a: 1192, 512 / 1232, 512 | Guide channels of the arch: dressed stone with a dark 2 px slot facing the shutter. |
| `ring_shutter_housing` | `floor01_ring_structure` | 48 x 6 | prop | 4a: 1192, 512 | The slot lintel under the walkway. It draws **over** the shutter's top 6 rows, so a closing slab emerges from under it. |
| `ring_shutter_closed` **K** | `floor01_ring_shutter` | 32 x 68 | prop | shutter rect (x, y) | Section 6. |
| `ring_shutter_grit_a`, `_b` | `floor01_ring_shutter` | 32 x 28 | prop | (x, y + 6), Warning only | 4..6 falling grit specks (stone 5..7, mortar 2) with a 1 px dust streak, alternating every 6 ticks. |
| `ring_shutter_shadow_a`, `_b`, `_c` | `floor01_ring_shutter` | 40 x 4 | prop | (x - 4, W - 4), Warning, Closing | The slab's shadow gathering on the floor under the slot: 25 %, 50 % and 75 % checker density of `mortar` 0. |
| `ring_shutter_dust_a`, `_b`, `_c` | `floor01_ring_shutter` | 48 x 10 | foreground | (x - 8, lower_edge_y - 6) | Scrape dust at the lower edge while moving, and a floor-contact puff on entering Closed. Stone 5..7 puffs, 1-bit. |
| `ring_fallen_voussoir_a`, `_b` | `floor01_ring_camp` | 32 x 16, 24 x 14 | prop | 4a: 1328, FB - 16 / 1358, FB - 14 | Broken outer-band blocks lying on the floor, one on its side showing a notch. The same stone values as the key. |
| `ring_camp_a` **K** | `floor01_ring_camp` | 144 x 72 | prop | 4b: 744, 488 | Section 11.2. |
| `ring_crates_b` | `floor01_ring_camp` | 56 x 40 | prop | 4b: 992, 520 | Two salvage crates side by side with a coil of rope. It fills the floor below and right of the R3 ledge. Uses the key's crate rules. |
| `ring_marker_a` | `floor01_ring_camp` | 24 x 56 | prop | 4b: 1048, 504 | **The shared REST-B marker (KR14):** a salvaged pole with a torn strip of the camp canvas tied at the top and the camp lantern hung below it. The lantern matches the key's lantern pixel for pixel. |
| `ring_tiers_far` | `floor01_ring_parallax` | 440 x 232 | parallax_far | 4a | The far ring seen through the arches: curved colosseum-like tiers in `gal` 2..6 with arched openings, dark grates, and 6..8 single warm lamp pixels (`gal_warm` 5). No figures. It replaces `gallery_far`. |
| `ring_tiers_mid` | `floor01_ring_parallax` | 344 x 232 | parallax_mid | 4a, 4b | A nearer curved arcade bridge with a balustrade, two stone-toned cloth strips, `gal` 3..7. It replaces `gallery_mid`. |
| `ring_camp_far` | `floor01_ring_parallax` | 440 x 232 | parallax_far | 4b | The far camp beyond the ring: two canvas awnings, lantern dots, a campfire glow (`flame` 2..3 core, `gal_warm` 3..4 spill, 1-bit), and `survivor_silhouette_c` and `_d` (flat `gal` 0..1, at most 24 px, no faces). |

## 6. The stone shutter: frames and timing

The presenter reads only `Shutter::phase()`, `edge_px()`, `lower_edge_y()` and `footprint()` (`src/env_prop.rs`). Any timer it keeps is presentation-only and never feeds the sim. Defaults: 4 px slices (17 for 68 px), 2 px per tick (34 closing ticks), warning 40 ticks.

| Phase | Shutter art | Extras |
| --- | --- | --- |
| `Open` | nothing (the housing and jambs stay) | none |
| `Warning` (40 ticks) | nothing | `grit_a`/`_b` alternating every 6 ticks; `shadow_a` on ticks 0..13, `_b` on 14..26, `_c` on 27..39 |
| `Closing` | the key's **bottom `edge_px` rows**, drawn at (x, y) | `shadow_c`; `dust_a`/`_b` alternating every 4 ticks at the lower edge |
| `Closed` | the full key at (x, y) | `dust_c` for 8 ticks after entering Closed (floor contact), then nothing |
| `Opening` | the key's bottom `edge_px` rows at (x, y) | `dust_a`/`_b` every 4 ticks |

- **Crop rule (`RNG-AD5`):** the slab slides down out of the slot, so its leading iron shoe is always the first thing seen. The housing hides the top 6 rows, so a slab shorter than 7 px shows only under the slot shadow.
- **One key, no drawn intermediate frames:** every moving frame is an exact crop of `ring_shutter_closed`. Travel strip: [`shutter_travel_4x.png`](../evidence/area-rings-01/gate/shutter_travel_4x.png).
- **Static court (`RNG-AD6`):** the court's shutter is static closed (`initial: closed`, `cycles` 0). In-scene it shows the closed key only. The moving phases are drawn, wired and unit-tested on a core `Shutter` driven through a full cycle. Their in-scene capture follows RING-C wire (`RNG-OQ3`).
- **Key design (32 x 68):** four dressed courses of 17 px, so each course matches 17 px of travel:
  - darker and cooler than the ring relief (stone 2..5);
  - a hard 1 px mortar outline on both sides;
  - two iron straps with rivets;
  - a small carved broken-ring roundel in the second course (the court's mark, not a glyph);
  - an 8-row iron shoe: lit top row, body, a bright scuffed sweep edge (iron 6, **the lower-edge cue**, #189), and four blunt teeth with transparent gaps;
  - top 3 rows: the slot shadow.

## 7. Palette

`floor01-catacomb-v2` only, with **no new hue and no new value** (`RNG-AD7`). #189 allows up to two extra stone values; none is needed.

| Use | Ramps |
| --- | --- |
| Ring stone, keystone, shield, ledges, soffit | `stone` 1..8, `mortar` 0..4 for joints, grooves and shadow |
| Recesses, landmark inner disc | `stone_cold` 1..2 |
| Shutter | `stone` 2..5, `iron` 1..6, `mortar` 0..3 |
| Chains, crate brackets, pot | `iron` 2..6 |
| Camp canvas and cloth | `stone` 3..8 (warm tan) |
| Crates, poles | `wood` 0..5 |
| Bedroll | `lichen` 0..2 |
| Lantern | `brass` 2..5, `flame` 3..4, `gal_warm` 3..5 |
| Parallax tiers, far silhouettes | `gal` 0..7, `gal_warm` 5 lamp pixels |
| Weathering | `lichen` 0..1, at most 1 % of a piece, bottom third only |

Every atlas must stay at or under 63 colours. The keys use 20, 17 and 33.

## 8. Lighting (presentation only)

| Source | Treatment |
| --- | --- |
| Wall torches (composer) | Unchanged positions and emitters. Baked key light on every ring piece comes from the upper left (`LIGHT = (-0.55, -0.83)`). The tops of arcs sit one value darker (the soffit's shadow). The legs are lit by the torches. |
| Camp lantern, marker lantern | **Baked only** (`RNG-AD11`): glow pixels in the piece, no new A1 emitter. The light budget is unchanged. |
| R3 and 4b parallax | Lamp and fire pixels are baked into the backdrop. No emitter. |
| Shutter | No light. Its shadow on the floor is the `shadow_*` pieces. |

## 9. Collision and state

The dressing and the shutter presenter are read-only. The authoritative state is unchanged:
- solids (the soffit slab, platforms, walls, the shutter's `solid_rect`);
- spawn, exit, the views and the walking line;
- the blink barriers, the encounter zones and the roster.

`ring_kit` tests assert equality with and without the dressing. A window capture's final `COMBAT_STATE` hash must equal the headless run's.

## 10. Readability rules

1. **The shutter is the strongest mass in its frame:**
   - darker than the arc and the brick around it;
   - outlined;
   - its sweep edge is the brightest line within 64 px of it.
2. **No arc band, notch or chain falls behind an actor's head line** (`W - 40 .. W - 60`) inside the lane between the step and the exit, except the arc legs at their outer x. The legs are lighter than the brick, so dark actors still separate.
3. **Arcs never read as arch openings:** they are always backed by the brick (transparent inside). Their legs end in the deck, and the break shows brick, never the gallery.
4. **Camp props stay below `FB - 72` and never cover a floor node or an actor spawn.** The camp is behind the actors' layer.
5. **Parallax tiers stay at least two values darker than the nearest lit brick,** so the opening reads as depth.
6. **Silhouettes:** flat, faceless, at most 24 px tall, at most two per piece; never in the play lane.

## 11. The keys

### 11.1 Hero key: `ring_arc_wall_a` (408 x 164)

| Part | Rule |
| --- | --- |
| Geometry | Centre (204, 212) piece-local; radii as in the piece table; angles from the apex, + to the right. |
| Outer voussoirs | 7.5° pitch, phase 3.75°, a 1 px `mortar` 2 radial joint. Per-block tone ±0.45, a soft pillow toward the block centre. A bevel 2 px on each radius: lit when it faces the key light. |
| Keystone | \|a\| < 4.2°, 4 px proud of the outer radius, one value lighter, a dark 1 px border. |
| Notch band | 5° pitch. A 4 px recess cut in the outer two thirds of the band (`mortar` 2, the left wall `mortar` 3 on the left half). |
| Inner voussoirs | 9° pitch, offset half a block from the outer. |
| Grooves | `mortar` 1. A 1 px lit lip (`mortar` 4) on the light-facing side. |
| Cast shadow | 5 px below the inner band: 2 px solid `mortar` 1, then a 50 %, then a 25 % checker. |
| Break | Outer +31..+51°, notch +33.5..+48°, inner +35..+46.5° (stepped). Fracture faces 3 px: pale on the left end, dark on the right end. A ragged `mortar` 2..3 rim 2..4 px wide on the bed. The rest of the gap is transparent. |
| Chains | Iron staples in the inner band at -21° (5 links) and +17.5° (3 links, torn). Links alternate face-on ovals and edge-on bars, `iron` 3..6. |
| Weathering | Six hairline crack polylines. Chipped outer arris pixels (transparent). Lichen 1 % in the bottom 34 rows. |
| Grain | Rounded values with a sparse ±1 grain (7 % each way). No screen-wide ordered dither. |

### 11.2 Camp key: `ring_camp_a` (144 x 72)

Read left to right:
1. a salvaged pole with a lashing;
2. a torn canvas lean-to: folds every ~9 px, its torn hem facing the lantern, the dark interior open at the floor;
3. a dull green bedroll with two leather ties, rolled end to the left;
4. two stacked salvage crates with plank gaps, diagonal braces and iron corner brackets, and a strip of the canvas over the top crate's corner;
5. the **camp lantern** on the top crate: brass frame, `flame` core, a 1-bit `gal_warm` spill;
6. a tied cloth bundle and a dented pot.

Bottom row on `FB`.

### 11.3 Shutter key

See section 6.

## 12. Acceptance targets

1. **Silhouette gate (pipeline):** a fresh top-tier agent compares the keys at 1x in the lit scene against the concept crop and passes them. Sheets: [`gate/`](../evidence/area-rings-01/gate/).
2. **In both rooms of `--scene f01_late --f01-run` at 640 x 360 and 1920 x 1080:**
   - the court reads as the inside of a broken ring at every camera stop;
   - the shutter reads as a heavy closing barrier, not a door;
   - 4b reads as "people are near" before REST-B.
3. **Native captures:** every still is captured twice, byte-identical. The final `COMBAT_STATE` hash of every window equals the headless run's.
4. **Byte-identical state:** `ring_kit` tests prove solids, spawn, exit, views, walking line, blink barriers and the ceiling slab are byte-identical with and without the dressing. The ring fixtures and the pinned room and sidecar hashes are unchanged.
5. **Shutter presenter unit test:** on a core `Shutter` through Open → Warning → Closing → Closed → Opening, the presenter's piece and crop for each tick equal section 6's table.
6. **No greybox stand-in:** the `shutter (greybox stand-in)` piece no longer draws in 4a. The court's shutter is the key.
7. **Export:** `scripts/env_kit.py validate --contract assets/source/terrain/floor01/floor01_rings_kit.json` passes. Two export passes are byte-identical. `python3 scripts/validate_assets.py` and the art unittests pass. Every atlas has at most 63 colours, 1-bit alpha, and at least one transparent pixel per piece.
8. **Readability:** section 10 rules hold in the captures (the reviewer checks rules 1, 2 and 4 on the court and split stills).
9. **Rights:** nothing traced or resampled; no text, emblem or likeness; ids IP-neutral; silhouettes anonymous (KR16).

## 13. Execution list

1. Extend `scripts/author_rings01.py` with every non-key piece of section 5. **Keys stay byte-identical**; `ring_arc_wall_b` gets its own parameter set. Pack into five atlases: `floor01_ring_wall`, `_structure`, `_shutter`, `_camp`, `_parallax`. Write the manifests (`decor_atlas`, `prototype_fixture_unapproved`, provenance) and `floor01_rings_kit.json` (with `room_dressing` for both rooms).
2. `src/ring_kit.rs`: a pure dressing loader, `dress()`, and the parallax swap, with tests (section 12.4).
3. `f01_ring::launch`: build the room kit through the dressing hook.
4. `src/bin/floor01_scene/env_prop_art.rs`: replace the stand-in with the shutter presenter (section 6). The pure frame selection goes in a headless module with the unit test of section 12.5.
5. Evidence: `docs/evidence/area-rings-01/`:
   - native 1x and 1080p stills of 4a (entry, great arc and shutter, full ring, exit) and 4b (arc and route, camp, R3, marker and exit);
   - the hash lines, a contact sheet at 1x and 4x, and a layout diagram over a real still;
   - README and `SHA256SUMS`.

## 14. Files

| File | Content |
| --- | --- |
| `scripts/author_rings01.py` | the authoring record (keys now; the rest at execution) |
| `assets/source/props/floor01/ring_wall.grid.json` + `.ase` | `ring_arc_wall_a` key |
| `assets/source/props/floor01/ring_shutter.grid.json` + `.ase` | `ring_shutter_closed` key |
| `assets/source/props/floor01/ring_camp.grid.json` + `.ase` | `ring_camp_a` key |
| `docs/evidence/area-rings-01/gate/` | silhouette gate sheets: keys and silhouettes at 1x and 4x, the shutter travel strip, the court composite at 1x and 2x and its before still |

At execution, the key grids are packed into the five atlas grids of section 13. The single-key grids are then replaced by their atlases, and the keys' pixels are preserved.

## 15. Adopted defaults and open questions

| ID | Adopted default | Reason |
| --- | --- | --- |
| RNG-AD1 | The kit is a contract dressing pass (`floor01_rings_kit.json`, per-room `room_dressing`), applied by a pure `ring_kit.rs` from `f01_ring::launch`. No room or encounter JSON edit. | The gym precedent. The pinned room and sidecar hashes stay. |
| RNG-AD2 | Parallax shows only through the composer's existing openings. The R3 camp view is an opaque breach decal. No backwall tile is cleared. | No tile-layer change; the R3 reveal is guaranteed in both view heights. |
| RNG-AD3 | Every ring arc and relief is `backwall_decal`. The arcs' bottoms sit at `W - 16`. | The relief is never a collider (KR9), and the legs dive behind the deck so no arc reads as a door. |
| RNG-AD4 | The soffit is drawn as `structure` tiles at `W - 244 .. W - 228` across each room's width, exactly over the existing ceiling slab solid. | KR9: the bonk line matches the drawn geometry. |
| RNG-AD5 | Moving shutter frames are bottom-row crops of the closed key keyed by `edge_px`, under a 6-row housing. No drawn intermediate frames. | Exact match with the sim's travel at any speed; one source of truth. |
| RNG-AD6 | In-scene, the court's shutter is static closed. The moving states are drawn and unit-tested now, and their in-scene capture follows RING-C wire. | The court has no animated sim state (`cycles` 0). The brief's fallback. |
| RNG-AD7 | No stone value beyond the kit palette. | The keys read without one. Diverges from #189's allowance of two. |
| RNG-AD8 | The crowned keystone is three blunt carved points with one snapped, over the entry door of both rooms. | #189 landmark i, without heraldry. |
| RNG-AD9 | The three-notch shield is a plain relief shield with three V notches on its top edge, under the walkway right of the shutter. | #189 landmark ii ("next to the shutter"), without glyphs. |
| RNG-AD10 | The camp proper sits in 4b x 744..888 (between the route and R3). R3 shows the far camp through the breach; REST-B owns every actor. | KR14 (the Ring owns the view). The free floor in R3 (x 992..1072) is too narrow for the camp. Diverges from #189's "improvised camp" in R3: the camp's edge is near and its heart is seen. |
| RNG-AD11 | Lanterns and the far campfire are baked glow, with no new light emitter. | The A1 light budget is unchanged; the change stays presentation only. |
| RNG-AD12 | Ring stone uses the warm `stone` ramp. The shutter slab is darker (`stone` 2..5 plus iron). The tiers are cold `gal` with `gal_warm` lamp pixels. | #189: "mostly stone and gold torchlight", "no ultraviolet". |
| RNG-AD13 | Ledge fronts in ring stone (32 px middle tile and 8 px end caps) replace the thin brick ledge look of every platform in both rooms. | The walkway, step and shelves read as built parts of the ring, not floating bricks. |
| RNG-AD14 | Distant survivors are `survivor_silhouette_a..d`: flat `gal` 0..1, at most 24 px, no faces, at most two per piece, only in `ring_breach_r3` and `ring_camp_far`. | KR14, KR16. |
| RNG-AD15 | The concept's red hanging banners are dropped. Torn canvas in stone tones carries the cloth motif. | Distinct from the Ball of Swine arena (#189); no marks. |
| RNG-AD16 | `door_timber` (composer, x 216) stays in both rooms under the crowned keystone. The composer's exit marker stays suppressed by `room_exit_art` (`dfx_exit`). | Reuse; the entry reads as a door into the ring. |

| ID | Open question | Default until answered |
| --- | --- | --- |
| RNG-OQ1 | Should the camp lantern and the marker lantern be real A1 light emitters (one each) in 4b? | Baked glow (RNG-AD11). |
| RNG-OQ2 | Who owns the shutter's stone-scrape and grit audio (#189 asks for "stone scraping" during the warning)? RING-D appends cue reuse in `audio.rs`; new sounds are not in this art kit. | Silent art; RING-D reuses an existing cue. |
| RNG-OQ3 | Will the court's shutter become a moving machine (RING-C wire) in 4a, and in which room should the moving-state capture be taken? | The moving frames are unit-tested only (RNG-AD6). |
| RNG-OQ4 | Where exactly does REST-B place `ring_marker_a` at its entrance? | REST-B places it; this kit ships the piece. |
| RNG-OQ5 | Game feel: is one full ring per court plus two arcs enough "awe" for the RING beat, or does the owner want a larger arc above the soffit in the 640 x 400 view? | The current layout. The owner reviews after merge. |

## 16. Execution record

| Step | Result |
| --- | --- |
| Keys and spec | Frozen. Every key validates: `scripts/validate_assets.py` has 0 invalid, the `test_art_*` suite passes (1055 tests, 1 skipped), and `scripts/check_docs.py` passes. |
| Silhouette gate | Pending: a fresh top-tier agent compares the keys against the concept crop. |
| Execution | Pending (section 13). |
| Independent review | Pending. |
