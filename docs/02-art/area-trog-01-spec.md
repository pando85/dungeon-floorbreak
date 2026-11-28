# AREA-TROG-01: the troglodyte territory and the Test of Strength, design spec

**Status:** design spec and frozen keys; the independent silhouette gate froze all sixteen keys (FREEZE-with-notes G1..G8, section 16). **Stage 1 (`trog_run`) is executed and merged (#311); stage 2 (`trog_strength`) is executed on the static bench TROG-B landed (#312, TROG-AD46)** (kit, dressing, wiring, evidence; prototype art, independent art review pending, not owner-approved). The bench was first a statically placed key; **TROGB-AD1 restored the partition**, so the bench stack is now drawn by a presenter with its break burst (section 7, TROG-ART-AD-S8) and the stage-2 amendment below is history. This order dresses the two troglodyte rooms of the [TROG plan](../01-design/floor-01-troglodyte-integration.md):
- `trog_run` (plan 4.1, #191 T0..T3: the approach, the low hunter, the fist-shaped hole, the bell and the tongue), landed as a greybox by TROG-A;
- `trog_strength` (plan 4.2, #191 T4: the Test of Strength and its bench stack), landed by TROG-B.

The eight key sources are drawn by `scripts/author_trogarea01.py` (16 frozen frames):
- the run's hero key `trog_fist_hole`, the territory's signature;
- `trog_bell`, the bell of T3;
- the wall marks `trog_knuckle_marks` and `trog_slam_crack`, and the floor decal `trog_impact_crater`;
- the strength room's `trog_threshold` (stone turning to gym tile) and `trog_mirror_shards` (the plan's hanging mirror fragments);
- the bench stack `trog_bench_{intact,weakened,broken}` (only `intact` is placed: TROG-ART-AD-S2);
- the 6-frame debris burst `trog_bench_debris_0..5` (wired by TROGB-AD1, TROG-Q14 resolved).

The gate sheets are in [`docs/evidence/area-trog-01/gate/`](../evidence/area-trog-01/README.md). The executor pieces, the contract, the dressing, the two presenters and the Bevy wiring are the execution (section 13). It runs in two stages, `trog_run` once TROG-A is on main and `trog_strength` once TROG-B is. Nothing here is owner-approved. Launch targets: `floor01_scene --scene f01_late --f01-room trog_run` and `--f01-room trog_strength`.

**Route:** difficulty score 3, so **B + silhouette gate** ([routing by difficulty](../07-agents/pixel-art-pipeline.md#routing-by-difficulty)). The reasons:
- a new family (a territory marked by brute force) and a gym arena graft;
- pieces above 40 px (the hole, the bell and the 80 px bench);
- a gameplay prop with three states and a presentation-only debris burst.

Most of the strength room is reuse of the gym kit. The design step stops at this spec and the keys. Work order: AREA-TROG-01 ([work order](../07-agents/floor-01-troglodyte-workorder.md), plan section 16).

**Authorities:**
- **Design intent:** the TROG plan:
  - 4.1 and 4.2, the geometry and what each pocket teaches;
  - 5.4, the bench prop (target {640, 500, 32, 80}, remnant {640, 564, 32, 16}, `break_hits` 3) and its static fallback (TROG-AD12);
  - section 16, the art needs, the reuse and the constraints ("every capture at 1x shows the troglodytes as the brightest, highest-contrast shapes"; fluorescent light only in `trog_strength`);
  - TROG-Q7: no audio path.
- **Geometry:** `assets/levels/floor01/f01_trog_run.room.json`, `f01_trog_strength.room.json` and their sidecars (pinned hashes, read-only here), `src/f01_trog.rs`, and the composer via `f01_boiler::compose_kit`. TROG-A composes the room with **no** authored step art: the two sidecar statics (the low refuge and the bell step) are pushed in as plain solids, and the composer draws nothing for them (section 5, rows `trog_step_*`).
- **Sprites (read-only):** the troglodytes TROG-01 Pygmy, TROG-02 Basher, TROG-03 Virtuoso. Their pixels are not touched by this order.
- **Mood only:** the territory and gym panels of the [floor art atlas](floor-01-art-atlas-v1.md). No pixel is cropped, traced or resampled from a concept image.
- **Reuse:**
  - ENV-01 [catacomb kit](floor-01-catacomb-kit.md): walls, piers, arches, torches;
  - [AREA-GYM-01](area-gym-01-spec.md) (#275): `gym_fluoro_cold`, `gym_dumbbell_rack`, `gym_plate_tree`, `gym_mat_a/b`, `gym_girder`, and the pixels of `gym_bench` and `gym_mirror_bay`, which the bench stack and the shards are built from;
  - [AREA-HEAT-01](area-heat-01-spec.md): the shelf body the step and shelf faces are derived from, and the partition presenter pattern (`heat_kit::partition_piece`, the scene's `corral_art`).
- **Lessons of [AREA-FASTFOOD-01](area-fastfood-01-spec.md) and [AREA-CAMP-01](area-camp-01-spec.md):**
  - floor decals sit on `structure`, above the collision deck;
  - a bricked bay covers both pier inner columns, so no background shows through;
  - no chain across a window or a decal;
  - no slivers;
  - no decal hidden behind structure (a measured visibility, section 10);
  - unlit stills hide the scene's light falloff, so the read targets are measured **lit** (section 10).
- **Pipeline and tone:** [art direction](art-direction.md), [asset pipeline](asset-pipeline.md), [antihero tone](../01-design/antihero-tone.md).

## 1. Canon and tone constraints

- **The run is territory, not a dungeon set.** Brute force has rewritten an ordinary catacomb: a wall punched through in the shape of a fist, knuckle dents in the brick, a slam crack, a crater in the floor. Nobody decorated it. The marks are the evidence of strength the troglodytes are proud of, shown plainly, with no trophies, skulls or gore.
- **The bell is a found object.** An old iron-and-brass bell on a wall bracket with a frayed pull rope. It is dented and dull. It is decor only (TROG-Q7: no sound in this order).
- **The Test of Strength is a gym grafted onto stone.** The catacomb floor turns into gym tile at a worn threshold; mats, a dumbbell rack, a plate tree and one cold fluorescent tube sit in a bricked bay. The gym's mirror is smashed: only shards hang in its frame over dark plywood. It is a cheap, worn test, not a temple.
- **The bench stack is a barricade built by brutes.** A gym bench upended on its end, weight plates stacked and lashed against it with one ratchet strap. It is heavy and crude, and it looks breakable.
- **No text, glyphs, logos, numerals or faces.** Plates carry no numerals; the mirror reflects nothing recognisable.
- **Engine ids are IP-neutral:** `trog_*` names the room family; the atlases are `floor01_trog_*`.

## 2. Scale and coordinates

- Layout px, y down, 16 px grid, as the composer. The floor deck's top is y 560 and the walking line is y 580. A prop that stands on the floor has its bottom row on y 559; a `structure` floor strip covers rows 561..579.
- The window view is 640 x 360. Its top is y 292, so the deck top is screen row 268 and the walking line screen row 288.
- The lane ceiling slab (DP-Q45) is the solid y 336..351 across each room. It has no art in this order (TROG-ART-Q1).
- Every piece is placed by its top-left corner (`anchor [0, 0]`).
- Actors at 1x: Carl is 36 x 56 (pivot (19, 55)); the Pygmy 80 x 56, the Basher 112 x 88, the Virtuoso 128 x 88 (their frame-0 canvases, feet on y 579).
- **Bays** (pier_l x .. pier_r x + 15) and **openings** (pier_l + 40 .. pier_l + 191):
  - `trog_run` (width 2304): piers at 144, 480, 816, 1152, 1488, 1984; openings 184..335, 520..671, 856..1007, 1192..1343, 1528..1679, 2024..2175;
  - `trog_strength` (width 1024): piers at 16, 352, 688; openings 56..207, 392..543, 728..879.
- **Window rule (the #305 lesson):** inside an opening that is not bricked up (x in an opening, y < 544), a prop's opaque pixels stay at y ≥ 520.

## 3. Room layout (zones, left to right)

**`trog_run`** (plan 4.1; Pygmy at 640, Basher at 1376, Virtuoso at 2144):

| x | Zone | Beat |
| --- | --- | --- |
| 176..383 | bay 1 bricked up: the **slam crack** high on the brick and the **knuckle marks** low, beside the spawn | T0: the area's identity, before any enemy can see the spawn |
| 352..415 | the **low refuge** face (32 px step art) | T1: a breather |
| 560..631 | the **impact crater** on the floor, under the Pygmy's leash | T1: what a landing does to stone |
| 1184..1391 | bay 4 bricked up: the **fist-shaped hole** at 1312..1375, right behind the Basher's spawn | T2: the Basher emerges in front of what it did |
| 1800..1839 | the **bell** on its bracket, left of the step | T3: the bell and the tongue |
| 1840..1903 | the **bell step** face (32 px step art) | T3: line control |

**`trog_strength`** (plan 4.2; Virtuoso at 496, Basher at 776):

| x | Zone | Beat |
| --- | --- | --- |
| 0..224 | protected spawn; plain catacomb | arrival |
| 288..351 | the **retreat shelf** face (48 px shelf art) | the Virtuoso's tongue stops at it |
| 640..671 | the **bench stack** (the intact key, placed statically on the sidecar's 80 px solid) | the room's question |
| 672..751 | the **threshold**: the stone floor turns into gym tile under the bench's far side | crossing into the test |
| 720..927 | bay 3 bricked up: girders, the cold tube, the **mirror shards**, the rack and the plate tree; mats on the floor | the gym grafted onto stone |

## 4. Mapping onto the room solids (presentation only)

Neither room gains a solid. Every dressing piece is non-collidable decor; the low refuge, the bell step, the shelf and the bench keep the sidecar's collision. The dressing goes through `room_dressing.{room}` in one new contract `floor01_trog_kit.json` (extends `floor01_gym_kit.json`), applied with `ring_kit::dress_with` in `f01_boiler::compose_kit_hooked`, like the camp and the aftermath.

**`trog_run`**:
- `remove_at`:
  - `structure arch_chain (224, 384)` and `structure arch_chain (1232, 384)`: the catenaries of the two bricked bays would hang across the slam crack and the fist hole;
  - `structure arch_threshold (208, 544)` and `structure arch_threshold (1216, 544)`: the composer's blue window sills of the two bricked bays (cold skirting under warm brick, and a 4 px lip on the refuge face); the `backwall_fill` brick shows down to the deck (TROG-ART-AD20, review R2);
  - `prop torch_wall (2240, 432)` and `backwall_decal torch_pool_a (2136, 348)`: the composer places two torches at x 2240 (y 432 and 448); the upper one and its pool go (TROG-ART-AD7).
- `backwall_fill`: `{176, 352, 208, 208}` and `{1184, 352, 208, 208}`. Bays 1 and 4 are bricked across both pier inner columns, so no background shows through the piers (FF-AD29).
- `parallax`: empty.
- `place`: section 5.

**`trog_strength`**:
- `remove_at` (ten entries, TROG-ART-AD-S2, AD-S3, AD-S7):
  - `structure arch_chain (768, 384)`: bay 3's catenary would hang across the mirror shards;
  - `structure arch_threshold (752, 544)`: the composer's blue window sill of the bricked bay (the stage-1 review's lesson, TROG-ART-AD20);
  - `prop torch_wall (48, 432)` with `backwall_decal torch_pool_a_m (-56, 348)`, and `prop torch_wall (960, 432)` with `backwall_decal torch_pool_b (856, 348)`: the doubled torches at the room's two ends (TROG-ART-AD7);
  - `prop torch_wall (664, 432)` with `backwall_decal torch_pool_a (560, 348)`: the middle torch of the three round pier 688, whose flame column (x 664..679) stood over the bench (gate note G1);
  - `prop torch_wall (328, 432)` with `backwall_decal torch_pool_b (224, 348)`: the middle torch of the three round pier 352, whose flame column (x 328..343) stood over the retreat shelf (x 288..351), the same G1 pattern (TROG-ART-AD-S7).
- `backwall_fill`: `{720, 352, 208, 208}` (bay 3).
- `parallax`: empty.
- `place`: section 5.
- **The bench is placed**, as one static `prop`: `trog_bench_intact` at (640, 500), exactly the sidecar bench static {640, 500, 32, 80} (TROG-AD46: an 80 px static solid, no partition, no `state_groups` entry). The weakened, broken and debris keys are reserved for TROG-Q14 (the bench break) and are not packed or wired.

## 5. Piece list

K = key (frozen, drawn here). E = executor (drawn in `scripts/author_trogarea01.py` as new functions, from this list). R = reuse (an existing piece, unchanged). Sizes are w x h; the position is the top-left placement.

**`trog_run`**

| Piece | K/E/R | Size | Layer | Placement | Notes |
| --- | --- | --- | --- | --- | --- |
| `trog_slam_crack` | K | 48 x 48 | backwall_decal | (296, 400) | a radial crack with a crushed centre, high on bricked bay 1; the far "slam" beat of T0 |
| `trog_knuckle_marks` | K | 96 x 40 | backwall_decal | (272, 488) | rows of four 3 x 3 knuckle dents (fresh and old) and three drag scrapes, low on bay 1 at fist height; clear of the door (216) and the refuge |
| `trog_step_end_l`, `trog_step_mid` x 3, `trog_step_end_r` | E | 8 x 32, 16 x 32, 8 x 32 | structure | (352, 548), mids at 360, 376, 392, end_r at 408 | the low refuge's face (sidecar static x 352..415, y 548..579); see "step and shelf faces" below |
| `trog_impact_crater` | K | 72 x 20 | structure | (560, 560) | a shallow bowl, radial cracks and a dust fan on the deck's back half; `structure`, so it draws above the deck |
| `trog_fist_hole` | K | 64 x 80 | backwall_decal | (1312, 440) | the hero key: the void is world y 446..487, above the Basher's sprite top at spawn (493), so the Basher stands in front of the breach, never inside it |
| `trog_bell` | K | 40 x 64 | prop | (1800, 420) | bracket on the wall, the bell, a frayed rope hanging to y 483; on solid wall right of pier_r 1712; never crosses a torch rect |
| `trog_step_end_l`, `trog_step_mid` x 3, `trog_step_end_r` | E | (as above) | structure | (1840, 548), mids at 1848, 1864, 1880, end_r at 1896 | the bell step's face (sidecar static x 1840..1903) |

The 64 px platform at 1920..1983 is drawn by the composer's own ledge pieces (no change).

**`trog_strength`**

| Piece | K/E/R | Size | Layer | Placement | Notes |
| --- | --- | --- | --- | --- | --- |
| `trog_shelf48_end_l`, `trog_shelf48_mid` x 3, `trog_shelf48_end_r` | E | 8 x 48, 16 x 48, 8 x 48 | structure | (288, 532), mids at 296, 312, 328, end_r at 344 | the retreat shelf's face (sidecar static x 288..351, y 532..579) |
| `trog_mirror_shards` | K | 96 x 74 | backwall_decal | (792, 470) | the gym mirror bay's frame with most of the glass gone: plywood backing, glue dabs, five shards; it stands on the arch sill, so the arch threshold (752, 544) hides none of it |
| `gym_girder` | R | 32 x 10 | structure | (892, 440) first, then (756, 440), (788, 440), (820, 440), (852, 440), (884, 440) | the beam run across bricked bay 3; the first girder, drawn under the last, closes the gap to the right pier (the camp's CAMP-AD15 rule) |
| `gym_fluoro_cold` | R | 40 x 10 | prop | (820, 450) | the one cold tube, hanging under the girders (plan 16: fluorescent light only here) |
| `gym_dumbbell_rack` | R | 56 x 28 | prop | (812, 532) | low in front of the bricked bay |
| `gym_plate_tree` | R | 28 x 36 | prop | (892, 524) | at the bay's right end |
| `gym_mat_a`, `gym_mat_b`, `gym_mat_a` | R | 48 x 22 | structure | (760, 561), (808, 561), (856, 561) | gym floor on the deck's back half, past the threshold |
| `trog_threshold` | K | 80 x 19 | structure | (672, 561) | the stone deck turning into tile courses under the bench's far side; no pixel over the pier plinth (columns ≥ 20 start at row 11) |
| `trog_bench_intact` | K | 32 x 80 | prop | (640, 500) | the 80 px static bench; its drawn bounds equal the sidecar static (section 7) |
| *(reserved)* `trog_bench_weakened`, `trog_bench_broken`, `trog_bench_debris_0..5` | K | 32 x 80, 32 x 80, 112 x 96 | none | none | frozen keys, pinned, not wired: TROG-Q14 (the bench break) |

**Step and shelf faces (E).** The low refuge, the bell step and the retreat shelf are sidecar solids the composer does not draw. Their faces are new executor pieces, derived from the AREA-HEAT-01 shelf body (`author_heat01._shelf_body`, 16 x 32): the same arris, slab lip, courses and nick, one stone step darker (every `stone` index below row 0 lowered by 1, floored at 0; the arris row is kept) so they never outshine a troglodyte in the lit read (TROG-ART-AD5). The 32 px mid is the heat body itself (courses at rows 10..19 and 21..31, bed joint at 20). The 48 px mid adds a third course (courses at rows 10..21, 23..34 and 36..47, bed joints at 22 and 35, head joints alternating at x 0 and 8). An 8 px end is one half of its mid (the left end the mid's right half, the right end its left half), with its two outer columns one step darker below row 0, a `stone` 6 corner on row 0, and the bottom outer corner cleared. Each piece keeps exactly one transparent pixel. The design gate used scratch versions of these six pieces built by exactly this rule.

No new piece goes in an open window above y 520. No new piece crosses a remaining torch rect (x torch..torch+15, y torch_y..torch_y+55).

## 6. Hero keys

**`trog_fist_hole` (64 x 80)**, the run's signature:
- a breach in the running bond shaped like a fist seen from the knuckles: four rounded knuckles across the top with finger webs between them, a body with rounded lower corners and a thumb bulging out of the left side;
- the void is `mortar` 0 with a few deeper specks; its inner rim shows the thickness of the wall (lit lower-right faces, dark upper-left);
- a spall zone round the breach where the weathered face was knocked off and fresh fired clay shows (`brick` warm);
- stepped cracks that follow the bond outward to the next head joints, and a loose brick fallen at the foot.

**`trog_bell` (40 x 64):** a riveted wall plate, a flat bar arm with a diagonal strut and a curled tip, a yoke, a dull brass bell (one ramp step darker than the raw brass, with a dent and one spot of highlight), and a frayed rope hanging from the clapper. Its brightest pixels are on the dent's rim; its lit mean stays under the Virtuoso's (section 10).

**`trog_mirror_shards` (96 x 74):** built from `gym_mirror_bay`'s own frame, clips, skirting and tape. The glass that is left (five polygons: the top-left sheet, a dagger from the seam clip, the right clip's shard, the taped corner and a shard in the bottom channel) is dark glass (`gal` 3..4, sheens `gal` 7). The missing glass shows the plywood backing (`mortar` 1/2) with glue dabs (`mortar` 3); broken edges are pale above and dark below, and fallen slivers lie in the bottom channel.

## 7. States and motion (presentation only)

**Superseded by TROGB-AD1 (TROG-ART-AD-S8): the bench is a partition again, the presenter and the burst below are built, and the three states and six burst frames are packed in the atlas `floor01_trog_bench`; the text below is the history of the static stage.** **Stage-2 amendment (TROG-AD46, TROG-ART-AD-S2).** TROG-B landed the bench as an 80 px **static solid** {640, 500, 32, 80}, not the plan's three-state partition: rule (b), the bench smash and the break are cut (TROG-Q14 is open). So the bench is **one statically placed `trog_bench_intact`** at (640, 500) (its drawn bounding box is exactly the static's rect, tested), and there is no bench presenter, no debris presenter, no `state_groups` entry and no partition lookup. The partition presenter and the debris burst below are **reserved for TROG-Q14** (kept as the frozen keys and this text, not built); TROG-ART-A8 and A9 are dropped with them, and gate notes G2, G3 and G7 are moot.

**The bench stack, `trog_bench` (32 x 80, 3 frames).** It is `gym_bench` (44 x 18) turned on its end, the pad on the left and the legs to the right, with seven weight plates lying flat in its lower half (rows 48..79), one ratchet strap lashed diagonally round the whole stack and its own shadow filling the gaps:
- `intact`: the drawn bbox is exactly the target {640, 500, 32, 80}. Row 0 is one walkable arris (`brick` 9) so the top reads as climbable (80 px, Carl's class limit);
- `weakened` (one or two counted hits): the same bbox. The pad is torn and spills foam, a plate has slid down its leg, the strap has snapped and hangs, the leg's weld is cracked. It is the only frame with pixels above luma 160 (the torn foam and the fresh metal), so the state change reads at 1x;
- `broken`: only the remnant, rows 64..79, exactly {640, 564, 32, 16}: plates flat on the floor, the crushed pad, a bent bar, the loose strap and a half-buried disc.

**The bench presenter** (the heat partition's pattern):
- it reads the partition's `PartitionState` of the room's `bench_stack` prop through `env.partition(prop)` and shows frame `PartitionState as usize` at the origin (640, 500), on the prop z just under the actors;
- it writes nothing back to the sim. The sim's collision is the sidecar's; the art only sits exactly on it;
- a pure function `bench_piece(state) -> &'static str` and `bench_origin(target) -> (i32, i32)` (= the target's top-left) live in `src/trog_kit.rs`.

**The debris burst, `trog_bench_debris` (112 x 96, 6 frames, origin (600, 484)).** The canvas spans x 600..711 and y 484..579, centred on the bench (x 656). Frames 0..4 are a ballistic burst sampled at ticks 1, 5, 9, 14 and 20 from the bench's foot (plates, vinyl, foam, the bar and strap ends, with one damped bounce on the floor row and floor friction after it) and a dust cloud of round puffs that spreads left and thins. Frame 5 is the settled litter (rows 93..95, world 577..579).
- **Timing:** frames 0..4 last 3, 4, 4, 5 and 6 ticks (50, 67, 67, 83 and 100 ms at 60 Hz); frame 5 then holds while the bench stays `Broken`.
- **Trigger:** the rising edge of `Broken` (the presenter's previous state was `Intact` or `Weakened`, the current is `Broken`). A room that starts with the bench already broken (a respawn keeps it broken, plan section 10) shows frame 5 with no burst.
- **Direction (TROG-ART-AD9):** the frames are drawn flying left, away from a Basher at the bench's right face. The burst is mirrored about x 656 (the canvas keeps its origin, 600, because it is centred on the bench) when the breaker is left of the bench: the breaker is the Basher when the break tick carries the adapter's `Trog { act: BenchSmash }` event, otherwise the living hero nearest to x 656. If the event is not reachable from the scene, the executor keeps the authored direction and records it (TROG-ART-Q2).
- **Cut and rewind guard:** if the presentation tick goes backwards (a fixture restart or a checkpoint retry) or the partition is restored to `Intact`, the burst is hidden and its state cleared. The burst never replays on a rewind; it replays only on a new rising edge.
- **Pure selection:** `debris_frame(state: &mut DebrisBurst, tick: u64, bench: PartitionState, mirror: bool) -> Option<(usize, bool)>` in `src/trog_kit.rs` (frame index and mirror flag, `None` when hidden).

No sound in this order (TROG-Q7, TROG-ART-Q3).

## 8. Lighting (presentation only)

- **`trog_run`:** minus one torch at x 2240 (its flame light and floor pool, minus 2 lights). No light is added: the territory is lit by the composer's torches only.
- **`trog_strength`:** minus two torches at the room ends (minus 4 lights), plus `gym_fluoro_cold`'s two lights (`tube` and `tube_floor`, the gym kit's own values). The net change is −2 lights. The cold tube is the room's only cool light, so the bay reads as "gym" against the warm catacomb (plan 16).
- The worst 640 px view of each room stays at or under `MAX_LIGHTS` (64).
- Light colours are not palette pixels; the pixels stay on `floor01-catacomb-v2`.

## 9. Collision and state

1. Solids, spawns, checkpoints, exits, views, the walking line, the ceiling slab, the blink barriers and the bench's static rect are identical with and without the dressing.
2. The bench and debris presenters read the partition state, the room tick and, for the burst's direction only, the break tick's events and the heroes' x. They write nothing back.
3. The sim never reads art. The bench's collision is the sidecar's (plan 5.4); the drawn bounds equal it (section 7).
4. Steps and shelves keep their sidecar heights: the art face of each covers exactly the solid's rect (x0..x1, y0..579).

## 10. Readability rules

1. **Troglodytes first** (plan 16). In every gate view, measured on the **lit** composite (the kit's light model, `env_kit_light.render_lit`), with `lookdev_compose.readability(img, mask, boxes, ring=6)` in CIE L*:
   - every new or reused dressing piece in view has a mean L* at or under the lowest in-view troglodyte's actor L*. Emissive fixtures (`gym_fluoro_cold`) are excluded;
   - every troglodyte's silhouette edge contrast (`edge_dL`) is at least 15.
   The heroes are measured and reported, not gated against the dressing: Carl's sprite is brighter than any troglodyte sprite (TROG-ART-Q6).
2. **Decals show.** Each new backwall decal and each new structure or prop piece has at least 90 % of its opaque pixels visible behind the architecture (the prop and foreground layers emptied), and no backwall decal pixel lies over a null (background) pixel. The figure with the props in front is reported too.
3. **Windows stay windows.** The window rule of section 2. No background shows in a bricked bay (piers included).
4. **Lanes stay clear.** Nothing in front of the actors (no `foreground` piece). Floor pieces from y 560 down are flat strips on `structure` (the crater, the threshold, the gym mats).
5. **No slivers.** Every placement meets its neighbour or a wall edge, or leaves at least 4 px of gap: the girder run, the step and shelf runs, the mats.
6. **The bench reads as an object, not a wall.** Its plates, strap and pad give it inner contrast; its top arris is one bright row (climbable); its weakened frame is visibly damaged at 1x.

## 11. The keys

`python3 scripts/author_trogarea01.py` writes the eight grids and their `.ase` sources and prints the key pins; `keys` prints the pins only; `measure` prints the key measurements. Every pixel comes from rules (shapes, ramps, a hash noise) or from committed kit pixels (`gym_bench` for the stack, `gym_mirror_bay` for the shards). The SHA-256 pins of the sixteen key frames are in `gate/measurements.txt`. The execution keeps them byte-identical. A key change is a design change.

- `assets/source/props/floor01/trog_fist_hole.grid.json`: 64 x 80.
- `assets/source/props/floor01/trog_bell.grid.json`: 40 x 64.
- `assets/source/props/floor01/trog_wall_marks.grid.json`: `trog_knuckle_marks` 96 x 40, `trog_slam_crack` 48 x 48.
- `assets/source/props/floor01/trog_crater.grid.json`: 72 x 20.
- `assets/source/props/floor01/trog_threshold.grid.json`: 80 x 19.
- `assets/source/props/floor01/trog_mirror_shards.grid.json`: 96 x 74.
- `assets/source/props/floor01/trog_bench.grid.json`: 32 x 80, 3 frames.
- `assets/source/props/floor01/trog_bench_debris.grid.json`: 112 x 96, 6 frames.

All keys together use 51 distinct palette entries (budget 63 per atlas).

The gate stills are **scratch composites**, not runtime captures (neither room is dressed in code yet, and `trog_strength` is not on main):
- the two rooms are composed by the real Rust composer (the plain TROG-A composition), dressed with the reused pieces and the keys through a scratch contract and `ring_kit::dress_with`, and dumped as layouts;
- the layouts are rendered with the kit renderer and its light model (`env_kit_light.render_lit`), lit and unlit;
- the troglodytes (frame 0, facing left) and Carl are pasted at their spawn x, feet on y 579; the bench is pasted at its origin;
- the six step and shelf faces are scratch versions built by the rule of section 5.

The probe is not part of this PR. The executor replaces the stills with real window captures (section 13).

## 12. Acceptance targets

| Id | Target | How it is checked |
| --- | --- | --- |
| TROG-ART-A1 | Solids, spawns, checkpoints, exits, views, walking line, ceiling slab, blink barriers and the bench partition rect are identical with and without the dressing, in both rooms | `trog_kit` unit test on the composed layouts |
| TROG-ART-A2 | Room, sidecar, `MANIFEST_HASH` and TABLE hashes, the state hash and every golden unchanged | existing pins; the window `COMBAT_STATE` hash equals the headless hash at every captured tick of both rooms |
| TROG-ART-A3 | Every `remove_at` matches (zero unmatched) in both rooms | unit test: `dress_with` returns 0 |
| TROG-ART-A4 | Window rule: no prop's opaque pixel in an open window above y 520 | art unittest on the composed layouts (pixel model) |
| TROG-ART-A5 | No background pixel in the bricked bays (run x 176..383 and 1184..1391, strength x 720..927; y 352..559) | unit test (the FF-AD29 test) |
| TROG-ART-A6 | Every placement in y 561..579 is on `structure` | unit test |
| TROG-ART-A7 | The worst 640 px view of each room stays at or under `MAX_LIGHTS` | test as `the_gym_lights_stay_inside_the_light_budget` |
| TROG-ART-A8 | *(dropped under TROG-AD46)* the bench is a static placement: its drawn bbox equals the sidecar bench static {640, 500, 32, 80} | `trog_kit` test `strength_the_shelf_and_the_bench_are_drawn_on_their_statics`, art unittest |
| TROG-ART-A9 | *(dropped under TROG-AD46)* no debris presenter; the weakened, broken and debris keys are reserved for TROG-Q14 and appear in no atlas or contract entry | art unittest |
| TROG-ART-A10 | Key frames byte-identical to the pins in `gate/measurements.txt` | art unittest |
| TROG-ART-A11 | Palette: every piece on `floor01-catacomb-v2`, at most 63 colours per atlas, 1-bit alpha, at least one transparent pixel per piece | `validate_assets.py`, art unittests |
| TROG-ART-A12 | Lit read (section 10.1) holds on the runtime stills: every dressing piece's lit mean L* ≤ the lowest in-view troglodyte's, every troglodyte's `edge_dL` ≥ 15 | the evidence script measures the window stills against the same masks |
| TROG-ART-A13 | Visibility (section 10.2): ≥ 90 % of each new piece visible behind the architecture, `backwall_null_px` 0 | art unittest on the composed layouts |
| TROG-ART-A14 | No text, glyph, logo, numeral or face | review |
| TROG-ART-A15 | Window stills at 640 x 360 show the slam crack and knuckle marks, the refuge face, the crater, the fist hole with the Basher in front, the bell and step, the shelf, the bench in each state with the debris burst, the threshold, the gym bay and the shards legibly at 1x, with the troglodytes reading first | owner review of the stills |

## 13. Execution list

Execution starts only when the room exists on main: **stage 1** (`trog_run`) after TROG-A has merged, **stage 2** (`trog_strength`) after TROG-B has merged. The two stages may be one PR each.

1. The pieces marked E in section 5 go in `scripts/author_trogarea01.py` as new functions; the keys stay untouched. Then the atlases, the pixel-grid and `.ase` sources, the manifests and the exports through `scripts/export_assets.sh`.
2. The contract `assets/source/terrain/floor01/floor01_trog_kit.json` (extends the gym kit): its atlases and pieces, a `room_dressing` block per room (section 4 and 5), and (stage 2) the two `state_groups` entries for the bench and the debris.
3. `src/trog_kit.rs`: the dressing load and apply through `f01_boiler::compose_kit_hooked`, keeping TROG-A's ceiling slab and statics exactly as `f01_trog::launch` pushes them; (stage 2) `bench_piece`, `bench_origin` and the pure `debris_frame`. Tests TROG-ART-A1, A3, A5..A9.
4. Bevy:
   - `f01_trog::launch` composes the dressed kit (a `dressed` flag like `f01_rest::launch_with`; the plain composition stays for the presentation-only tests);
   - the binary skips the flat greybox decor for a dressed room (`f01_trog::decor` stays as data, its entry line unchanged);
   - (stage 2) the bench and debris presenter sprites in `src/bin/floor01_scene/trog_art.rs`, modelled on `corral_art::spawn_partition` / `sync_partition_art`.
5. Evidence at `docs/evidence/area-trog-01/`: real window stills of both rooms, the double captures, the hash equality, the lit read and visibility measurements on the real stills, contact sheets, a bench state strip and a debris strip from the window, the art unittest `tests/test_art_trogarea01.py`.

## 14. Files

| Path | Content |
| --- | --- |
| `docs/02-art/area-trog-01-spec.md` | this spec |
| `scripts/author_trogarea01.py` | the authoring record (keys; at execution every piece, atlases, manifests, contract and measurements). `scripts/author_trog01.py` is the TROG-01 sprite's script and is not touched |
| `assets/source/props/floor01/trog_fist_hole`, `trog_bell`, `trog_wall_marks`, `trog_crater`, `trog_threshold`, `trog_mirror_shards`, `trog_bench`, `trog_bench_debris` (`.grid.json` + `.ase`) | the keys |
| `docs/evidence/area-trog-01/gate/` | gate sheets, the composed stills, `measurements.txt` with the key pins |
| *(execution)* `assets/manifests/floor01_trog_*.json` | manifests (`decor_atlas`, `prototype_fixture_unapproved`) |
| *(execution)* `assets/source/terrain/floor01/floor01_trog_kit.json` | the contract |
| *(execution)* `src/trog_kit.rs`, `src/bin/floor01_scene/trog_art.rs`, `tests/test_art_trogarea01.py` | the dressing, the presenters, the art unittest |

## 15. Adopted defaults and open questions

Adopted under the owner's design-defaults authorization (AGENTS.md, 2026-11-19); each is a value the owner may change in a follow-up PR.

| Id | Adopted default | Reason |
| --- | --- | --- |
| TROG-ART-AD1 | Route B + silhouette gate (score 3) | a new family, pieces above 40 px, a state prop with a burst |
| TROG-ART-AD2 | One contract `floor01_trog_kit.json` for both rooms, extending the gym kit, applied with `ring_kit::dress_with` | the strength room needs the gym pieces; the run needs only the catacomb, which the gym kit already extends; one contract keeps one atlas family |
| TROG-ART-AD3 | Bays 1 and 4 of the run and bay 3 of the strength room are bricked up (`backwall_fill`) and their `arch_chain` removed | the wall marks, the hole and the shards need a wall, not a window; no chain across a decal |
| TROG-ART-AD4 | The fist hole sits at (1312, 440), not the greybox (1344, 500) | the void must stay above the Basher's sprite top (493) so the Basher stands in front of the breach; x 1312..1375 is on the bricked bay and ends at pier_r 1376 |
| TROG-ART-AD5 | The refuge, bell step and shelf faces are new executor pieces from the heat shelf body, one stone step darker | the composer draws nothing for the sidecar statics; at the heat shelf's value they out-read the Pygmy and the Virtuoso in the lit gate |
| TROG-ART-AD6 | The gym mirror is not reused whole: `trog_mirror_shards` replaces it, at (792, 470) | in the gate the whole `gym_mirror_bay` measured lit L* 34.2 against the Virtuoso's 25.5..26.2 (it out-read the troglodytes); the shards placed at y 486 were only 75..78 % visible behind the arch threshold, 100 % at y 470 |
| TROG-ART-AD7 | Of each doubled composer torch pair (run 2240; strength 32/48 and 944/960), the y 432 torch and its pool are removed | two flames 16 px apart read as a glitch and add two lights each |
| TROG-ART-AD8 | The bell is dull brass (one ramp step darker than raw brass) | at raw brass it measured lit L* 34.7 against the Virtuoso's 26.2 in the gate |
| TROG-ART-AD9 | The debris burst flies away from the breaker: authored leftward (the Basher smash), mirrored for a breaker left of the bench | the plan's headline beat is the Basher smash; a hand break from the left must not throw debris at the hitter |
| TROG-ART-AD10 | The lit read of section 10.1 is the operational form of plan 16's "troglodytes the brightest, highest-contrast shapes": against the dressing, not against the heroes | the troglodyte sprites are darker than Carl by design and are out of scope here (TROG-ART-Q6) |
| TROG-ART-AD11 | The authoring script is `scripts/author_trogarea01.py`, not the brief's `author_trog01.py` | `author_trog01.py` already exists on main as the TROG-01 Pygmy sprite's script |
| TROG-ART-AD12 | No audio; no Pygmy-specific dressing beyond the crater | TROG-Q7; the Pygmy's lesson is its landing, which the crater already marks |

| Id | Open question | Recommendation | Owner of the answer |
| --- | --- | --- | --- |
| TROG-ART-Q1 | The lane ceiling slab (y 336..351) has no art in the plain composition. Should it get a beam or vault piece? | No piece in this order; follow whatever the Boiler room shows for the same slab, in a kit-wide follow-up | owner art review |
| TROG-ART-Q2 | (Closed by TROG-ART-AD-S8: the event is appended and reachable.) Is the break tick's `BenchSmash` event reachable from the scene for the debris direction? | If not, keep the authored leftward burst for every break and note it; a hand break from the left is the rarer case | TROG-B / executor |
| TROG-ART-Q3 | The slam through stone, the bench impact and the gym hum (TROG-Q7) | none until the game has an audio path | audio order |
| TROG-ART-Q4 | More Pygmy territory dressing (a nest, gnawed bones) in T1? | no; keep the run readable and the Pygmy's lesson about its landing | owner art review |
| TROG-ART-Q5 | The static fallback bench (TROG-AD12, a 48 px solid) has no art | if the fallback fires, the executor draws `trog_bench_low` 32 x 48 from the intact key's lower rows plus the cap, as an executor piece, and the presenter is cut | TROG-B outcome |
| TROG-ART-Q6 | The troglodytes' mean value is low against their surround (`delta_L` 0.4..11.5 in the gate) and Carl is brighter than any of them (L* ≈ 41 vs 25..33). Plan 16 asks the troglodytes to be the brightest shapes | a rim or value pass on the TROG sprites in the troglodyte family's own review, not in this kit | trog family review |

**Horizon fit:** the kit changes no move, ability, unlock or gate. It dresses the troglodyte run and the Test of Strength; the bench art shows a state the sim already has. The [progression horizon](../01-design/character-progression-horizon.md) still fits; it is not edited.

## 16. Execution record

### 16.1 Stage 1: `trog_run` (executed on top of TROG-A, #310)

Executed by Claude Sonnet from the frozen keys and the gate's binding notes G1..G8; designed by Claude Opus, gated by an independent Claude Opus. **Prototype art, independent art review pending, not owner-approved.** Stage 2 (`trog_strength`: the 48 px shelf, the threshold, the shards, the bench presenter and its debris burst) is **not** executed: TROG-B is not on main (gate note G8). No stage-2 piece is authored, wired or tested here beyond the sixteen key pins, which stay in the art unittest.

**Pieces and atlases** (`scripts/author_trogarea01.py`: new functions below the keys, which are byte-identical; `python3 scripts/author_trogarea01.py` writes grids, `.ase`, manifests and the contract):

| Atlas | Canvas | Colours | Pieces (layer) |
| --- | --- | --- | --- |
| `floor01_trog_run_wall` | 176 x 136 | 12 | `trog_fist_hole` (backwall decal), `trog_knuckle_marks` (backwall decal), `trog_slam_crack` (backwall decal) |
| `floor01_trog_run_props` | 128 x 104 | 34 | `trog_bell` (prop), `trog_impact_crater` (structure), and the three executor pieces `trog_step_mid` 16 x 32, `trog_step_end_l` and `trog_step_end_r` 8 x 32 (structure) |

Pieces sit in the atlas at their exact size with 8 px padding. The step pieces are the AREA-HEAT-01 shelf body one stone step darker below the arris row, each with exactly one transparent pixel and no `stone` above index 7 below row 0 (section 5; the code is the gate's rule, reproduced as `end_of` and `darken`). The contract is `assets/source/terrain/floor01/floor01_trog_kit.json` (extends the gym kit, no inherited key redeclared, nothing collidable, manifests `floor01_trog_run_wall.json` and `floor01_trog_run_props.json`, status `prototype_fixture_unapproved`). Its `room_dressing.trog_run` is exactly sections 4 and 5: six `remove_at` (both bricked bays' catenaries and blue sills, the upper torch at x 2240 and its pool), two `backwall_fill` rects, an empty `parallax`, and 15 placements (three decals, then structure, then the bell). Every `remove_at` matches exactly one composer piece.

**Wiring.** `src/trog_kit.rs` composes the room through `f01_boiler::compose_kit_hooked` + `ring_kit::dress_with` (an unmatched entry is an error) and pushes the DP-Q45 ceiling slab and the sidecar's `solid` statics in one shared `lane_solids`, for the dressed and the plain composition alike. `f01_trog::launch` is `launch_with(root, room, true)`; `launch_with(.., false)` is the plain TROG-A composition. The binary no longer draws the flat decor (`f01_trog::decor` stays as data; the `F01_TROG_RUN_ENTRY` line is unchanged; the `F01_TROG_ART` line prints `decor=dressed` and the `F01_TROG_DECOR` lines are gone, the camp's REST precedent).

**Tests** (`cargo test --lib trog_kit`, `tests/test_art_trogarea01.py`):

| Id | Test |
| --- | --- |
| A1 | solids (slab and statics included), spawn, exit, views, walking line, blink barriers, collision cells and tiles, actor z and deck are identical with and without the dressing; the launched rooms and their hashes and statics too; the state hash is equal on all 500 ticks of the smoke script |
| A3 | `dress_with` returns 0; each of the six `remove_at` drops exactly one composer piece, the bays' sills included; an unmatched entry is an error; the torch at (2240, 448) and the open windows' chains stay |
| A5 (G5) | the undressed bays' background span (x 144..1503 pier to pier, y 352..559) lies inside the `backwall_fill` rects; the dressed layout shows no background in either bay |
| A6 (G5) | the 11 dressing placements that touch y 561..579 (ten step pieces and the crater) are on a layer above `collision` |
| A4 | no `prop` pixel in an open window above y 520, from the real pixels |
| clearance | no new `trog_*` prop or structure pixel inside a remaining `torch_wall` rect |
| A7 | the worst 640 px view stays within `MAX_LIGHTS`, net minus two lights |
| 9.4 | the step faces cover the two sidecar statics edge to edge, down to the walking line |
| A10 (G4) | the sixteen key pins equal the name and digest of `gate/measurements.txt` (not the raw line), and the eight key grids are byte-identical |
| A11 | sizes, at most 63 colours per atlas, 1-bit alpha, one transparent pixel per piece, kit palette only |

`dump_trog_layouts` (ignored) writes the composed layouts for the sheets.

**Evidence** (`docs/evidence/area-trog-01/`): twelve window stills at 640 x 360 and 1920 x 1080, each captured twice (byte-identical), window `COMBAT_STATE` hash equal to the headless hash at every still, and equal to TROG-A's hash on the nine stills it also captured (same pilot, tick and size). Drawn views measured from landmarks. The lit read and visibility on the real stills:

- **A12 (G6), gated stills** (a troglodyte in an idle pose in view): `t0_spawn`, `pygmy_dormant`, `basher_dormant`, `virtuoso_dormant`. The troglodytes are located on each still by masked normalised cross-correlation of the atlas frames (the pose and position are not logged; every gated match has ncc 1.00). Result: every dressing piece's lit mean L* is below the lowest in-view troglodyte's actor L*, and every edge_dL is at least 15.

  | Gated still | Troglodyte (idle frame) | actor L* | edge_dL | Pieces in view (lit mean L*) |
  | --- | --- | --- | --- | --- |
  | `t0_spawn` | Pygmy (2) | 29.0 | 18.1 | slam crack 17.5, knuckle marks 20.0, step faces 17.3..22.2, crater 19.1 |
  | `pygmy_dormant` | Pygmy (1) | 28.2 | 19.3 | slam crack 17.3, knuckle marks 20.0, step faces 14.2..21.8, crater 19.1 |
  | `basher_dormant` | Basher (1) | 33.6 | 25.7 | fist hole 12.5 |
  | `virtuoso_dormant` | Virtuoso (2) | 25.4 | 18.6 | bell 24.1, bell step faces under 22.2 |

  The thinnest margin of the run is **the bell against the Virtuoso: 24.1 against 25.4 (1.3 L*)**, as the gate predicted; it passes, so no owner note on Q6 is raised by a failure. Attack and walk poses are measured and reported, not gated (`lit_read.txt`): the Pygmy 27.8..31.3 (edge_dL 15.1..38.9), the Basher 32.8..35.3 (24.1..25.0) and the Virtuoso 25.7..29.2 (16.8..17.9) against the same dressing, all passing.
- **A13:** all 15 placed pieces are 100 % visible behind the architecture and with the props in front, `backwall_null_px` 0.
- **Drawn views** (`drawn_views.txt`): the lit model slid over each still's wall rows, trimmed error 0.25..0.67, cross-checked on the torch flames (flame centre = placement x + 7.5, mean offset +0.1..+2.2 px).

**Gate notes G1..G8 and the open ends.**

| Note | Status |
| --- | --- |
| G1 (torch above the bench) | stage 2, not executed |
| G2 (debris start from the event log) | stage 2, not executed |
| G3 (breaker direction) | stage 2, not executed; TROG-ART-Q2 stays open until TROG-B lands |
| G4 (pin format) | executed: the art unittest compares the name and digest |
| G5 (A5, A6 wording) | executed as the gate worded them (table above) |
| G6 (A12 executable) | executed on stage 1 (above); no key failed |
| G7 (static fallback piece) | not needed unless TROG-B fires TROG-AD12 |
| G8 (stage gating) | applied: TROG-A is on main, stage 1 only; the room JSON, sidecar statics (refuge 352..415, bell step 1840..1903), pier x and openings were re-read on the landed files and equal sections 2 to 5 |

Stage 2 rooms launch plain until their `room_dressing` block lands (AD19).

**Adopted defaults of the executor** (the owner may change any in a follow-up PR):

| Id | Default | Reason |
| --- | --- | --- |
| TROG-ART-AD13 | Stage 1 ships alone: no stage-2 piece, atlas or contract block is authored | G8: the strength room and its bench numbers are not on main; unwired pieces would be dead code without tests |
| TROG-ART-AD14 | Two atlases for stage 1, `floor01_trog_run_wall` and `floor01_trog_run_props`, 8 px padding; the crater and the step pieces are `structure` inside the props atlas (per-piece layer override, the camp pattern) | the brief's atlas split; floor pieces draw above the deck (A6) |
| TROG-ART-AD15 | The binary draws none of `f01_trog::decor` and prints `decor=dressed`; the `F01_TROG_DECOR` lines are removed | nothing consumes them; every troglodyte room is dressed (the REST precedent) |
| TROG-ART-AD16 | The troglodytes' pose and position on a still are measured by masked normalised cross-correlation of the atlas frames; a still is gated when an idle frame (0..2) is in view | the pose is not logged and the idle loop picks frame 0, 1 or 2 by tick; every match is pixel-exact (ncc 1.00 on the gated stills) |
| TROG-ART-AD17 | New pilot ticks for stills TROG-A did not take: 20 (`t0_spawn`), 440 (`basher_dormant`), 840 (`virtuoso_dormant`) | the dormant poses the gate needs; TROG-A's nine other ticks keep its names and its hashes |
| TROG-ART-AD18 | A piece's lit mean in a window still counts only the pixels visible in the layout and explained by the lit model (|L* difference| at most 12) | Carl, Donut, the HUD and effects must not enter a dressing piece's mean; `lit_read.txt` prints the dropped pixel counts per piece. A downed troglodyte (atlas tag `down`) is not counted as an actor (G6) |
| TROG-ART-AD19 | `trog_kit::dressing_of` returns `Option`: a troglodyte room whose contract has no `room_dressing` block composes plain (`launch` stays `launch_with(.., true)`), and the binary prints `decor=dressed` or `decor=plain` from `trog_kit::is_dressed`; no flat decor is drawn either way. A malformed block is still an error. Stage 2 rooms launch plain until their block lands | review R1: a room without a block (TROG-B's `trog_strength`, which also runs on the static bench fallback, TROG-AD12, so stage 2 follows G7) must not fail to launch; tested with a synthetic room id |
| TROG-ART-AD20 | The composer's blue `arch_threshold` sills of the two bricked bays, (208, 544) and (1216, 544), are removed | review R2: in a bricked bay the moonlit sill reads as a cold band under warm brick and leaves a 4 px blue lip on the refuge face; the fill already covers y 352..559. The camp aftermath's B2 sill is a kit-wide owner call, not this PR |

No design problem was found: no key needed a change, and no placement contradicted the landed geometry.

### 16.2 Stage 2: `trog_strength` (executed on top of TROG-B, #312)

Executed by Claude Sonnet from the frozen keys, the gate's binding notes and the stage-1 review's lessons; designed by Claude Opus, gated and reviewed (stage 1) by independent Claude Opus agents. **Prototype art, independent art review pending, not owner-approved.** Presentation only: no sim, room, sidecar, MANIFEST, TABLE, golden or playtrace pin moves; the sixteen keys are byte-identical.

**What TROG-B changed** (#312, TROG-AD46). The bench is an 80 px **static solid** {640, 500, 32, 80} in the sidecar, with the 48 px shelf {288, 532, 64, 48} (both re-read on the landed files and equal to sections 2 to 5; the Basher spawns at 800 and its leash starts at 704, the Virtuoso at 496, the exit at 944). Rule (b), the bench smash and the partition are cut (TROG-Q14 is open). So the partition presenter, the debris burst, `state_groups`, `BenchSmash`, TROG-ART-A8/A9, `DebrisBurst`, `debris_frame` and gate notes G2, G3 and G7 are not built (section 7 amendment). `trog_art::spawn` (changed by #312) draws the flat decor rectangles only for a plain room; with the block below the room composes dressed and none are drawn.

**Pieces and atlas.** One new atlas, `floor01_trog_strength` (184 x 162, 40 colours, manifest `floor01_trog_strength.json`): the keys `trog_threshold` (structure), `trog_mirror_shards` (backwall decal) and `trog_bench_intact` (prop), and three executor pieces `trog_shelf48_mid` 16 x 48, `trog_shelf48_end_l` and `trog_shelf48_end_r` 8 x 48 (structure; the 48 px rule of section 5, the gate's code). The bench is folded into this atlas as one 32 x 80 frame, the smaller change than a second one-frame atlas. The weakened, broken and debris keys stay in the author script and the pinned measurements and are in no atlas, manifest or contract entry (reserved for TROG-Q14). `room_dressing.trog_strength` is sections 4 and 5: ten `remove_at`, one `backwall_fill`, 20 placements (the shards; the girder run, the first girder under the last; the shelf face; the mats; the threshold; the bench; the rack, the cold tube and the plate tree).

**Lessons applied.** No blue sill: the bricked bay's `arch_threshold` at (752, 544) is removed (the stage-1 review's R2). `dressing_of` is optional (AD19), so a room without a block still launches plain. The torch above the bench (664, 432) and its pool are removed (G1), and so are the torch above the retreat shelf (328, 432) and its pool (the #313 review's R1, AD-S7); no remaining torch flame column stands over the bench or the shelf, tested. Floor pieces are `structure` (A6). Nothing hangs in an open window (A4).

**Tests** (`cargo test --lib trog_kit`, `tests/test_art_trogarea01.py`): the strength set repeats the run's: the dressing changes no collider, spawn, exit, view, line, barrier or cell, nor the launched room, its hashes or statics, and the state hash is equal on 500 ticks (A1); ten `remove_at`, each dropping exactly one piece, the open bays keeping their sills and chains (A3); no background in the bricked bay, the undressed span lying inside the fill (A5); the ten floor placements above the deck (A6); no prop in an open window and no covered torch (A4); the light budget, minus eight lights for the four torches and plus the tube's two (A7); the shelf faces cover the shelf static edge to edge; the placed bench key's opaque bounding box equals the sidecar bench static exactly (the stage-2 form of A8); and no weakened, broken, debris or `state_groups` entry is in the contract.

**Evidence** (`docs/evidence/area-trog-01/`, README): seven window stills at 640 x 360 and 1920 x 1080 with the TROG-B pilot (`strength_*`), each captured twice and byte-identical, window hash equal to the headless hash on all 38 lines of both rooms, and equal to TROG-B's on the eight lines it shares (ticks 30, 270 and 360 at their sizes, 105, 381 and 450). Lit read (A12, gated on `strength_open`, `strength_bench_beside`, `strength_bench_climb`): every piece below the lowest living troglodyte, the thinnest margin the shelf mid against the Virtuoso (18.6 against 25.6, 7.0 L*, after the shelf torch's removal, AD-S7), the bench 19.2..19.9, the shards 11.9; every troglodyte edge_dL at least 18.9. A13: all eight placed pieces 100 % visible behind the architecture, the shards 96.7 % with the rack in front.

**Adopted defaults** (the owner may change any in a follow-up PR):

| Id | Default | Reason |
| --- | --- | --- |
| TROG-ART-AD-S2 | The bench is one statically placed `trog_bench_intact` at (640, 500) under TROG-AD46; the weakened, broken and debris keys are reserved for TROG-Q14 and not wired; the partition presenter, debris presenter, `state_groups`, A8/A9 and gate notes G2, G3 and G7 are dropped | TROG-B landed an 80 px static solid, not a partition; the key's drawn bounds equal the static exactly |
| TROG-ART-AD-S3 | The bricked bay 3's blue `arch_threshold` sill at (752, 544) is removed | the stage-1 review's R2 (AD20): a cold band under warm brick |
| TROG-ART-AD-S4 | The bench key is folded into `floor01_trog_strength` instead of a second atlas | the smaller change: no extra manifest, source or contract entry |
| TROG-ART-AD-S5 | The torch above the bench (664, 432) and its pool are removed (G1); the fallback of G1 (restore it if A12 fails) was not needed | the bench's lit mean stays 19.2..19.9 under the Basher (32.5) and the Virtuoso (25.7) |
| TROG-ART-AD-S6 | Seven strength stills with the TROG-B pilot at the ticks TROG-B used (30, 105, 270, 360, 381, 450) and one new tick, 230 (`strength_bench_beside`); a gated still is one with a living troglodyte in an idle pose | the new tick gives the sleeping Basher beside the bench and the gym bay in one view; TROG-B's ticks keep their hashes |
| TROG-ART-AD-S7 | The torch above the retreat shelf, `torch_wall (328, 432)`, and its pool `torch_pool_b (224, 348)` are removed (data only); the G1 clearance test covers the shelf as well as the bench | #313 review R1: the raised middle torch of the pier-352 group stood over the shelf's top, so a hero on it had a flame growing from the head, and three flames crowded the shelf; the torches at 272 and 368 still flank it the way 608 and 704 flank the bench |
| TROG-ART-AD-S8 | **TROGB-AD1 restores the bench presenter and the burst** (no new key). The three state frames and the six burst frames, byte-identical to the frozen keys, are packed in the new atlas `floor01_trog_bench` (240 x 408, 31 colours; the bench key moves out of `floor01_trog_strength`), and the dressing no longer places `trog_bench_intact`. `trog_kit::{bench_piece, bench_origin, bench_atlas_rects, debris_frame, break_tick, break_was_smash, break_mirror}` are the pure half; `trog_art::{spawn_bench, sync_bench_art}` the Bevy half (the bench on the prop layer just under the actors, the burst just above them). The burst frame is a pure function of the break tick (`PropPhase::Broken` in the attempt's log) and the tick, not of a rising edge seen frame by frame, so a capture that skips frames draws the same frame (a bench first seen broken, a respawn after the burst, shows the litter, frame 5); the mirror is `false` for a `BenchSmash` on the break tick, otherwise taken once from the living hero nearest the bench; TROG-ART-A8 and A9 are replaced by the `trog_kit` tests `strength_the_shelf_and_the_bench_are_drawn_on_their_statics` and `debris_frame_mapping`. Stills: `docs/evidence/trog-bench-smash/` | The sim now logs the break and the smash; the key's drawn bounds equal the partition's target and remnant (tested) |
| TROG-ART-AD-S8 | The 4 px seam between the shelf face (ends x 351) and the pier-352 plinth (starts x 356) is accepted, not covered | #313 review R2 (optional nit): the face covers exactly the static (9.4); a covering strip would be a new executor piece and a design call on a contact shadow, for the top tier |

Owner notes carried from the gate for the review: the bench may read as a dark cabinet or doorway with a strap rather than an upended gym bench (its value is capped by the lit rule, TROG-ART-Q6); the plywood between the shards' glass corners may read as mountains at 1x. TROG-ART-Q5 (the static-fallback art `trog_bench_low`) is closed: the static bench is the full 80 px key.
