# AREA-HEAT-01: the scorched corral kit of Floor 01 room 0, design spec

**Status:** design spec and five keys (`heat_partition_intact`, `_weakened`, `_broken`, `heat_scorch_wall_a`, `heat_scorch_floor_a`), frozen for the silhouette gate. Gate sheets are in [`docs/evidence/area-heat-01/gate/`](../evidence/area-heat-01/README.md). The rest of the kit, the room dressing and the Bevy wiring are not executed yet (section 13). Not owner-approved. Launch target: `floor01_scene --scene f01_late --f01-run` (room 0 `corral` is the first room of the run).

**Route:** difficulty score 2, a new environment kit on the approved ENV-01 language, with a state-keyed hero key (the partition): route **B + silhouette gate** ([routing by difficulty](../07-agents/pixel-art-pipeline.md#routing-by-difficulty)). The design step stops at this spec and the keys. Work order: AREA-HEAT-01 (pipeline order batch 3, [pixel-art pipeline](../07-agents/pixel-art-pipeline.md)).

**Authorities:**
- **Design intent:** `docs/01-design/floor-01-llama-magic-baptism.md` (M2: "a broad scorched masonry stock pen ... one obvious vulnerable timber partition"; level presentation: "scorched stone/brick corral with goblin-built rotten timber, **not** a lava biome/medieval ranch"), and `docs/01-design/enemy-gameplay-llama-rush.md` (the partition, DV-9, DV-10, KC17).
- **Geometry:** `assets/levels/floor01/f01_corral.room.json` and `f01_corral.encounter.json` (pinned hashes, read-only here), `src/f01_corral.rs`, and the composer `src/camera_lab/layout.rs`.
- **Mood only:** [atlas panel 03](../../assets/concept/floor-01/atlas-v1/03-bad-llamas-scorched-corridor.png). No pixel is cropped, traced or resampled from it.
- **Environment language:** the [ENV-01 kit](floor-01-catacomb-kit.md) contract and palette.
- **Precedent:** the [Broken Ring kit](area-rings-01-spec.md) (a `room_dressing` pass through `f01_boiler::compose_kit_hooked`).
- **Pipeline and tone:** [art direction](art-direction.md), [asset pipeline](asset-pipeline.md), [antihero tone](../01-design/antihero-tone.md).

## 1. Canon and tone constraints

- **What the room is:** an old masonry stock pen in the catacomb brick that goblins patched with salvaged timber. It has been burnt over and over by the llamas' spit. The heat is **evidence of past fights**: soot blooms licked up the brick, char and ash on the deck, small spit splashes. It is not a lava level:
  - no lava, no molten cracks, no glowing floor seams;
  - no fire emitter and no animated flame in the kit;
  - embers are at most a few `flame` 0..1 pixels, cold-dark orange, low in a decal.
- **Goblin-built, cheap and failing:** rotten planks, rope lashings, a snapped brace, nails, stone caps salvaged from the wall. Nothing is decorated.
- **Not a ranch:** no hay bales, saddles, horseshoes, wagon wheels or cattle brands. Straw only as a few litter pixels at the partition's foot.
- **No text, glyphs, logos or heraldry.** Engine ids are IP-neutral: `heat_*` pieces, `floor01_heat_*` atlases, `floor01_heat_kit.json`.

## 2. Scale and coordinates

Carl is 52 px, about 1.8 m. All coordinates are **room layout pixels** (top-left origin, y down), as composed for room 0 by `f01_corral::launch`. `W` is the walking line.

| Line | y | Note |
| --- | --- | --- |
| Walking line `W` | 580 | room `floor_y` 576 on the 16 n + 4 deck line |
| Floor back line `FB` | 560 | `W - 20`: props stand here (their bottom row is 559) |
| Deck surface | 560..584 | floor decals use only its back half, 561..572 |
| Step walkable top | 548 | the entry shelf and the dock (rect y 544, h 32); the step's drawn top band is 548..556, its front 557..579 |
| Bumper walkable top | 500 | rect (1856, 496, 32, 80) |
| Glob bands (llama spit in flight) | 532..538 (floor llamas), 500..506 (llama A on the dock) | `W - 48 .. W - 42`; no bright or ember pixel here |
| Actor head and torso band | 524..560 (floor), 492..528 (dock) | wall decals stay light here (rule 10.3) |
| 640 x 360 view | 292..652 | the HUD plates cover about y 292..352 at both corners |

| Room 0 `corral` (2304 wide) | Rect (x, y, w, h) or x | Note |
| --- | --- | --- |
| Ground bays (composer) | x 80, 416, 752, 1216, 1552 (304 wide) | each: `pier_l` 72 wide at x, `pier_r` 80 wide at x + 224, y 304..576 |
| Bay openings (parallax) | x 112..320, 448..656, 784..992, 1248..1456, 1584..1792; y 352..576 | **no wall decal** may overlap them |
| Plain brick at floor level | x 16..80, 384..416, 720..752, 1056..1216, 1520..1552, 1856..2304 | the only wall-decal ground |
| Ground torches (composer) | bay x + 24 and bay x + 264 | unchanged |
| Entry shelf | 1120, 544, 96, 32 | top 548 |
| Bumper | 1856, 496, 32, 80 | top 500 |
| Exit dock | 1888, 544, 416, 32 | top 548 |
| Checkpoint | 1048, 484, 48, 96 | |
| M3 gate grate (code-drawn) | x 2048..2064 | not in this kit (HEAT-OQ1) |
| Exit | 2232, 484, 64, 96 | goal x 2256 |
| Goblins (M0) | x 336, 416 | |
| Llama B (pen) | x 1488, leash 1248..1830 | stops at 1428 while the partition stands |
| Llama A (dock) | x 1968 | |
| Partition solid (CORRAL-D, not built) | 1392, 532, 16, 48 | KC17; the art anchor is (1380, 532) |

## 3. Room layout (zones, left to right)

| Zone | x | Heat dressing density |
| --- | --- | --- |
| M0, goblins and Donut's first casts | 0..448 | light: one ash drift, one wall spit trace |
| M1, one burning throat | 448..1056 | medium: one floor scorch, one small wall lick, ash and a spit trace |
| M2, the pen (signature) | 1056..1856 | heaviest: the hero wall scorch at the pen's entry, a second wall lick, four floor scorches, spit traces, two pen rails, timber debris, and the partition (CORRAL-D) |
| Dock, llama A | 1856..2048 | light: the bumper, the dock fronts, one wall bloom behind the dock |
| M3, the threshold | 2048..2304 | none beyond the dock fronts (the refuge threshold reads cooler) |

Coverage caps, measured over every 640 px window of the room: heat wall decals cover at most **18 %** of the visible plain brick between y 352 and 560, and floor decals at most **25 %** of the deck's back half (y 561..572).

## 4. Mapping onto the room solids (presentation only)

- **Mechanism (`HEAT-AD2`):** the kit is a **dressing pass** on the Broken Ring precedent. `floor01_heat_kit.json` is an `env-kit-v2` contract that extends the opening kit and carries one `room_dressing` block, `corral` (`remove` [], `place`, `parallax` {}). `f01_corral::launch` builds the kit through `f01_boiler::compose_kit_hooked` with the room's own steps, the heat contract and the dressing hook. No room or encounter JSON changes.
- **Never a collider.** No piece adds, moves or removes a solid. These stay byte-identical with and without the dressing: solids (the ceiling slab, the steps, the walls), spawn, exit, checkpoint, views, walking line, blink barriers, the pinned room and sidecar hashes, and the `COMBAT_STATE` hash on every tick.
- **Step fronts** are `structure` pieces laid exactly on the step rects at y 548 (32 rows: the top band and the front). They replace the composer's plain step look and do not change the collision tiles.
- **The partition is not placed by the dressing (`HEAT-AD1`).** Its three frames are drawn, exported and named in a `state_groups.heat_partition` entry. CORRAL-D's presenter draws them at (1380, 532) from the prop's state.

## 5. Piece list

`K` = frozen key drawn with this spec. Every other row is executed to this spec. Layers are the kit's: `backwall_decal` 3, `structure` 5, `prop` 6.

| Piece | Atlas | Size | Layer | Placed at (x, y) | Design |
| --- | --- | --- | --- | --- | --- |
| `heat_partition_intact` **K** | `floor01_heat_partition` | 40 x 48 | prop (CORRAL-D) | (1380, 532), not in the dressing | Section 6. |
| `heat_partition_weakened` **K** | `floor01_heat_partition` | 40 x 48 | prop (CORRAL-D) | same | Section 6. |
| `heat_partition_broken` **K** | `floor01_heat_partition` | 40 x 48 | prop (CORRAL-D) | same | Section 6. |
| `heat_scorch_wall_a` **K** | `floor01_heat_wall` | 64 x 96 | backwall_decal | (1056, 464) | Section 11.2. The pen's entry: the biggest soot bloom of the room. |
| `heat_scorch_wall_b` | `floor01_heat_wall` | 48 x 64 | backwall_decal | (1912, 484), bottom row y 547 (just above the dock's top band) | `heat_scorch_wall_a`'s method (section 11.2) with its own parameters: base ellipse centre (23.5, 66), radii 19 x 30; four streaks `(8, 0.60, 3)`, `(18, 0.90, 4)`, `(28, 1.00, 4)`, `(39, 0.65, 3)` (x, strength, width); burn-off from row 44 (red weight `clamp((y - 44) / 10)`), embers **none** (the dock is llama A's glob height). **No brick index below 3 anywhere** (it stands behind the dock's head band). Two mortar 1 crazing hairlines. Seed key `"wall_b"`. |
| `heat_scorch_wall_c` | `floor01_heat_wall` | 32 x 40 | backwall_decal | (720, 520), (1520, 520), bottom row y 559 | A small low lick: base ellipse centre (15.5, 44), radii 13 x 22; three streaks `(8, 0.55, 3)`, `(16, 1.0, 3)`, `(24, 0.7, 3)`; no brick index below 3; burn-off from row 30; embers: two `flame` 0 pixels in rows 36..39 at the strongest streak. Seed key `"wall_c"`. |
| `heat_trace_wall_a` | `floor01_heat_wall` | 16 x 16 | backwall_decal | (392, 544), bottom row y 559 | A spit splash on the brick: a ragged dark star 9..11 px wide (brick 3 core of about 12 px, brick 4 rim, three 1 px drips down 2..3 px), centred at (8, 10); one `flame` 0 pixel and one `flame` 1 pixel at rows 11..13. |
| `heat_scorch_floor_a` **K** | `floor01_heat_structure` | 72 x 12 | structure | (1044, 561), (1456, 561) | Section 11.3. |
| `heat_scorch_floor_b` | `floor01_heat_structure` | 48 x 10 | structure | (560, 562), (1264, 562), (1680, 562) | `heat_scorch_floor_a`'s method with an ellipse of 22 x 4.2 centred (23.5, 5), `floor` 2/3/4, five ash flecks (`iron` 5/6), no slab joint. Seed key `"floor_b"`. |
| `heat_ash_floor_a` | `floor01_heat_structure` | 32 x 6 | structure | (288, 563) | A drift of ash on the deck: `iron` 5 and 6 flecks (1..2 px each, at most 30 % of the pixels, never a horizontal run over 3 px) over a sparse `floor` 4 smudge (at most 20 %), densest at the centre, fading to single flecks at both ends. |
| `heat_ash_floor_b` | `floor01_heat_structure` | 24 x 6 | structure | (880, 564), (1792, 564) | The same rule, its own seed, densest left of centre. |
| `heat_trace_spit_a` | `floor01_heat_structure` | 16 x 6 | structure | (968, 564), (1600, 564) | A spit splat on the deck: a `floor` 2 blot about 9 x 3 with `floor` 3 spatter dots out to the piece edge, and **at most 2** `flame` 0..1 pixels in its core. |
| `heat_trace_spit_b` | `floor01_heat_structure` | 12 x 6 | structure | (1336, 565) | A smaller splat (about 6 x 2), one `flame` 0 pixel. |
| `heat_shelf_mid_a`, `_b` | `floor01_heat_structure` | 16 x 32 | structure | entry shelf: x 1128..1192 step 16; dock: x 1888..2288 step 16; y 548 | Section 11.4. Sequence: entry shelf `a b a a b`; dock index i (0..25) is `b` for i in {1, 4, 9, 12, 17, 22}, `a` otherwise. |
| `heat_shelf_end_l`, `_r` | `floor01_heat_structure` | 8 x 32 | structure | (1120, 548), (1208, 548) | Section 11.4. The dock has no ends (the bumper covers its left end, the room wall its right). |
| `heat_bumper` | `floor01_heat_structure` | 32 x 80 | structure | (1856, 500) | Section 11.5. |
| `heat_pen_rail_a` | `floor01_heat_props` | 64 x 28 | prop | (1240, 532), (1720, 532), bottom row y 559 | Section 11.6. |
| `heat_pen_rail_b` | `floor01_heat_props` | 64 x 28 | prop | (1560, 532) | Section 11.6, the broken variant. |
| `heat_timber_debris_a` | `floor01_heat_props` | 32 x 8 | prop | (1316, 552), (1648, 552), bottom row y 559 | Two charred plank offcuts and a splinter lying on the back line: planks 3 px tall (`wood` 3 top row, `wood` 2 body, `wood` 0 burnt ends of 2..4 px), one lying on the other at a 1 px step; a 1 px `stone` 7 fresh-break pixel on one end; at most 8 rows, no dark run longer than 4 px. |

**Keep clear:** no piece of any layer at x 1372..1428 (the partition, its debris and llama B's stop), and no floor decal overlapping a step's x range (1120..1216, 1856..2304).

## 6. The partition: states

The art is one 40 x 48 canvas per state, top-left at (1380, 532). The solid is columns 12..27 (layout x 1392..1407), rows 0..47 (y 532..579). Columns 0..11 and 28..39 are the margin for rope tails, straw and debris only.

| State | When (CORRAL-D) | Drawing |
| --- | --- | --- |
| `intact` | start | Two square posts (columns 12..14 and 25..27; `wood` 5/4 lit left, a shaded right side) carrying five horizontal planks of 7 rows (rows 4..10, 12..18, 20..26, 28..34, 36..42) with 1 px `wood` 0 gaps, under a 4-row salvaged **stone cap** (rows 0..3; row 0 is the lit arris, `stone` 8). One diagonal brace, 3 px wide, from the bottom left (13, 42) to the top right (26, 5). Rope lashings (`stone` 6/4) at the cap. Rot and soot climb from row 26 down (clump noise; `wood` 0 rot pixels, two `lichen` 0 pixels). A 4-row sill (rows 44..47) with ash and straw litter in the margins (at most 2 rows tall). A short rope tail hangs in the left margin (column 11, rows 5..10). |
| `weakened` | after the first counted hit (hits 1 and 2) | The intact frame plus damage: plank 3 snapped at column 21 (dark depth behind, pale `stone` 7..9 splinter ends); plank 2 cracked diagonally with its right part sagging 1 px; the brace snapped (its lower part rows 25..42 stays, a top stub rows 5..9 with pale ends); the left lashing torn (a longer rope tail in the margin). **The cap is unchanged** (the top surface and its read do not move). |
| `broken` | on the 3rd hit or a rush contact (same tick) | The 16 px remnant: post stumps and plank 4 up to row 32 only, a flat pale broken top (row 32, `stone` 8/7) so the remnant top reads as standable; splinter spikes of at most 3 px at columns 13, 19, 23, 26 (heights 3, 2, 1, 3; dark-tipped); flat debris in both margins (at most 3 rows, `wood` 2..5 and `stone` 6..8, no dark run over 2 px). |

- Frames swap on the state change tick; no transition frame, no particles in this kit (the break's splinters and dust are VFX owned by CORRAL-D, presentation only).
- `HEAT-AD3`: the 16 px remnant is rows 32..47 of the canvas (y 564..579), so the drawn top equals the prop core's remnant height.

## 7. Palette

`floor01-catacomb-v2` only, with **no new hue and no new value** (`HEAT-AD4`).

| Use | Ramps |
| --- | --- |
| Partition planks, posts, brace, rails, bumper timber, debris | `wood` 0..5 |
| Stone caps, lit tops, splinter faces, rope | `stone` 4..9 |
| Wall soot and burn-off | `brick` 2..6 (index 2 only in `heat_scorch_wall_a` above row 60), `mortar` 1 crazing |
| Floor scorch | `floor` 0..4 |
| Ash | `iron` 5..6, `floor` 4 |
| Nails, bands, rivets | `iron` 1..5 |
| Embers | `flame` 0..1 only |
| Rot | `wood` 0, `lichen` 0 (at most 2 pixels per piece) |

Every atlas stays at or under 63 colours, 1-bit alpha. The keys use 19, 20, 15 (partition) and 8, 6 (scorch).

## 8. Lighting (presentation only)

No new light emitter and no change to the composer's torches or pools (`HEAT-AD5`). The baked key light on every piece comes from the upper left, like ENV-01: lit left arrises, shaded right sides. Embers are baked pixels, not lights, and they do not animate.

## 9. Collision and state

The dressing is read-only. The authoritative state is unchanged (section 4). The `heat_kit` tests assert equality with and without the dressing. A window capture's final `COMBAT_STATE` hash must equal the headless run's.

## 10. Readability rules

1. **Walkable tops are the brightest run within 64 px:** the partition cap (row 0, `stone` 8), the shelf and dock top arris (row 0 of every shelf piece, `stone` 8), the bumper cap (row 0, `stone` 8). Nothing else within 64 px of a walkable top uses `stone` 8 or above, or another index of equal or higher lightness, except the partition's pale break faces (splinter ends and debris chips: isolated pixels, at most 2 in a row; 8 in `weakened`, 2 in `broken`).
2. **Rush lanes stay clean:** floor decals sit only in the deck's back half (y 561..572), so no decal reads as the rush scrape at the hooves (y 576..584) or as a trap on the lane. No decal has a hard straight edge across the lane.
3. **Actors read over soot:** in the head and torso band of the floor an actor stands on (section 2), no wall decal pixel is darker than `brick` 3. Only `heat_scorch_wall_a` uses `brick` 2, above that band (rows 0..59).
4. **Spit reads over everything:** no ember or `flame` pixel and no index lighter than `wood` 4 or `stone` 5 in a glob band (532..538 for the floor, 500..506 above the dock). Embers stay in a decal's bottom 6 rows and at y 552 or lower; the dock bloom has none. Exempt: the pieces drawn on a solid that stops globs (the partition's cap at y 532, the bumper's cap at y 500), because a glob dies on their face and never flies in front of them.
5. **Wall decals only on plain brick:** never over a bay opening, pier or arch ring.
6. **The partition reads as a separate, breakable object:** warm brown timber against the cold blue opening behind it (hue carries the read, measured mean L* is close; section 12.3); its cap is the brightest line in its frame; the broken remnant is visibly lower and flat-topped.
7. **Props stand on the back line** (bottom row y 559) and never exceed 28 px above it outside the partition.

## 11. The keys

### 11.1 Partition keys (40 x 48, three states)

See section 6. Measured on the gate composite (Carl, Donut and llama B at gameplay scale, the pen's still at 1x): mean L* intact 25.6, weakened 25.0, broken 24.9 (the brick or opening behind: 25.1 / 25.1 / 35.0); cap row L* 63.9 intact and weakened, 65.1 broken top; deck p95 within 64 px L* 52.0; 0 pixels above the cap in intact and weakened.

### 11.2 Hero wall key: `heat_scorch_wall_a` (64 x 96)

| Part | Rule |
| --- | --- |
| Density field | A base ellipse `1 - ((x - 31.5) / 25)^2 - ((y - 97) / 40)^2` (a bloom rising from below the deck line) plus five **leaning streaks** `(x, strength, width)` = `(11, 0.60, 4)`, `(21, 0.86, 5)`, `(33, 1.00, 5)`, `(45, 0.72, 4)`, `(55, 0.48, 3)`, each leaning slightly right as it rises (heat drift). Density = `clamp(max * 1.05, 0, 0.92)`. |
| Holes | Clump noise (`0.65 * vnoise(cell 6) + 0.35 * vnoise(cell 3)`) cuts the field: soot is drawn where `d > 0.02 + 0.7 n`, so the bloom is holed and its edges break into tongues, never a solid box. |
| Values | `brick` 2 where `d > 0.72` and row < 60 (HEAD_ROW), `brick` 3 where `d > 0.5`, otherwise `brick` 4/5. Rows 60..95 never below `brick` 3. |
| Burn-off | From row 64: the brick nearest the floor is burnt red, weight `clamp((y - 64) / 14)`; `brick` 5/6 where `d > 0.45 + 0.55 n`. |
| Crazing | Three `mortar` 1 hairlines in the bloom. |
| Embers | Four pixels, `flame` 0..1, rows 92..95 at (28, 93), (33, 94), (36, 92), (25, 95). |

Measured: 16.0 % coverage of its rect; darkest brick index at rows 60 and below: 3. Placement: bottom row y 559, on plain brick x 1056..1119, between the pier at 976..1056 and the entry shelf.

### 11.3 Hero floor key: `heat_scorch_floor_a` (72 x 12)

An elongated char patch on the deck's back half: an ellipse whose edge is broken by clump noise (sampled at `(x, 3 y)`, so the edge is ragged along the plank grain), values `floor` 2/3/4 (darkest at the centre), `iron` 5/6 ash flecks, a `floor` 0 slab-joint pixel run at x 38. Coverage 39.9 % of its rect. Top row y 561.

### 11.4 Step fronts (executed)

`heat_shelf_mid_a` (16 x 32), rows top to bottom:
- row 0: `stone` 8 (the walkable arris), full width;
- rows 1..2: `stone` 6; rows 3..8: `stone` 5 with a sparse `stone` 4 grain (10 %);
- row 9: `mortar` 0 (the shadow under the slab lip);
- rows 10..31: two block courses (rows 10..19 and 21..31) with a `mortar` 1 bed joint at row 20 and a `mortar` 1 head joint at column 0 of the upper course and column 8 of the lower one (running bond across tiles); per course: top row `stone` 4, body `stone` 3, bottom row `stone` 2, a 1 px lit left arris (`stone` 4) right of each head joint;
- a transparent nick at (15, 31) (validator: one transparent pixel per piece).

`heat_shelf_mid_b`: `_a` plus **char**: a soot tongue rising from row 31 to row 14 over columns 3..11 (clump noise, `stone` 1..2, densest at the bottom), and one burnt-red `brick` 5 pixel pair at its foot.

`heat_shelf_end_l`, `_r` (8 x 32): `_a`'s rows, with the outer two columns one step darker (the return face), the outer pixel of row 0 `stone` 6 (a rounded corner), opaque elsewhere (a transparent corner would show the step tile under it), one transparent nick at the outer bottom pixel.

### 11.5 Bumper (executed): `heat_bumper` (32 x 80)

Goblin-stacked squared timbers seen end-on, capped with a salvaged slab:
- rows 0..3: the stone cap (`stone` 8 row 0 = the walkable top, `stone` 6, `stone` 5, `mortar` 0 shadow row);
- rows 4..79: a 2 x 5 stack of 16 x 16 end grains (the bottom one is 12 px; rows 68..79), each with a 1 px `wood` 0 outline, a `wood` 3 face, two `wood` 2 growth rings (squares of radius 3 and 6 around the block centre, corners rounded), one radial check crack of 3..5 px in `wood` 0, and a lit top-left arris in `wood` 4;
- two iron straps across the full width at rows 35..36 and 67..68 (`iron` 2 body, `iron` 5 rivet pixels at x 4, 12, 20, 28);
- char on the two lowest rows of timbers (rows 52..79): clump-noise darkening by two steps, floor `wood` 0, no ember;
- one transparent nick at (31, 79).

### 11.6 Pen rails (executed): `heat_pen_rail_a`, `_b` (64 x 28)

`_a`: two posts (columns 2..5 and 58..61, rows 0..27; `wood` 4 lit left column, `wood` 3, `wood` 2 right column, a `wood` 1 bottom 2 rows), two horizontal rails (rows 6..8 and 17..19: `wood` 4, `wood` 3, `wood` 1) spanning columns 0..63, the lower rail sagging 1 px between columns 20 and 44; rope lashings at the four crossings (2 x 3 px, `stone` 6/4); the right third of the lower rail charred (`wood` 0..1) with a 1 px gap at column 50. Rows 0..6 never lighter than `wood` 4 (glob band).

`_b`: the same posts; the upper rail snapped at column 40, its right part hanging down to row 16 at its free end (a 1 px pale `stone` 7 break face); the left third of the lower rail charred instead of the right.

## 12. Acceptance targets

| # | Target |
| --- | --- |
| A1 | **Silhouette gate:** a fresh top-tier agent compares the keys at 1x in the lit pen still with Carl, Donut, a goblin and a llama at gameplay scale and passes them ([`gate/`](../evidence/area-heat-01/README.md)). |
| A2 | **Byte-identical keys:** after the gate, the five keys' pixel grids keep the SHA-256 pinned in `tests/test_art_heat01.py` (section 15, `HEAT-AD8`); a gate edit re-pins once and is recorded here. |
| A3 | **Partition read:** the three states read at 1x as intact, damaged and broken in that order; the cap row is the brightest line of its frame in intact and weakened; the broken remnant's top is at y 564 and flat; no pixel above row 0 in intact and weakened, at most 9 splinter pixels above row 32 in broken. |
| A4 | **In `--scene f01_late --f01-run` at 640 x 360, 640 x 400 and 1920 x 1080:** room 0 reads as a scorched stone pen patched with goblin timber, not lava and not a ranch; the heat density rises from M0 to the pen and drops at the dock and M3. |
| A5 | **Native captures:** every still is captured twice, byte-identical. The final `COMBAT_STATE` hash of every window equals the headless run's. |
| A6 | **Byte-identical state:** `heat_kit` tests prove solids, steps, spawn, exit, checkpoint, views, walking line, blink barriers and the ceiling slab are byte-identical with and without the dressing. `ROOM_HASH`, `ENCOUNTER_HASH`, `MANIFEST_HASH`, `TABLE_HASH`, the goldens and the replays are unchanged. |
| A7 | **Readability:** section 10 rules 1..7 hold on the dressed layout (checked by a test on the contract and the pixel grids) and in the captures (the reviewer checks rules 1, 2 and 3 on the pen and dock stills). |
| A8 | **Density:** the coverage caps of section 3 hold for every 640 px window (measured by script). |
| A9 | **Placement:** every placement of section 5 is in the contract, nothing in x 1372..1428, no floor decal over a step's x range, no wall decal over a bay opening or pier. |
| A10 | **Export:** `scripts/env_kit.py validate --contract assets/source/terrain/floor01/floor01_heat_kit.json` passes; two export passes are byte-identical; `python3 scripts/validate_assets.py` and the art unittests pass; every atlas has at most 63 colours, 1-bit alpha and at least one transparent pixel per piece. |
| A11 | **Rights:** nothing traced or resampled from the concept panel; no text, emblem or likeness; ids IP-neutral. |

## 13. Execution list

1. Extend `scripts/author_heat01.py` with every non-key piece of section 5 (keys stay byte-identical), pack the four atlases `floor01_heat_partition`, `_wall`, `_structure`, `_props`, write their `decor_atlas` manifests (`prototype_fixture_unapproved`, provenance) and `floor01_heat_kit.json` (pieces, `state_groups.heat_partition`, `room_dressing.corral`).
2. `src/heat_kit.rs`: the heat contract path, its dressing and `compose_heat_kit`, reusing the ring dressing code with the contract path as a parameter (ring output byte-identical); tests of A6 and A9.
3. `f01_corral::launch`: build the kit through `compose_heat_kit` with the room's steps.
4. Evidence in `docs/evidence/area-heat-01/`: native stills of M0, M1, the pen (entry scorch, rails), the dock and M3 in three sizes; before/after pairs; a contact sheet at 1x and 4x; the capture lines; README and `SHA256SUMS`.

## 14. Files

| File | Content |
| --- | --- |
| `scripts/author_heat01.py` | the authoring record: the keys, then every piece, atlases, manifests, contract and the gate sheets |
| `assets/source/props/floor01/heat_partition.grid.json` + `.ase` | the three partition keys (one canvas, three frames) |
| `assets/source/props/floor01/heat_scorch.grid.json` + `.ase` | the wall and floor scorch keys (two pieces packed with an 8 px pad) |
| `assets/source/props/floor01/heat_{partition,wall,structure,props}.grid.json` + `.ase` | the four atlas sources (execution; the keys sit in them pixel for pixel) |
| `assets/manifests/floor01_heat_*.json` | the four `decor_atlas` manifests (execution) |
| `assets/source/terrain/floor01/floor01_heat_kit.json` | the `env-kit-v2` contract (execution) |
| `src/heat_kit.rs` | the pure dressing loader and composer (execution) |
| `docs/evidence/area-heat-01/` | `gate/` (this stage) and the in-scene evidence (execution) |

## 15. Adopted defaults and open questions

| ID | Adopted default | Reason |
| --- | --- | --- |
| HEAT-AD1 | All three partition states are drawn and exported now, but the dressing does **not** place the partition in room 0 until CORRAL-D lands. CORRAL-D's presenter swaps the frames at (1380, 532): `weakened` after the first counted hero hit (hits 1 and 2), `broken` on the third hit or a rush contact. | Drawn geometry must equal the collider: a drawn screen without its 16 x 48 solid would lie to the player (bolts and globs would pass through what looks like cover). |
| HEAT-AD2 | The kit is a contract dressing pass (`floor01_heat_kit.json`, `room_dressing.corral`) applied in `f01_corral::launch` through `compose_kit_hooked`. No room or encounter JSON edit. | The Broken Ring precedent; the pinned hashes stay. |
| HEAT-AD3 | The partition remnant is the canvas rows 32..47 (16 px, flat pale top). | The prop core's remnant height; the drawn top equals the remnant-lift rule's surface. |
| HEAT-AD4 | No new hue or value; embers are `flame` 0..1 only. | Not a lava biome (the design doc's presentation row). |
| HEAT-AD5 | No emitter and no animation in the kit. | The light budget and the torch layout stay; heat is history, not an active hazard. |
| HEAT-AD6 | Floor decals use only the deck's back half (y 561..572) and never a step's top band; the dock gets wall soot and charred fronts instead of floor decals. | The rush scrape and the hooves live at the front of the deck; on the dock, feet stand on the top band's first row, so any decal there would read under the feet. |
| HEAT-AD7 | The step fronts and the bumper are scorched dressed stone and stacked timber capped with a stone slab; every walkable top's first row is `stone` 8. | One rule for "you can stand here" across the partition, shelf, dock and bumper. |
| HEAT-AD8 | Key pins (pixel-grid SHA-256, `json.dumps(px)`): `heat_partition_intact` `316ba5fa894f1e9eff499ada194aa4e2443a778247f7b32e264c7947467e5d30`, `heat_partition_weakened` `102a72bf8f4e8feedf7194ba397319f25f2ff81afba470e2d4f253e9028e46eb`, `heat_partition_broken` `3e7f7445d2321cd39bef1f5b15368d949628bd934bbd8d93a5bd240f488f7007`, `heat_scorch_wall_a` `fd35e8198a759892c02f4c6f26551337b50f069fe0a85e723f34eea2d4303fd3`, `heat_scorch_floor_a` `30261053739064530cbb0adc4ce6d637b3888a51bda0720656a29ec9e4b2456d`. | A2. |
| HEAT-AD9 | The pen rails stand on the back line behind llama B's leash, not across it. | A rail across the lane would read as a collider. |
| HEAT-AD10 | The hero wall scorch sits at the pen's entry (1056, 464), not behind the partition. | Plain brick at floor level is scarce in the pen (the bays fill it); the entry spot greets the player on the shelf approach, and the partition stays against the cold opening for hue separation. |

| ID | Open question | Default until answered |
| --- | --- | --- |
| HEAT-OQ1 | Who draws the M3 gate grate (x 2048..2064, code-drawn greybox)? | Out of this kit; the grate stays code-drawn. |
| HEAT-OQ2 | Should the ceiling slab (y 336..352) get a soffit like the Ring's? | No soffit; the composer's brick shows. |
| HEAT-OQ3 | The partition can only be seen in-scene after CORRAL-D. | Gate composites only (`HEAT-AD1`). |
| HEAT-OQ4 | Soot on the bay piers? | None; piers are composer structure and stay clean in this kit. |
| HEAT-OQ5 | Game feel: is the pen's density enough "the corral fights back", or does the owner want heavier burn? | The caps of section 3; the owner reviews after merge. |
