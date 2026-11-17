# Floor 01 catacomb environment kit (ENV-01)

**Status:** owner-approved kit art and contract v2 on 2026-11-16 (PR #26). Lighting values are a Pillow model; Bevy implementation and re-tune happen in COMP-01.
**Target frame:** the owner-approved look-dev frame **A1** ([ADR-0007](../05-decisions/ADR-0007-rendering-resolution-lighting.md), [look-dev write-up](lookdev-floor01-resolution-spike.md)): 640×360 palette-limited pixel layers plus a screen-resolution lighting pass, parallax gallery layers and a deep floor.
**Binding direction:** [Floor 01 opening](floor-01-opening-catacomb-goblins.md): grammar A/B/C and the Environment locks (dry, clean brick catacomb; no sewer, toxic, factory or TV look; industrial detail sparing and local). ADR-0007 replaces the old "muted red-brown brick" wording with warm torch-lit brick falling into cold blue shadow.
**Authored by:** Claude Opus (AI art agent), 2026-11-16. **Evidence:** [`docs/evidence/floor01-catacomb-kit/`](../evidence/floor01-catacomb-kit/README.md)

## Purpose

A modular, editable pixel-art kit for the calm first corridors of Floor 1, built so that a sample layout made only from kit pieces reproduces the A1 mood:

- **A:** plain warm brick corridor, torches with baked light pools, a timber door, an optional plain pilaster;
- **B:** a thick sculpted arch on piers, opening onto a deep layered gallery (two parallax strips);
- **C:** one thick copper/brass pipe run with a valve, used sparingly.

The kit is authored at the native virtual resolution (640×360, 640×400 for Steam Deck) on the 16×16 tile grid, so actors sit in front of it without scaling. It contains no entrance landmark, goblins, Dozer, workshop, gameplay or Rust/Bevy code.

## What changed in round 3 (owner review of PR #26, 2026-11-16)

| Owner feedback | Round 3 |
| --- | --- |
| Pipes rejected | Thick (14 px) copper pipe with brass flanges, collars and valve wheel, iron clamps, cylinder-shaded with a specular streak. One run, C only. |
| Brick colour rejected | Warm orange torch-lit brick that falls off into cold blue-violet shadow: brick tones come from the A1 hue-shifted ramp; the wall is built from height bands (cold under the ceiling, warmer lower down) and baked torch pools. |
| Arch background rejected | The flat blue arcade and the `recess_*` tiles are gone. Arch windows are empty in the backwall; two horizontally tiling gallery strips (far: lower arcade with distant torches and tall cold windows; mid: pillars, balcony with railing, arches, one torch) show through at parallax 0.35 and 0.6. |
| Liked: readability, Carl/Donut positions, 0 px foot contact | Same actor positions as A1; feet stand on the walking line with 0 px sink; a lighter warm band behind actor height plus a presentation-only contact shadow (measured below). |
| Floor (ADR-0007) | Deep floor: 24 px visible top surface, walking line 4 px behind the front lip. |

## Revision round 4 (independent Claude Opus review of round 3)

| Review item | Round 4 |
| --- | --- |
| S1 bottom band read as bright blue small bricks (L* 9.8, b* −18) | `fill_*` redrawn as 32×16 dark blocks in three tone levels by absolute row (L* 6.5/4.8 → near black). Bottom 25 px of the lit 640×360 view: **L* 2.8, b* −8.0** (A1: 5.6 / −13.2). |
| S2 light-blue line under platforms; platforms read as shafts | No fill edge singles any more: fill courses are continuous rows whose joints are phase-locked to the world running bond (`x ≡ 16 × (row mod 2) mod 32`), so platform and floor fill join seamlessly (tested). |
| S3 torch side of the pipes darker than the centre | One pipe key from the torch side (+x for vertical, below for horizontal; elbows blend both): highlight about 2 px toward the torch, 1 px hot rim on that side, cold rim on the far side. |
| S4 valve wheel bloomed | `bloom.emissive_colours` (flame ramp + distant-torch warms) masks bloom to flame pixels; copper and brass occlude it (tested). |
| S5 base course too close to Carl's legs | Base course darker and cooler toward the deck: lit L* next to Carl **33.1** (was 43.1), b* 7.6 (was 17.1). `brick_low` kept. |
| S6 identical stepped pools | Four pools (`torch_pool_a`, `_b`, `_a_m`, `_b_m`): two sizes, an off-centre light mirrored in `_m`, a soot plume that leans either way and a ragged, noise-cut top edge. Bricks are re-shaded, not pixel-mirrored, so the bond stays on the wall grid. |
| S7 pier light cut off at its outer edge | The last 3 px of a pier's outer face fade; new `pier_spill_l/r` decals warm the bricks beside a pier that has no wall pool (B2 in the sample). Column-averaged luma at the B2 right pier edge: 114 → 99 → 58 → 70 (was 114 → 84 → 55 → 50). |
| S8 deck read as wall courses | Deck slab joints lean 1 px per 2 rows, like A1's receding floor. |
| Chain (open question) | Optional `arch_chain` structure piece (dry iron, A1 swag), used on B1 only. |
| Torch rhythm (open question) | Three pooled wall torches; C1/A2 east of x 1040 is an unlit stretch of about 590 px with a burnt-out sconce. |
| Platforms (open question) | Kept solid; S1/S2 make them read as raised floor. |
| Nice-to-haves | Tapered flame core; warm top rim on the rubble; door frame warmer on its torch side; `lichen_crust` is now a 24×8 muted crust and is left out of the sample layout (optional piece). Pilaster unchanged. |

## Contents

| Role (layer) | Atlas | Size, colours | Contents |
| --- | --- | --- | --- |
| `collision` | `floor01_terrain` (tileset) | 256×176, 24 colours | 176 opaque tiles: deep-floor panels by row type and tone (`deck_back_*` ×30, non-collidable; `deck_front_*` ×30, walking line at y 4; `face_a_*` ×18; `face_b_*` ×10), fill panels `fill_<level>_j<joint>_*` ×36 (levels 0–2 by absolute row, joint phase 0/16) and `fill_wall_l_<level>` ×3, 20 single edge tiles (`*_end_l/r` platform ends, `*_inner_l/r` lower floor beside a step, `*_wall_l` beside an end wall, for the four deck/face row types), `ceiling_fill_*` ×10, `cornice_*` ×9, `ceiling_corner_in/out_l/r`, `wall_end_l/r` (+`_b` quoin phase), `wall_fill`, `wall_fill_b` |
| `backwall` | `floor01_backwall` (tileset) | 272×96, 18 colours | 102 opaque tiles: five brick height bands (`brick_top/upper/mid/lower/low_*`, four 4-tile panels + two solo tiles each) and the pale `base_course_*` (two panels + four solos) |
| `backwall` decals, `foreground` | `floor01_walldecal` | 528×624, 18 colours | `torch_pool_a/_a_m` (radius 118) and `torch_pool_b/_b_m` (radius 102): whole-brick pools with soot and a ragged top, placed on a wall torch; `pier_spill_l/r` (64×176, beside a pier without a pool); `grime_a/b`; foreground `rubble_a/b` |
| `structure` | `floor01_structure` | 576×416, 26 colours | `arch_ring_a/b` (15 voussoirs, keystone, soffit, back aperture; carries the arch window and two cold gallery lights), `pier_l`, `pier_r` (impost, jamb, plinth, own torch light baked, outer edge faded), `arch_threshold`, optional `arch_chain`, `pilaster_plain` |
| `parallax_far`, `parallax_mid` | `floor01_parallax` | 448×464, 17 colours | `gallery_far` (440×232, opaque, 7 distant torch lights), `gallery_mid` (344×232, 1-bit alpha, 1 torch light) |
| `prop` | `floor01_props` | 256×120, 37 colours | `pipe_h`, `pipe_h_seam`, `pipe_v`, `pipe_v_seam`, `pipe_elbow_rd/ld/ru/lu` (32×32, centreline radius 16), `pipe_flange_v/h`, `pipe_bracket_v/h`, `pipe_collar_ceiling`, `pipe_collar_floor`, `valve_wheel`, `door_timber` (48×88 frame, 36×78 px timber leaf; Carl is 52 px), `torch_wall_unlit`, `lichen_crust` |
| `prop` (animated) | `floor01_torch` | 64×56, 8 colours | `torch_wall`: four frames, tag `burn`, 120/140/100/130 ms; flame and floor-pool emitters with a per-frame flicker curve |

Inventory: 278 tiles (176 collision: 35 `deck_back`, 35 walkable `deck_front`, 106 other; 102 backwall) and 38 pieces (19 prop, 7 structure, 8 backwall decal, 2 foreground, 1 `parallax_far`, 1 `parallax_mid`).

All pixels are 1-bit alpha. Collision-layer and backwall tiles are fully opaque. The h78/h60 look-dev cast redraws (`assets/source/lookdev/characters/`) are **not used** by this kit; actor checks use the approved Carl 52 / Donut 40 atlases.

## Palette `floor01-catacomb-v2`

One combined kit palette of 112 named colours, `<ramp>_<index>`: the A1 look-dev ramps (`brick` 12, `brick_cold` 4, `mortar` 7, `stone` 11, `stone_cold` 5, `floor` 11, `floor_cold` 4, `gal` 11, `gal_warm` 7, `copper` 9, `copper_cold` 2, `brass` 7, `iron` 7, `flame` 6) plus `wood` 6 and `lichen` 3. Index 0 is the deepest cold shadow and the last index the hottest light; shadows shift toward blue-violet and light toward yellow-orange.

Each atlas uses a subset of at most 63 opaque colours (the asset validator counts transparency as the 64th). Grid keys are atlas-local; grid palette entries must match a kit palette entry by name and RGBA. Per-atlas counts are in the Contents table. The lit frame is not palette-limited (6,968 colours at native 640×360), as ADR-0007 expects.

## Rows and A/B/C rules

- **Rows (base corridor, 25 rows):** 0 ceiling fill; 1 cornice; 2–3 `brick_top` (coldest); 4–7 `brick_upper`; 8–12 `brick_mid`; 13–14 `brick_lower`; 15–16 `brick_low` (the lit band behind actor heads); 17 `base_course` (pale dressed stone); 18 `deck_back` (floor back line y = 288); 19 `deck_front` (walking line y = 308, front lip y = 312); 20 `face_a`; 21 `face_b`; 22+ `fill`. A raised platform shifts its own floor stack up by 1 or 2 rows. Fill is chosen by absolute row (level 0 up to row 21, level 1 at row 22, level 2 below) with its joint phase set so joints sit at world x ≡ 16 × (row mod 2) mod 32; fill rows have no edge singles except `fill_wall_l_<level>` beside an end wall.
- **Panels:** within a row a panel cell is followed by its right neighbour; a panel's last column may be followed by column 0 of any panel of the same family; solo panels are one tile wide. Bricks and slabs cut by a panel edge use a neutral tone, so panels join without seams. Edge singles must touch panel edge columns.
- **A:** brick bands, base course, deck tones dim/mid/lit (lit under a torch, mid next to it). A wall `torch_wall` sits with its flame at y 178 (`mount_y.wall` = 160) and one of `torch_pool_a/_b/_a_m/_b_m` at (flame x − 120, 48). Keep at least 240 px between pooled torches so pools never overlap, and leave an unlit stretch of at least 400 px about every two screens. Occasional `grime_*` (mid band, away from pools), `door_timber`, `torch_wall_unlit` or one `pilaster_plain`.
- **B:** an arch bay is 304 px wide from a 16-px-aligned bay origin at y 32: `pier_l` (+0, 0), `arch_ring_a/b` (+16, 0), `pier_r` (+224, 0), `arch_threshold` (+64, 240), `torch_wall` on both piers at (+24, 176) and (+264, 176). Backwall cells touching the ring window stay empty. Piers bake their own torch light; do not add pools at pier torches. Where no wall pool is adjacent, add `pier_spill_l` at (bay − 48, 96) or `pier_spill_r` at (bay + 288, 96). `arch_chain` at (+80, 80) is optional; use it on some bays only.
- **C:** one copper run (elbows, straight pieces, at least one flange and one bracket, a `valve_wheel` on a vertical drop). Every end terminates in `pipe_collar_ceiling` or `pipe_collar_floor`. `lichen_crust` is optional (not used in the sample) and only next to a pipe joint. Copper only in C; a test enforces it for the sample layout.

## Consumer contract (COMP-01 and any other loader)

Everything a runtime needs to load the kit, in one place. The longer v2 rules follow in [Integration contract v2](#integration-contract-v2-for-comp-01). The contract (`env-kit-v2`) and layout (`env-kit-layout-v2`) are IP-neutral data; the Bevy loader is not written yet.

**Coordinates.** Everything is in virtual pixels, origin top-left, **x right, y down**, tile rows top to bottom (`coordinate_system` in the contract). Layout pixel (x, y) = (col × 16, row × 16). Atlas rects are `[x, y, w, h]` from the atlas top-left. Piece `x`/`y` in a layout is the top-left of the piece rect. **Bevy is y-up: flip y once at load time** (`world_y = layout_height − y`, remembering that a rect's world origin is its bottom edge). Never flip inside the data.

**Collision rects.** Only tiles in the `collision` layer with `collidable: true` produce colliders. For the tile at (col, row) with tile-local `collider: [cx, cy, cw, ch]`, the solid rect is `[col × 16 + cx, row × 16 + cy, cw, ch]`. A `deck_front` tile has `[0, 4, 16, 12]`, every other collidable tile `[0, 0, 16, 16]`. Merge rects per row or per chunk as needed (see `destruction.md`). `deck_back` tiles are visual only (`collidable: false`, no `collider`). Backwall tiles, every piece (props, structure, decals, parallax) and every light are never collidable.

**Walking line.** The standing surface of a floor column is `row × 16 + walk_line_y` of its single `walkable_top` tile (`deck_front`, `walk_line_y: 4`), which equals the top of its collider. The base corridor walks at y = 308 (`base_walk_line_y`); platforms are 16 or 32 px higher. A column that is collidable from the top row to the bottom row is an end wall, not floor. Actors are placed with their pivot (bottom feet row) at `walk_line − 1` (0 px sink); `spawn` and `exit` markers already store that y. Validation guarantees: one walking surface per floor column, steps of at most 2 tiles (32 px), no pits, no floating colliders.

**Draw order (z).** Lower draws first. Use these as the sort key (or map to Bevy z in steps of 1).

| Order | Layer | Kind | Light group | Notes |
| --- | --- | --- | --- | --- |
| 0 | `parallax_far` | strip, factor 0.35 | `parallax_far` | opaque gallery, seen through arch windows |
| 1 | `parallax_mid` | strip, factor 0.6 | `parallax_mid` | 1-bit alpha, over the far strip |
| 2 | `backwall` | tiles | `scene` | opaque brick; empty inside arch windows |
| 3 | `backwall_decal` | pieces | `scene` | torch pools, pier spills, grime: over the brick, under the floor |
| 4 | `collision` | tiles | `scene` | floor, ceiling, end walls, `deck_back` |
| 5 | `structure` | pieces | `scene` | arch rings, piers, threshold, optional chain, pilaster |
| 6 | `prop` | pieces | `scene` | torches (animated), pipes, door, lichen |
| 7 | actors | n/a | `actors` | Carl, Donut, enemies |
| 8 | `foreground` | pieces | `foreground` | rubble silhouettes over actors |

**Parallax.** Strips repeat horizontally forever (`tile_x: true`, seamless wrap). A strip pixel at strip-local u is drawn at `screen_x = x_origin + k × strip_width + u − camera_x × factor` for every integer k, then snapped to whole virtual pixels; `camera_x` is the view's left edge in layout px and y is fixed (`parallax_layers.<name>.y`). `x_origin` is a phase in `[0, strip_width)`. Validation proves the far strip fills every arch window with the camera clamped at x = 0 and at the right end, for 640×360 and 640×400. The loader must keep repeating the strip so the camera can never see an empty edge.

**Lights (presentation only).** Emitters live on pieces: `lights: [{id, offset, colour, radius, intensity, group, glow?, flicker?, project?, project_dy?, squash_y?}]`. `offset` is piece-local px from the piece top-left (for a parallax strip, strip-local; it repeats with the strip). `colour` is sRGB `#rrggbb`. `radius` and `glow.radius` are falloff radii in virtual px; `intensity` is a unitless multiplier into the group's light map (0..4); `glow.additive` is the 0..1 strength of the halo. `flicker` is one multiplier (0..2) per frame of the piece's animation, in animation order (torch: 4 frames, 120/140/100/130 ms). `project: "walk_line"` moves the light straight down to the walking line of its column plus `project_dy` px (floor pools); `squash_y` (0.05..1) flattens the falloff vertically. `group` is the light group the emitter lights (`scene`, `foreground`, or the strip's own parallax group). The layout `lighting` profile gives per-group `ambient` [r, g, b] multipliers (0..2), optional `haze` (`colour`, `amount`, `floor_fog`), `light_scale` for actors/foreground, a contact shadow and bloom settings. The pass runs at output resolution on top of the integer-scaled pixel layers (ADR-0007).

**Presentation-only data.** All of `lights`, `lighting`, `light_group`, `parallax`, `window`, the `preview` block and the `status`/`migration`/`note` fields are rendering or review data. Validation rejects any collision or gameplay section (`tiles`, `markers`, `segments`, `requirements`, `walk_line`) that references lighting, and a lighting profile that carries anything but ambient, haze, shadow and bloom values. Collision, AI and damage must never read them. The `preview` block (actor atlases, target frame) is an art-review input, not runtime data.

**Committed vs generated.** Committed (source of truth): the contract and layout JSON, the pixel grids and `.ase` files under `assets/source/terrain/floor01/` and `assets/source/props/floor01/`, the export manifests under `assets/manifests/floor01_*.json` and the review evidence under `docs/evidence/floor01-catacomb-kit/`. **Generated and gitignored** (run `bash scripts/export_assets.sh`): the atlas PNGs and metadata JSON named by the contract (`generated/art/floor01_*.png` / `.json`; the contract references these paths) and the unlit previews in `generated/art/env_kit/`. A loader must read atlas rects from the contract and animation timing from the contract (copied from the exported metadata), not hard-code numbers.

## Integration contract v2 (for COMP-01)

`floor01_catacomb_kit.json` is now `env-kit-v2` and the sample layout `env-kit-layout-v2`. Both carry a `migration` note. No v1 id keeps its meaning, so v1 files are rejected rather than silently reinterpreted (see [Migration from v1](#migration-from-v1)).

### Walking line and colliders

- A walkable tile declares `walk_line_y` (tile-local, 0–15). The collider top of a floor column is `row * 16 + walk_line_y`. `deck_front` tiles use `walk_line_y: 4`.
- Every collidable tile has `collider: [x, y, w, h]` (tile-local) that must equal `[0, walk_line_y, 16, 16 − walk_line_y]`; full tiles use `[0, 0, 16, 16]`. **Bevy:** for each collidable cell spawn (or merge per row) a static AABB at `(col * 16 + x, row * 16 + y, w, h)`, flipping y for y-up.
- `deck_back` tiles are opaque, drawn in the collision layer, and **not collidable**: they are the back 16 px of the 24 px floor surface, behind the walking line. Every walkable tile must have a `deck_back` above it.
- Actors are drawn with their pivot row at `collider_top − 1` (0 px sink). Spawn and exit markers use the same row. A column collidable from top to bottom is a wall, not floor. Steps are limited to `max_step_tiles` (2) × 16 px of walking-line difference.

### Layers and parallax

Draw order: `parallax_far` (0) → `parallax_mid` (1) → `backwall` (2) → `backwall_decal` (3) → `collision` (4) → `structure` (5) → `prop` (6) → actors (7) → `foreground` (8). Collision now draws before structure and props, so pier plinths and pipe collars sit on the deck. Every layer names a `light_group`.

Parallax layers have `kind: "parallax"` and a factor (far 0.35, mid 0.6). Their pieces declare `tile_x: true`. The layout places each strip with `parallax_layers.<name> = {piece, x_origin, y}`. A strip pixel at strip-local u is drawn at screen x = `x_origin + k × width + u − camera_x × parallax` for every integer k, snapped to whole virtual pixels. The sample's `x_origin` values make the primary view match the A1 frame.

### Arch windows

`arch_ring_*` declares `window: {arc_centre, arc_radius, rect}` in piece-local px: the circle above the arc centre line plus the rectangle down to the floor back line. Rule: no backwall tile, collision tile or decal pixel inside a window; every transparent pixel of backwall + decals + collision + structure lies inside a window; the far strip covers every window vertically and, rendered with the camera clamped at both ends of the corridor, leaves no empty pixel behind a window.

### Lights (presentation only)

Pieces carry `lights: [{id, offset, colour, radius, intensity, glow?, flicker?, project?, project_dy?, squash_y?, group}]`. Examples: the torch flame (radius 250, intensity 0.36, glow radius 44, flicker 1.0/0.93/1.07/0.97), the torch floor pool (`project: "walk_line"`, `project_dy: −6`, `squash_y: 0.32`), the arch gallery bounce and sky lights, and the gallery strip torches (which move with their strip). The layout's `lighting` profile sets per-group ambient (`parallax_far`, `parallax_mid`, `scene`, `actors`, `foreground`), haze with ground fog for the parallax groups, the actor `light_scale`, an actor contact shadow and bloom (emissive layers `prop`, `parallax_mid`, `parallax_far`, restricted to the atlas colours in `emissive_colours`, i.e. flame pixels). It must declare `presentation_only: true`. Lights never change collision or gameplay.

### Sample layout

`assets/levels/floor01/corridor_abc_sample.json`: 128×25 tiles (2048×400), segments A1 (0–512), B1 (512–816), C1 (816–1280), A2 (1280–1600), B2 (1600–2048). It has a left end wall, a continuous walkable floor with a 1-tile platform in A1 and a 2-tile platform in A2, two arch bays (chain on B1, pier spills on B2), three pooled wall torches, four pier torches, two burnt-out sconces in an unlit stretch (x 1040–1632), one copper run in C1, a door, two pilasters, three grime patches and two rubble silhouettes. View `abc_640x360` (x 328, y 20) reproduces the A1 composition with kit pieces; `abc_640x400` is the 16:10 framing.

## Migration from v1

`env-kit-v1` contracts and `env-kit-layout-v1` layouts are **rejected** with a pointer to this section; no v1 tile or piece id keeps its meaning, so there is no automatic converter. To move a v1 kit or layout:

1. **Palette:** replace the shared 64-colour palette with a named kit palette; each atlas grid uses a subset of at most 63 opaque colours whose names and RGBA match the kit palette.
2. **Floor:** the collision surface is now the walking line, not the tile top. Mark the walkable tile `walkable_top` with `walk_line_y`, give every collidable tile a `collider` rect (`[0, walk_line_y, 16, 16 − walk_line_y]`), put a non-collidable `deck_back` tile above every walkable tile, and replace `base_floor_top_y` with `base_walk_line_y`. Markers move to `walk_line − 1`.
3. **Layers:** add the `parallax_far` and `parallax_mid` layers (kind `parallax`, factors in (0, 1), increasing with draw order, pieces with `tile_x`), give every layer a `light_group`, and draw `collision` before `structure` and `prop`. The layout gains `parallax_layers` entries `{piece, x_origin, y}`.
4. **Arch openings:** delete `recess_*` backwall tiles and `far_arcade_*` pieces; declare a `window` on the arch ring and leave the backwall empty inside it.
5. **Lights:** add `lights` to pieces and a `presentation_only` `lighting` profile to the layout. Both are optional for gameplay and mandatory for the validator.
6. Set `format` to `env-kit-v2` / `env-kit-layout-v2` and run `python3 scripts/env_kit.py validate --contract <kit> --layout <layout>`.

The machine-readable per-file notes are in the `migration` fields of the contract and the layout.

## Lighting pass and previews

- `scripts/env_kit.py` validates contracts and layouts and renders **unlit** previews from the exported atlases (overview, views, contact sheets, unlit vs A1, actor check). `scripts/export_assets.sh` runs it.
- `scripts/env_kit_light.py` adds the A1 lighting model on the same layer images at an integer output scale, using the A1 compositor's falloff sprites, bloom and readability metric (`scripts/lookdev_compose.py`). Parallax glows and bloom are masked by nearer opaque layers. It writes the lit evidence and `measurements.json`. It is not run by the export (the evidence is committed).

## Validation

`python3 scripts/env_kit.py all` (part of the export) fails on, among others:

- **Format:** any format other than `env-kit-v2` / `env-kit-layout-v2`; v1 files get a message that points to [Migration from v1](#migration-from-v1).
- **Pixels:** a grid colour outside the kit palette or more than 63 colours per atlas; partial alpha; a non-opaque collision or backwall tile; a far parallax strip that is not fully opaque.
- **Walking line and colliders:** a collidable `deck_back` or backwall tile; a `walkable_top` flag that is not a boolean; `walk_line_y` outside 0..15 or on a non-walkable tile; a collider that is not exactly `[0, walk_line_y, 16, 16 − walk_line_y]` in integers; a floor column without exactly one walking surface on its top tile, a walkable tile in a ceiling or wall column, a walkable tile without `deck_back` directly above it, or a `deck_back` tile without a walkable tile below; a pit, a floating collision run, a third run in a column, or a step over 2 tiles (`max_step_tiles` is capped at 2 and `continuous_floor` cannot be switched off); a `base_walk_line_y` that is not the most common walking line or differs from the contract's `walk_line.base_floor`; a spawn, exit or preview actor that is not on a walking line (y = walking line − 1 for markers).
- **Parallax:** a factor outside the open interval (0, 1) or not increasing with depth; a strip that is animated, narrower than 32 px, or whose last and first columns differ in more rows than the 90th percentile of interior neighbouring columns (the wrap seam must be as smooth as any other column); an `x_origin` outside `[0, strip_width)`; a strip outside the layout height; a far strip that fails to cover a window vertically; a far strip that leaves an arch window empty with the camera clamped at x = 0 or at the right end (640×360, 640×400 and every declared view); windows without a far layer.
- **Arch windows:** malformed `window` declarations (non-integer arc, radius or rect, or on a non-structure piece, or reaching outside the layout); a backwall tile, a collision tile or a decal pixel inside a window; transparent pixels outside every window.
- **Lights:** unknown fields; a bad colour, offset, radius (1..1024), intensity (0..4), glow, flicker value (0..2) or `squash_y`; a flicker curve whose length differs from the piece's animation frame count, or on a static piece; a light on the `actors` group; a parallax strip lighting a group other than its own, or a non-parallax piece lighting a parallax group; a free-floating layout light or an instance carrying lights; a walking-line projection over a column with no walking line.
- **Presentation only:** a contract or layout lighting block that is not `presentation_only`; a lighting profile with fields other than ambient, haze, shadow and bloom values; any collision or gameplay section that references lighting.
- **Layout:** broken panel continuity or a misplaced edge tile; a backwall hole; an off-grid piece or a `fixed_y` piece elsewhere; segments that are not contiguous multiples of 16 px; missing spawn or exit markers.

`tests/test_art_env_kit.py` proves each rule fires on a mutated copy, plus deterministic unlit and lit renders (including the actor-check previews), parallax motion at the declared factor, copper-only-in-C, the world-phase fill bond and flame-only bloom. CI hashes every file under `generated/art` (recursively) after two exports and requires a clean working tree.

## Regenerating

```bash
python3 scripts/author_floor01_kit.py --force      # grids, .ase, manifests, contract, layout (overwrites hand edits)
ART_CLI=/path/to/aseprite-or-libresprite bash scripts/export_assets.sh
python3 scripts/env_kit_light.py --out docs/evidence/floor01-catacomb-kit
```

`scripts/author_floor01_kit.py` imports the A1 look-dev shading core (`scripts/lookdev_floor01_author.py`): light evaluation with a windowed falloff quantised onto the ramps, per-brick/per-block shading, bevels, pits and chips. The arch, piers, threshold and gallery strips re-render the A1 look-dev geometry as modules with a local light rig; the gallery strips use periods that tile (440 and 344 px). The committed grids are the source of truth.

## Self-review against the owner's five art criteria (agent, before human review)

Measured on the lit 640×360 frame (`measurements.json`), with the A1 frame and the same actor positions as the reference. Round 3 values in brackets.

| Metric | Kit, lit | A1 target |
| --- | --- | --- |
| Carl silhouette edge mean \|ΔL*\| | **13.9** [15.3] | 13.1 |
| Carl edge by height band (head → feet) | 14.9 / 12.9 / 12.5 / **19.2** / 14.3 / 11.9 | 9.9 / 14.9 / 14.1 / 11.1 / 16.5 / 12.0 |
| Carl mean ΔL* (actor − 6 px ring) / contrast ratio | 8.6 / 1.81 [6.1 / 1.64] | 18.2 / 2.35 |
| Donut silhouette edge mean \|ΔL*\| | 29.2 [33.5] | 33.0 |
| Donut mean ΔL* / contrast ratio | 14.0 / 2.25 [11.5 / 2.04] | 16.3 / 2.00 |
| Bottom 25 px of the view, mean L* / b* | 2.8 / −8.0 [9.8 / −18.4] | 5.6 / −13.2 |
| Base course beside Carl, mean L* / b* | 33.1 / 7.6 [43.1 / 17.1] | 17.5 / −7.1 |

1. **Mood match vs A1:** the side-by-side reads as the same world: warm pools on orange brick, cold violet wall top, glowing flames, hazy blue gallery with distant torch dots and the chain swag, copper catching torchlight, a lit receding deck and a dark ledge base. The baked pools are slightly tighter than A1's (they must not overlap).
2. **Element likeability:** round-4 fixes cover the bottom band, pipes, valve bloom, pools, pier edges and deck. The door, pilaster, rubble and the optional lichen still need the owner's eye.
3. **Readability:** darkening the base course (S5) traded part of the jacket-height gain (28.7 → 19.2, still above A1's 11.1) for a better overall Carl ΔL* (6.1 → 8.6). Legs and feet (14.3 / 11.9) remain the weakest Carl edges, close to A1. `brick_low` is kept.
4. **Grammar and locks:** A brick with pools and an unlit stretch, B arch bays with the gallery, C one copper run. Dry, clean, flat; no sewer, toxic, factory or TV look; copper only in C.
5. **Display check:** integer nearest scaling only; panels and fill courses join without seams; three pools of different shape across 2048 px. No Bevy, 4K or Deck hardware check was possible here.

**Iterations:** round 3 (three passes) is summarised in the PR history: bright face course, grime holes, dark head/torso backdrop, gallery glow and bloom leaking through walls, over-bright copper, identical pools. Round 4 (two passes after the review): the first pass darkened the base course by about 10 L*, which dropped Carl's jacket edge to 17.8 and drew a dotted cool line at the course bottom, and made the lichen a green blob. The second pass settled the base course at −10 L* without the dither, widened the contact shadow, made the fill cooler (b* −6 → −8) and removed the lichen from the sample.

## Open questions for the owner

1. Is the darker base course (S5) the right balance between Carl's jacket edge and his legs/shorts?
2. Keep the optional chain on some bays, or none?
3. Is the unlit stretch in C1/A2 long enough, or should A1 also get one?

## ENV-02 follow-ups (listed, not built)

The opening-route pieces (chase passage, CP-03 breakable, guild exit) are designed in the [ENV-02 spec](env-02-opening-spec.md) as an `extends` contract on this kit; the items below remain open.

- More brick panel variants and an extra pool size if Bevy shows visible repetition.
- Pier and arch variants for adjacent bays (a shared double-jamb pier).
- Bevy capture with the real lighting pass (COMP-01), 4K and Deck hardware checks.

## Rights and provenance

Original pixel art authored by Claude Opus with deterministic scripts; no image-model raster output, no crops, traces or resamples of concept images. The A1 look-dev frame (same project, same agent) is the target; the concept image `01-catacomb-environment-clean.png` was a mood reference for A1 only and is never read. Actor checks composite the approved Carl 52 and Donut 40 atlases under the owner-reported `ART-PUBLICATION-ALL-V1` scope in [`rights-and-release.md`](../04-production/rights-and-release.md). That scope is not independently verified, is not a public reuse licence and grants no game or asset-pack distribution right. Manifests stay `prototype_fixture_unapproved`.
