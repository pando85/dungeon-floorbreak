# AREA-CAMP-01: the survivor camp and the quiet after the Juicer, design spec

**Status:** design spec, keys and execution (prototype art, silhouette gate passed with notes G1..G8, independent art review pending, not owner-approved). This order dresses the two greybox quiet rooms that REST-WIRE landed:
- `rest_camp` (REST-B, the survivors' camp, between `ring_split` and the Juicer);
- `rest_aftermath` (REST-C, after the Juicer, before `mastery`).

The five keys are drawn by `scripts/author_camp01.py`:
- the camp's hero key `camp_hearth`, the protected central light;
- its 4-frame flame loop `camp_fire`;
- the survivor figure language `camp_figure_sentry`;
- the aftermath's hero key `aft_mirror_wreck`;
- the 5-frame `aft_heavy_bag` (rest and four sway frames).

The gate sheets are in [`docs/evidence/area-camp-01/gate/`](../evidence/area-camp-01/README.md). The rest of the pieces, the two contracts, the dressing, the bag presenter and the Bevy wiring are the execution (section 13). Nothing here is owner-approved. Launch targets: `floor01_scene --scene f01_late --f01-room rest_camp` and `--f01-room rest_aftermath`. Rooms 9 and 11 of `--f01-run` launch the same rooms.

**Route:** difficulty score 2, so **B + silhouette gate** ([routing by difficulty](../07-agents/pixel-art-pipeline.md#routing-by-difficulty)). The reasons:
- two environment families (a human camp and a wrecked gym);
- a figure language that must read as human, anonymous and never heroic;
- one moving prop (the bag) that responds to the heroes, presentation only.

Most of the props are reuse. The design step stops at this spec and the keys. Work order: AREA-CAMP-01 (REST-AD15).

**Authorities:**
- **Design intent:** [quiet set pieces](../01-design/floor-01-quiet-setpieces-and-transitions.md):
  - section B (REST-B beats 1..5);
  - section C (C1..C3);
  - G.3, the geometry;
  - G.11, the implementation defaults: REST-IMP-8 (no platform, crates and bench are decor), REST-IMP-9 (the bag is an inert `hit_target` static {624, 516, 32, 64}) and REST-IMP-10 (`goal_x` 1480 and 1224);
  - REST-Q2: anonymous figures only;
  - REST-Q6: the bag's reaction.
- **Geometry:** `assets/levels/floor01/f01_rest_camp.room.json`, `f01_rest_aftermath.room.json` and their sidecars (pinned hashes, read-only here), `src/f01_rest.rs`, and the composer via `f01_boiler::compose_kit`.
- **Mood only:** the camp half of [atlas panel 08](../../assets/concept/floor-01/atlas-v1/08-circular-maze-survivor-camp.png) and the [juicer gym panorama](floor-01-art-atlas-v1.md) for the aftermath. No pixel is cropped, traced or resampled from a concept image.
- **Reuse:**
  - [AREA-RINGS-01](area-rings-01-spec.md) (#290): `floor01_ring_camp` (`ring_camp_a`, `ring_crates_b`, `ring_marker_a`) and the `ring_tiers_far` / `ring_tiers_mid` parallax;
  - [AREA-GYM-01](area-gym-01-spec.md) (#275): `gym_girder`, `gym_tyres`, `gym_bench`, `gym_plate_tree` and the mirror glass of `gym_mirror_bay`, which the wreck key is torn from.
- **Lessons of [AREA-FASTFOOD-01](area-fastfood-01-spec.md) (#305), review F1..F5:**
  - floor decals sit on `structure`, above the collision deck;
  - a filled bay shows no background through its piers;
  - no slivers;
  - no prop stands across a window.
- **Pipeline and tone:** [art direction](art-direction.md), [asset pipeline](asset-pipeline.md), [antihero tone](../01-design/antihero-tone.md).

## 1. Canon and tone constraints

- **The camp is people organising themselves against an impossible place** (REST-B thesis). It is tired, practical and warm. Repaired canvas, a pot on a tripod, a woodpile, a sentry who faces the wrong way, a map on a trestle, a toy built from salvage. Nobody poses. There is no misery staged for effect and no hero worship.
- **The aftermath is the silence after violence** (REST-C thesis). It shows the gym's consequences: a mirror torn off its wall, a bent bar, a rolled-off plate, glass on the floor. A bag still hangs and moves when you hit it. Nothing is burning, nothing threatens, nothing rewards.
- **Survivors are anonymous and faceless (REST-Q2).** Hoods, hunched shoulders, wrapped hands, dark coats. A hood opening is a flat dark void: no eyes, no mouth, no hair colour, no skin tone, no recognisable costume. The concept boards' named survivors (Imani, Elle McGibbons, Brandon, Chris Andrews) are not drawn. Odette is not placed.
- **No text, glyphs, logos or brands.** The map carries lines and blots, not words. The strength motifs are crude painted shapes (a kettlebell, a bar, a flexed arm), not slogans. Plates carry no numerals.
- **Engine ids are IP-neutral:** `camp_*`, `aft_*`, atlases `floor01_camp_*` and `floor01_aft_*`.

## 2. Scale and coordinates

- Layout px, y down, 16 px grid, as the composer. The floor deck's top is y 560 and the walking line is y 580. A prop that stands on the floor has its bottom row on y 559.
- The window view is 640 x 360. Its top is y 292, so the deck top is screen row 268 and the walking line screen row 288.
- Every piece is placed by its top-left corner (`anchor [0, 0]`), like the rings and gym props.
- Heroes at 1x: Carl is about 24 x 48, Donut about 40 x 32. A survivor is 20 px wide and at most 48 tall, so a standing figure is the same height as Carl and never larger.
- **Window rule (the #305 lesson):** inside an aperture (x in an opening, y < 544), a prop's opaque pixels stay at y ≥ 520. So a prop in a window is at most 40 px tall.
  - camp openings: x 184..335, 520..671, 856..1007, 1192..1343;
  - aftermath openings: x 216..367, 552..703, 888..1039. B2 (552..703) is bricked up in this order (section 4).

## 3. Room layout (zones, left to right)

**`rest_camp`** (width 1536, checkpoint {744, 484, 48, 96}, exit {1456, 484, 64, 96}):

| x | Zone | Beat |
| --- | --- | --- |
| 0..183 | entrance: `ring_marker_a` at 156 | 1, arrival: the shared marker (KR14) |
| 336..519 | the lean-to (`ring_camp_a`) and the sentry facing left, toward the entrance | 1, the sentry looks the wrong way |
| 520..707 | supplies: `ring_crates_b` low in window B2 | 2, supplies |
| 720..815 | **the hearth**, over the checkpoint | 4, the protected central light |
| 856..1007 | repair: a kneeling figure mending gear in window B3 | 2, gear repair |
| 1008..1191 | a seated figure with a salvage toy cart; the map trestle with a standing figure | 2, the childlike aid, the hand-drawn maps |
| 1344..1455 | strength motifs painted on the wall; chalk arrows on the floor (1320..1447) | 5, toward gym territory |

**`rest_aftermath`** (width 1280, checkpoint {1056, 484, 48, 96}, exit {1200, 484, 64, 96}):

| x | Zone | Beat |
| --- | --- | --- |
| 216..367 | low wreckage in window B1: tyres, a bent bench | C1, the egress |
| 368..527 | **the mirror wreck**, leaning on pier and wall | C1, the fight's consequence |
| 512..719 | B2 bricked up, five girders at y 440, **the heavy bag** at x 620..659 | C2, the bag swings once |
| 768..880 | chalk ticks at both ends of the 112 px blink lane | C2, the lane |
| 1120..1279 | circular grooves cut into the back wall around the exit door | C3, the arena entrance |

## 4. Mapping onto the room solids (presentation only)

Both rooms have no platform (REST-IMP-8). Every piece is non-collidable decor. The sim rects (hit target, checkpoints, exits) are unchanged. The dressing goes through `room_dressing.{room}` in a new kit contract, applied in `f01_boiler::compose_kit_hooked` like the refuge.

**`rest_camp`**, contract `floor01_camp_kit.json` (extends `floor01_rings_kit.json`):
- `remove_at`. These are the four torches and their pools that the lean-to and the hearth replace. The hearth's own fire is the light here.
  - `prop torch_wall (368, 448)`, `backwall_decal torch_pool_b (264, 364)`;
  - `prop torch_wall (704, 448)`, `backwall_decal torch_pool_a_m (600, 364)`;
  - `prop torch_wall (760, 432)`, `backwall_decal torch_pool_a_m (656, 348)`;
  - `prop torch_wall (800, 448)`, `backwall_decal torch_pool_a_m (696, 364)`.
  `grime_a (600, 376)` stays: a lived-in camp is not clean.
- `parallax`: `ring_tiers_far` / `ring_tiers_mid`. The Ring owns the distant view (beat 1).
- `place`: see section 5.

**`rest_aftermath`**, contract `floor01_aftermath_kit.json` (extends `floor01_gym_kit.json`):
- `remove_at`:
  - `structure arch_chain (560, 384)`: B2's catenary would hang across the bag;
  - `backwall_decal torch_pool_b_m (1112, 348)`: re-placed after the grooves, so the pool draws over them (section 5).
- `backwall_fill {512, 352, 208, 208}`. B2 is bricked up across both pier inner columns, so no background shows through the piers (FF-AD29). The bag hangs in front of brick, never in front of a window.
- `parallax`: empty (the gym room's own).
- `place`: see section 5.

## 5. Piece list

K = key (frozen, drawn here). E = executor (drawn in `scripts/author_camp01.py` as new functions, from this list). R = reuse (an existing piece, unchanged). Sizes are w x h; the position is the top-left placement.

**`rest_camp`**

| Piece | K/E/R | Size | Layer | Placement | Notes |
| --- | --- | --- | --- | --- | --- |
| `ring_tiers_far`, `ring_tiers_mid` | R | (rings) | parallax | dressing `parallax` | the Ring in the distance |
| `ring_marker_a` | R | 24 x 56 | prop | (156, 504) | pixel-identical lantern (G.3: "the same piece at its entrance"); the greybox x 232 sits in the entrance door's opening |
| `ring_camp_a` | R | 144 x 72 | prop | (352, 488) | the lean-to, crates and lantern |
| `camp_figure_sentry` | K | 20 x 48 | prop | (500, 512) | faces left; the greybox x 520 is the B2 window's edge |
| `ring_crates_b` | R | 56 x 28 | prop | (652, 532) | low in window B2 (28 ≤ 40); stands in for the greybox crate stack {704, 548, 64, 32}, which would cover the hearth |
| `camp_hearth` | K | 96 x 64 | prop | (720, 496) | over the checkpoint (744..791) |
| `camp_fire` | K | 20 x 26, 4 frames | prop | (758, 526) | animated; the light is attached here (section 8) |
| `camp_figure_repair` | E | 24 x 36 | prop | (872, 524) | kneeling, mending a pack with a shuttered tin lamp beside it (one 2 x 2 `flame` 3 slit, no emitter: gate note G5, CAMP-AD18); window B3, so ≤ 40 |
| `camp_figure_seated` | E | 20 x 30 | prop | (1024, 530) | sitting on a low crate, hood down, hands around a cup |
| `camp_toy_cart` | E | 18 x 12 | prop | (1048, 548) | a salvage toy: a tin-can body, bottle-cap wheels and a string. The childlike prop of beat 2 |
| `camp_map_table` | E | 44 x 30 | prop | (1112, 530) | a trestle; the map is pale `stone` with ink-dark lines, blots and pins. No words |
| `camp_figure_map` | E | 20 x 46 | prop | (1152, 514) | standing, one hand on the table (listed after the table, so the hand draws over its end; gate note G3, CAMP-AD16), leaning over the map |
| `camp_motifs` | E | 80 x 48 | backwall_decal | (1344, 384) | crude painted kettlebell, bar and flexed arm in `brick` 6..8 and chalk `stone` 8; above torch 1376's top (448) |
| `camp_chalk_tally` | E | 24 x 40 | backwall_decal | (1404, 488) | chalk tallies and an arrow on the wall beside the exit door (x 1432) |
| `camp_chalk_floor` | E | 128 x 12 | structure | (1320, 561) | chalk arrows on the floor's back half toward the exit; `structure`, so it draws above the deck (FF-AD26) |

**`rest_aftermath`**

| Piece | K/E/R | Size | Layer | Placement | Notes |
| --- | --- | --- | --- | --- | --- |
| `gym_girder` | R | 32 x 10 | structure | (684, 440) first, then (548, 440), (580, 440), (612, 440), (644, 440), (676, 440) | the beam the bag hangs from, across the bricked B2. The sixth girder at 684, drawn under the fifth, closes the 8 px gap to the right pier (gate note G1, CAMP-AD15) |
| `gym_tyres` | R | 34 x 18 | prop | (236, 542) | low in window B1 |
| `aft_bench_bent` | E | 48 x 20 | prop | (284, 540) | `gym_bench` with its seat folded at a 20° kink and one leg sheared; derived from the gym bench's pixels like the wreck panel; low in window B1 |
| `aft_mirror_wreck` | K | 160 x 72 | prop | (368, 488) | its tall leaning panel sits at x 422..490 over the B1 pier and the wall; low parts only elsewhere; the torch columns 400..415 and 496..511 are clear above y 504 |
| `aft_heavy_bag` | K | 40 x 122, 5 frames | prop | (620, 450) | hook at (640, 450) under the girder. The body matches the sim's hit rect {624, 516, 32, 64}. Drawn by the presenter (section 7) |
| `aft_sand_spill` | E | 40 x 8 | structure | (622, 562) | the sand the split seam has spilled; `structure` on the deck's back half |
| `aft_lane_chalk` | E | 116 x 12 | structure | (766, 561) | two chalk ticks at x 768 and 876..879 and a dashed chevron between them: the blink lane's ends |
| `aft_groove_rings` | E | 160 x 160 | backwall_decal | (1120, 400) | concentric arcs cut into the wall around the exit door, drawn with the rings kit's arc generator; dark `mortar` cuts with a 1 px `stone` 7 lit lip |
| `torch_pool_b_m` | R | (opening) | backwall_decal | (1112, 348) | re-placed after the grooves (section 4) |
| `gym_plate_tree` | R | 28 x 36 | prop | (1000, 524) | a spare plate tree past the lane's far end, stripped of its plates; window B3, ≤ 40 |

No new piece goes in an opening above y 520. No new piece crosses a torch rect (x torch..torch+15, y torch_y..torch_y+55).

## 6. Hero keys

**`camp_hearth` (96 x 64)**, from back to front:
- two canvas windbreak wings on salvaged poles. The left wing has a torn top corner and a strip hanging from it. The right wing carries a stitched square patch. Their inner faces take the fire's warm bounce (`gal_warm` 2..5);
- a lashed tripod;
- a dented pot on a short chain. The lid sits ajar and the underside is warm-lit;
- the ember bed with two crossed logs and ember pixels;
- a ring of seven fire stones, warm on their tops;
- a woodpile (end grain toward the viewer) and a sitting log whose cut face is lit.

The flame is the separate piece `camp_fire`, placed at hearth (38, 30). Apart from the reused rings lanterns, it is the only `flame` 4/5 in the room's lower half. The hearth's brightest pixel is luma 139, so it never outshines Carl's skin.

**`aft_mirror_wreck` (160 x 72):**
- the left panel of `gym_mirror_bay`, torn off its wall and leaning right against the pier (each row shifts right by its height above the foot / 6). It has a jagged broken top edge, a radial crack star and a missing wedge that shows the plywood backing with glue dabs. The glass's reflected bricks and torch glows become dark glass (`gal` 2/3), so the panel reads as glass, not as a second wall;
- a bent bar with an edge-on plate at its left end;
- a face-on plate (no numerals);
- a twisted length of aluminium frame, rubber skirting and a spill of shards with a few white and warm glints.

## 7. States and motion (presentation only)

**`camp_fire`:** a 4-frame loop with durations 110, 90, 130 and 100 ms. It is a contract animation like `torch_wall`, and placements may take `frame_offset`. Each frame's light flicker is section 8.

**`aft_heavy_bag`:** five frames:
- `rest`;
- `r1` and `r2`: the bottom shifted +2 and +4 px;
- `l1` and `l2`: the bottom shifted −2 and −4 px.

Each sway frame is the rest frame sheared about the hook: row y shifts by round(dx · y / 115). The sand trickle falls straight down from the split seam in every frame. **The sim stays inert** (REST-IMP-9 and REST-Q6 stand). The presenter only reads state that already exists:
- **Entry swing:** on room entry the bag plays the sway table once. This is section C's thesis, "a hanging punching bag swings once".
- **Hit sway:** on the rising edge of an overlap between the bag's hit rect {624, 516, 32, 64} and either of these, the table restarts:
  - Carl's active melee hit box (`combat::hit_box(&move.spec.shape, &attacker)`, the same call the scene uses for its strike sparks);
  - Donut's `donut_hitbox()`.
  The direction is sign(bag centre x 640 − attacker x). A left push mirrors `r`↔`l`.
- **Sway table** (one frame per presentation tick, 38 ticks, then `rest`):
  - ticks 0..3 `r1`, 4..9 `r2`, 10..13 `r1`;
  - 14..17 `rest`;
  - 18..25 `l1`;
  - 26..29 `rest`;
  - 30..33 `r1`;
  - 34..37 `rest`.
- **Rewind guard:** if the presentation tick goes backwards (a fixture restart or a checkpoint retry), the presenter resets to `rest` with no active swing. Then the entry swing replays only on a fresh room entry.
- No sound in this order (CAMP-Q5).

## 8. Lighting (presentation only)

- **Camp:** minus four torches (each a flame light and a floor pool, so minus 8 lights), plus the `camp_fire` lights:
  - `flame`: colour `#ffb060`, radius 240, intensity 0.40, glow radius 40, additive 0.08, flicker [1.0, 0.9, 1.08, 0.95];
  - `floor_pool`: project `walk_line`, dy −6, squash_y 0.32, colour `#ffa95e`, radius 150, intensity 0.34.

  The light origin is the flame's base centre (768, 548): piece-local (10, 22). The values are the torch's, a little warmer and stronger. This is the protected central light of beat 4. The net change is −6 lights.
- **Aftermath:** no light change. The gym's fluorescent accent is **not** placed in the niche (CAMP-AD9): the bricked niche has no ceiling to hang it from, and a lit tube would make the bag a set piece instead of an after-image.
- Light colours are not palette pixels; the pixels stay on `floor01-catacomb-v2`.

## 9. Collision and state

1. Solids, spawns, checkpoints, exits, views, the walking line and the hit target are identical with and without the dressing.
2. The bag presenter reads the attacker hit boxes and the room tick. It writes nothing back. The sim's `hit_target` stays inert: no solid, no knockback, no score, no event.
3. Nothing in the dressing is tall and bright in the lane: there are no platforms, so no prop may read as one. A prop's top run stays below luma 160 (the fire excepted), and props stand on the back line.

## 10. Readability rules

1. **Heroes first.** No camp or aftermath piece is brighter than the heroes in the actor band (y 532..580):
   - the survivor figures' mean luma is at most 45 (the sentry measures 30.7);
   - the warm rims are `gal_warm` 1..2 only.
2. **Exits read.** The camp's chalk arrows and the aftermath's grooves both point at the exit door. Neither covers the door (`guild_portal_closed`) or its frame.
3. **Lanes stay clear.** Nothing in front of the actors (no `foreground` piece). Nothing stands in the blink lane x 768..880: only flat floor chalk (strokes at most 3 px tall) on the deck's back half.
4. **Windows stay windows.** The window rule of section 2. No parallax shows in the bricked B2 bay (piers included).
5. **No slivers.** Every placement meets its neighbour or a wall edge, or leaves at least 4 px of gap. No 1..3 px strip of background or brick shows between two pieces meant to touch: the girder run, the niche fill and the panel against the pier.
6. **Apart from the reused rings lanterns (`ring_camp_a`, `ring_marker_a`, unchanged), the fire is the only `flame` 4/5 in the camp below y 500.**

## 11. The keys

`python3 scripts/author_camp01.py` writes the five grids and their `.ase` sources. `python3 scripts/author_camp01.py gate CAPS OUT [BOARD]` writes the gate sheets. Every pixel comes from rules: ramps, a Bayer threshold, a hash noise, and the gym mirror glass for the wreck panel. The SHA-256 pins of the twelve key frames are in `gate/measurements.txt`. The execution keeps them byte-identical. A key change is a design change.

- `assets/source/props/floor01/camp_hearth.grid.json`: 96 x 64.
- `assets/source/props/floor01/camp_fire.grid.json`: 20 x 26, 4 frames.
- `assets/source/props/floor01/camp_figure.grid.json`: 20 x 48.
- `assets/source/props/floor01/aft_wreck.grid.json`: 160 x 72.
- `assets/source/props/floor01/aft_heavy_bag.grid.json`: 40 x 122, 5 frames.

The gate stills are 640 x 360 window frames of the two rooms with the walk pilot, from a scratch probe build:
- the greybox rectangles are off;
- the reused pieces of section 5 are applied from a scratch contract, with a light-only stand-in at the fire's position;
- each tick is captured twice, with and without the actor sprites.

The keys are composed at 1x over the actor-free still. The actor pixels are re-pasted from the other still. The keys in the stills are unlit raw pixels. The probe is not part of this PR.

## 12. Acceptance targets

| Id | Target | How it is checked |
| --- | --- | --- |
| CAMP-A1 | Solids, spawns, checkpoints, exits, views, walking line and the hit target are identical with and without the dressing, in both rooms | `camp_kit` unit test on the composed layouts |
| CAMP-A2 | Room, sidecar, `MANIFEST_HASH` and TABLE hashes, the state hash and every golden unchanged | existing pins; the window `COMBAT_STATE` hash equals the headless hash at every captured tick of both rooms |
| CAMP-A3 | Every `remove_at` matches (zero unmatched) in both rooms | unit test: `dress_with` returns 0 |
| CAMP-A4 | Window rule: no prop's opaque pixel in an opening above y 520 | art unittest on the composed layouts (pixel model) |
| CAMP-A5 | No background pixel in the bricked B2 bay x 512..719, y 352..559 | unit test (the FF-AD29 test) |
| CAMP-A6 | Every placement in y 561..572 is on `structure` (draws above the deck) | unit test |
| CAMP-A7 | The worst 640 px view of each room stays at or under `MAX_LIGHTS` | test as `the_gym_lights_stay_inside_the_light_budget` |
| CAMP-A8 | Bag presenter: `rest` with no trigger; the entry swing plays the table once; a hit's rising edge restarts it with the right direction; holding the overlap does not retrigger; a tick going backwards resets to `rest` | headless frame-selection test over the walk fixture and a synthetic overlap sequence |
| CAMP-A9 | Key frames byte-identical to the pins in `gate/measurements.txt` | art unittest |
| CAMP-A10 | Palette: every piece on `floor01-catacomb-v2`, at most 63 colours per atlas, 1-bit alpha, at least one transparent pixel per piece | `validate_assets.py`, art unittests |
| CAMP-A11 | No text, glyph, logo, numeral or face; survivors anonymous | review |
| CAMP-A12 | Window stills at 640 x 360 show the marker, the sentry, the hearth and its fire, the repair, seated and map figures, the motifs and chalk, the wreck, the bag (rest and a sway frame), the lane chalk and the grooves legibly at 1x, with the heroes in front | owner review of the stills |

## 13. Execution list

1. The pieces marked E in section 5 go in `scripts/author_camp01.py` as new functions; the keys stay untouched. Then the atlases, the pixel-grid and `.ase` sources, the manifests and the exports through `scripts/export_assets.sh`.
2. The contracts `assets/source/terrain/floor01/floor01_camp_kit.json` (extends the rings kit) and `floor01_aftermath_kit.json` (extends the gym kit). Each holds:
   - its atlases and pieces;
   - `camp_fire`'s animation and lights;
   - a `room_dressing` block for its room (section 4 and 5);
   - a `state_groups` entry for the bag (presenter-drawn, like the refuge's door).
3. `src/camp_kit.rs`: the dressing load and apply through `f01_boiler::compose_kit_hooked`, and the pure bag frame selection. Tests CAMP-A1, A3, A5..A8.
4. Bevy:
   - `f01_rest::launch_with` composes the camp and the aftermath with their dressing when `dressed`;
   - the binary skips `spawn_rest_greybox` for a dressed room and spawns the bag presenter sprite in the aftermath;
   - `f01_rest::decor` stays as data.
5. Evidence at `docs/evidence/area-camp-01/`:
   - window hash equal to the headless hash;
   - byte-identical double captures;
   - contact sheets, scene stills of both rooms, and a bag sway strip from the window;
   - docs and the art unittest `tests/test_art_camp01.py`.

## 14. Files

| Path | Content |
| --- | --- |
| `docs/02-art/area-camp-01-spec.md` | this spec |
| `scripts/author_camp01.py` | the authoring record (keys; at execution every piece, atlases, manifests, contracts, dressing, gate, measurements) |
| `assets/source/props/floor01/camp_hearth`, `camp_fire`, `camp_figure`, `aft_wreck`, `aft_heavy_bag` (`.grid.json` + `.ase`) | the keys |
| `docs/evidence/area-camp-01/gate/` | gate sheets, the composed stills, `measurements.txt` with the key pins |
| *(execution)* `assets/manifests/floor01_camp_*.json`, `floor01_aft_*.json` | manifests (`decor_atlas`, `prototype_fixture_unapproved`) |
| *(execution)* `assets/source/terrain/floor01/floor01_camp_kit.json`, `floor01_aftermath_kit.json` | the contracts |
| *(execution)* `src/camp_kit.rs`, `src/bin/floor01_scene/camp_art.rs`, `tests/test_art_camp01.py` | the dressing, the bag presenter, the art unittest |

## 15. Adopted defaults and open questions

Adopted under the owner's go-with-all authorization (AGENTS.md, design defaults 2026-11-19); each is a value the owner may change in a follow-up PR.

| Id | Adopted default | Reason |
| --- | --- | --- |
| CAMP-AD1 | Route B + silhouette gate (score 2) | two families, a figure language and a reactive prop; most props are reuse |
| CAMP-AD2 | Two contracts: the camp extends the rings kit, the aftermath extends the gym kit (single-chain `extends`) | each room reuses one family; no contract carries both |
| CAMP-AD3 | Survivors are four anonymous, faceless figures (sentry, repair, seated, map), at most Carl's height and darker than the heroes | REST-Q2; the greybox's four markers |
| CAMP-AD4 | The hearth replaces the greybox lantern as the central light, on the solid stretch 672..855 over the checkpoint; four torches and their pools are removed | beat 4 "a protected central light"; a light that sits in a window would break the window rule |
| CAMP-AD5 | The marker stands at x 156, not 232; the sentry at 500, not 520; the crates are `ring_crates_b` at 652, not the greybox stack at 704; the map trestle and figure move to 1112..1177, out of window B4 | the greybox x values sit in the entrance door, on a window edge, in front of the hearth and in a window |
| CAMP-AD6 | The camp keeps `grime_a` and the Ring parallax | lived-in, and the Ring owns the distant view (beat 1) |
| CAMP-AD7 | REST-Q6 (presentation half): the bag swings once on entry and on each hit's rising edge, with the 38-tick table and a rewind guard; the sim stays inert | section C "swings once"; the "usable" half of REST-C-02 needs visible feedback, and a presentation-only response changes no hash. The combat half of REST-Q6 stays open |
| CAMP-AD8 | B2 of the aftermath is bricked up (`backwall_fill`) with five girders; its `arch_chain` is removed | the bag needs something to hang from, and must not hang across a window (#305 lesson) |
| CAMP-AD9 | No fluorescent tube in the niche | it would float with no ceiling, and it would make the bag a set piece |
| CAMP-AD10 | The wreck panel reuses the gym mirror glass, its reflections darkened to `gal` 2/3 | provenance (the same room family) and the reading: torn-off glass no longer reflects the lit room |
| CAMP-AD11 | The bag body is drawn over the sim's hit rect {624, 516, 32, 64}: at rest its cap and body cover x 626..654, y 510..569 | what you hit is what you see |
| CAMP-AD12 | No crate-repositioning animation and no idle animation of the figures in this order | beat 3 is an animation candidate; static figures are cheaper and safer at the first art pass |
| CAMP-AD13 | No audio | the bag's clang and the fire's crackle belong to a REST audio order |

| Id | Open question | Owner of the answer |
| --- | --- | --- |
| CAMP-Q1 | REST-Q2: which survivors appear, with names, lines or native sprites? Until then the four figures stay anonymous (CAMP-AD3) | owner and narrative review |
| CAMP-Q2 | Beat 3: should a survivor reposition the crates after the party passes (a bounded presentation-only animation of `ring_crates_b`)? | owner game-feel review |
| CAMP-Q3 | Should the figures breathe or shift (two-frame idles), and the sentry turn its head when the heroes pass? | owner game-feel review |
| CAMP-Q4 | REST-Q6 (combat half): should the bag do anything in the sim (a hit counter, a stronger gauntlet reaction)? CAMP-AD7 adds only the presentation sway | combat design |
| CAMP-Q5 | The bag's clang, the trailing weight clang of section C, the fire's crackle and the camp's quiet ambience | REST audio order |
| CAMP-Q6 | Should the aftermath niche show a fallen fluorescent tube on the floor instead of none (CAMP-AD9)? | owner art review |

**Horizon fit:** the kit changes no move, ability, unlock or gate. It dresses REST-B (the camp consolidates known moves) and REST-C (it shows the Juicer's grants without granting again) of the [progression horizon](../01-design/character-progression-horizon.md). The bag's sway is feedback for an already granted move, not a new power. The guide still fits; it is not edited.

## 16. Execution record

Executed by Claude Sonnet after the independent silhouette gate (verdict FREEZE-with-notes, notes G1..G8). Prototype art, independent art review pending, not owner-approved. Every pixel is drawn by `scripts/author_camp01.py` from rules; the twelve key frames are byte-identical to the gate's pins (`tests/test_art_camp01.py`).

**Pieces and atlases** (`floor01-catacomb-v2`, at most 63 colours per atlas, 1-bit alpha, at least one transparent pixel per piece; sources in `assets/source/props/floor01/`, manifests `assets/manifests/floor01_camp_*.json` and `floor01_aft_*.json`, status `prototype_fixture_unapproved`):

| Atlas | Source stem | Pieces | Colours |
| --- | --- | --- | --- |
| `floor01_camp_hearth` | `camp_hearth` | `camp_hearth` (K) | 25 |
| `floor01_camp_fire` | `camp_fire` | the 4 `camp_fire` frames (K), one canvas per frame, tag `burn` | 6 |
| `floor01_camp_figures` | `camp_figure` | `camp_figure_sentry` (K), `camp_figure_repair`, `camp_figure_seated`, `camp_figure_map`, `camp_toy_cart`, `camp_map_table` | 36 |
| `floor01_camp_wall` | `camp_wall` | `camp_motifs`, `camp_chalk_tally`, `camp_chalk_floor` | 7 |
| `floor01_aft_wreck` | `aft_wreck` | `aft_mirror_wreck` (K), `aft_bench_bent`, `aft_sand_spill`, `aft_lane_chalk` | 32 |
| `floor01_aft_bag` | `aft_heavy_bag` | the 5 bag frames (K), presenter-drawn | 19 |
| `floor01_aft_grooves` | `aft_grooves` | `aft_groove_rings` | 4 |

The twelve executor pieces (E) are `camp_figure_repair`, `camp_figure_seated`, `camp_toy_cart`, `camp_map_table`, `camp_figure_map`, `camp_motifs`, `camp_chalk_tally`, `camp_chalk_floor`, `aft_bench_bent` (the gym bench's own pixels, the right half dropped 20 degrees at a kink and one leg sheared), `aft_sand_spill`, `aft_lane_chalk` and `aft_groove_rings`. The four survivor figures have mean luma 30.7 (sentry), 30.9, 32.6 and 26.9 (at most 45). The contracts are `floor01_camp_kit.json` (extends the rings kit; `camp_fire` carries the animation and the two lights of section 8) and `floor01_aftermath_kit.json` (extends the gym kit; the bag is the `aft_heavy_bag` state group at (620, 450)). The rest of the execution is `src/camp_kit.rs`, `src/bin/floor01_scene/camp_art.rs`, the `f01_rest::launch_with` wiring and the scene's REST-WIRE branch (`spawn_rest_greybox` and its colour table are removed: no quiet room draws flat rectangles any more).

**Tests.** `src/camp_kit/tests.rs` (CAMP-A1 solids, spawns, checkpoints, exits, views, walking line, hashes and the sidecar's inert `hit_target` unchanged and the state hash equal on every tick; CAMP-A3 every `remove_at` matches; CAMP-A5 the bricked bay x 488..743, y 352..559; CAMP-A6; the window rule; torch clearance; CAMP-A7 lights; CAMP-A8 the bag selection; the gate notes G1 and G6; the `punch` pilot) and `tests/test_art_camp01.py` (CAMP-A9 and A10, the contracts, figure language, G1, G3, G4 and G5, the sway table equals the Rust one). **Evidence:** [`docs/evidence/area-camp-01/`](../evidence/area-camp-01/README.md): 16 window stills (two sizes), every capture twice and byte-identical, window hash equal to the headless hash at every tick, the lit figure measurements of gate note G2 (all four pass), contact sheets, the bag sway in the window, layout diagrams, `SHA256SUMS`.

**Adopted defaults of the execution** (the owner may change any in a follow-up PR):

| Id | Adopted default | Reason |
| --- | --- | --- |
| CAMP-AD14 | The grooves are drawn here as concentric 2 px cuts (radii 82, 94, 106, broken in places, a 1 px `stone` lip on the upper inner side), centred on the portal's mid-height (1232, 512), instead of with the rings kit's `Arc` generator | `Arc` draws a 24 px voussoir relief on a 148..208 px radius, which cannot fit a 160 px piece; the three cuts wrap round the portal's top and left |
| CAMP-AD15 | Gate note G1: a sixth `gym_girder` at (684, 440), listed before the one at (676, 440); the pixel model shows no brick between the run and either pier shadow column (a transparent top corner at each girder joint is the gym piece's own) | the run stopped 8 px short of the right pier (rule 10.5) |
| CAMP-AD16 | Gate note G3: `camp_figure_map` at (1152, 514), after the table, so its wrapped hand draws over the table's end | a 2 px bbox gap broke "one hand on the table" |
| CAMP-AD17 | Gate note G4: `camp_toy_cart` at (1048, 548), 7 empty columns from the seated figure | 4..8 px, measured on pixels |
| CAMP-AD18 | Gate note G5: the repair figure's lamp is a shuttered `iron` 2..4 tin with one 2 x 2 `flame` 3 slit, no emitter | rule 10.6 keeps the fire the only `flame` 4/5 below y 500 |
| CAMP-AD19 | Gate note G6: the bag presenter's entry swing starts on the first tick it observes, not at tick 0; same tick, same frame; a rewind resets to `rest` and treats a held overlap as held; the overlap is Carl's OR Donut's, the direction uses Carl's x when his overlap rose; a tie follows the attacker's facing; Donut's bolts do not trigger it | executable without design calls; no sim or hashed state |
| CAMP-AD20 | The fire is its own atlas (`floor01_camp_fire`, one canvas per frame, tag `burn`) and the hearth atlas holds the hearth only | the contract's animation needs one exported sheet frame per animation frame (the `torch_wall` pattern) |
| CAMP-AD21 | The floor chalk, the sand spill and the lane chalk are `structure` pieces | they draw above the collision deck (FF-AD26, review F1 of the refuge) |
| CAMP-AD22 | A new evidence pilot `--rest-pilot punch` (aftermath only) | the entry swing runs while the bag is at the view's edge, so a hit fixture is the only way to show the sway in a window still |
| CAMP-AD23 | The window-rule and torch-clearance tests cover the dressing's own `prop` and `structure` placements; a wall decal sits behind a torch, and the composer's `torch_pool_*` decals are not floor decals | the reused lean-to and marker are the Ring's own pieces; the decals are wall washes |
| CAMP-AD24 | The drawn view lefts of the design stage (237, 483, 607) are 25 px short; the measured ones are 262, 508 and 632 | landmark match against the lit model, confirmed by the sentry's spear |

Gate recommendations adopted for the open questions (CAMP-Q1..6): Q1 the four figures stay anonymous and faceless until a separate narrative and art order after the REST-Q2 review; Q2 no crate animation in this order (a bounded, presentation-only drag of `ring_crates_b` may follow the owner's static-camp review); Q3 no idles (if wanted later: a 1 px two-frame breathing idle for the seated and map figures only; no sentry head turn); Q4 the sim stays inert on Floor 01 and the bag's presentation sway satisfies the "usable" half of REST-C-02 (a stronger reaction needs wider r3/l3 frames, a key change); Q5 no audio; Q6 no fallen tube (CAMP-AD9 stands).
