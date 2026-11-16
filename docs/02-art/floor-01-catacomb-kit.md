# Floor 01 catacomb environment kit (ENV-01)

**Status:** rework round 3, proposed, awaiting human art review. Nothing here is approved production art, level geometry or Bevy evidence.
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

## Contents

| Role (layer) | Atlas | Size, colours | Contents |
| --- | --- | --- | --- |
| `collision` | `floor01_terrain` (tileset) | 256×144, 26 colours | 144 opaque tiles: deep-floor panels by row type and tone (`deck_back_*` ×30, non-collidable; `deck_front_*` ×30, walking line at y 4; `face_a_*` ×18; `face_b_*` ×10; `fill_*` ×6), 25 single edge tiles (`*_end_l/r` platform ends, `*_inner_l/r` lower floor beside a step, `*_wall_l` beside an end wall, for all five row types), `ceiling_fill_*` ×6, `cornice_*` ×9, `ceiling_corner_in/out_l/r`, `wall_end_l/r` (+`_b` quoin phase), `wall_fill`, `wall_fill_b` |
| `backwall` | `floor01_backwall` (tileset) | 272×96, 18 colours | 102 opaque tiles: five brick height bands (`brick_top/upper/mid/lower/low_*`, four 4-tile panels + two solo tiles each) and the pale `base_course_*` (two panels + four solos) |
| `backwall` decals, `foreground` | `floor01_walldecal` | 512×256, 18 colours | `torch_pool_a` (radius 118) / `torch_pool_b` (radius 102): whole-brick pools with soot, placed on a wall torch; `grime_a/b`; foreground `rubble_a/b` |
| `structure` | `floor01_structure` | 576×416, 24 colours | `arch_ring_a/b` (15 voussoirs, keystone, soffit, back aperture; carries the arch window and two cold gallery lights), `pier_l`, `pier_r` (impost, jamb, plinth, own torch light baked), `arch_threshold`, `pilaster_plain` |
| `parallax_far`, `parallax_mid` | `floor01_parallax` | 448×464, 17 colours | `gallery_far` (440×232, opaque, 7 distant torch lights), `gallery_mid` (344×232, 1-bit alpha, 1 torch light) |
| `prop` | `floor01_props` | 256×120, 36 colours | `pipe_h`, `pipe_h_seam`, `pipe_v`, `pipe_v_seam`, `pipe_elbow_rd/ld/ru/lu` (32×32, centreline radius 16), `pipe_flange_v/h`, `pipe_bracket_v/h`, `pipe_collar_ceiling`, `pipe_collar_floor`, `valve_wheel`, `door_timber` (48×88 frame, 36×78 px timber leaf; Carl is 52 px), `torch_wall_unlit`, `lichen_crust` |
| `prop` (animated) | `floor01_torch` | 64×56, 8 colours | `torch_wall`: four frames, tag `burn`, 120/140/100/130 ms; flame and floor-pool emitters with a per-frame flicker curve |

Inventory: 246 tiles (144 collision: 35 `deck_back`, 35 walkable `deck_front`, 74 other; 102 backwall) and 33 pieces (19 prop, 6 structure, 4 backwall decal, 2 foreground, 1 `parallax_far`, 1 `parallax_mid`).

All pixels are 1-bit alpha. Collision-layer and backwall tiles are fully opaque. The h78/h60 look-dev cast redraws (`assets/source/lookdev/characters/`) are **not used** by this kit; actor checks use the approved Carl 52 / Donut 40 atlases.

## Palette `floor01-catacomb-v2`

One combined kit palette of 112 named colours, `<ramp>_<index>`: the A1 look-dev ramps (`brick` 12, `brick_cold` 4, `mortar` 7, `stone` 11, `stone_cold` 5, `floor` 11, `floor_cold` 4, `gal` 11, `gal_warm` 7, `copper` 9, `copper_cold` 2, `brass` 7, `iron` 7, `flame` 6) plus `wood` 6 and `lichen` 3. Index 0 is the deepest cold shadow and the last index the hottest light; shadows shift toward blue-violet and light toward yellow-orange.

Each atlas uses a subset of at most 63 opaque colours (the asset validator counts transparency as the 64th). Grid keys are atlas-local; grid palette entries must match a kit palette entry by name and RGBA. Per-atlas counts are in the Contents table. The lit frame is not palette-limited (6,968 colours at native 640×360), as ADR-0007 expects.

## Rows and A/B/C rules

- **Rows (base corridor, 25 rows):** 0 ceiling fill; 1 cornice; 2–3 `brick_top` (coldest); 4–7 `brick_upper`; 8–12 `brick_mid`; 13–14 `brick_lower`; 15–16 `brick_low` (the lit band behind actor heads); 17 `base_course` (pale dressed stone); 18 `deck_back` (floor back line y = 288); 19 `deck_front` (walking line y = 308, front lip y = 312); 20 `face_a`; 21 `face_b`; 22+ `fill`. A raised platform shifts its own floor stack up by 1 or 2 rows.
- **Panels:** within a row a panel cell is followed by its right neighbour; a panel's last column may be followed by column 0 of any panel of the same family; solo panels are one tile wide. Bricks and slabs cut by a panel edge use a neutral tone, so panels join without seams. Edge singles must touch panel edge columns.
- **A:** brick bands, base course, deck tones dim/mid/lit (lit under a torch, mid next to it). A wall `torch_wall` sits with its flame at y 178 (`mount_y.wall` = 160) and its `torch_pool_a/b` at (flame x − 120, 48). Keep at least 240 px between pooled torches so pools never overlap. Occasional `grime_*` (mid band, away from pools), `door_timber`, `torch_wall_unlit` or one `pilaster_plain`.
- **B:** an arch bay is 304 px wide from a 16-px-aligned bay origin at y 32: `pier_l` (+0, 0), `arch_ring_a/b` (+16, 0), `pier_r` (+224, 0), `arch_threshold` (+64, 240), `torch_wall` on both piers at (+24, 176) and (+264, 176). Backwall cells touching the ring window stay empty. Piers bake their own torch light; do not add pools at pier torches.
- **C:** one copper run (elbows, straight pieces, at least one flange and one bracket, a `valve_wheel` on a vertical drop). Every end terminates in `pipe_collar_ceiling` or `pipe_collar_floor`. `lichen_crust` is optional and only next to a pipe joint. Copper only in C; a test enforces it for the sample layout.

## Integration contract v2 (for COMP-01)

`floor01_catacomb_kit.json` is now `env-kit-v2` and the sample layout `env-kit-layout-v2`. Both carry a `migration` note. No v1 id keeps its meaning, so v1 files are rejected rather than silently reinterpreted.

### Walking line and colliders

- A walkable tile declares `walk_line_y` (tile-local, 0–15). The collider top of a floor column is `row * 16 + walk_line_y`. `deck_front` tiles use `walk_line_y: 4`.
- Every collidable tile has `collider: [x, y, w, h]` (tile-local) that must equal `[0, walk_line_y, 16, 16 − walk_line_y]`; full tiles use `[0, 0, 16, 16]`. **Bevy:** for each collidable cell spawn (or merge per row) a static AABB at `(col * 16 + x, row * 16 + y, w, h)`, flipping y for y-up.
- `deck_back` tiles are opaque, drawn in the collision layer, and **not collidable**: they are the back 16 px of the 24 px floor surface, behind the walking line. Every walkable tile must have a `deck_back` above it.
- Actors are drawn with their pivot row at `collider_top − 1` (0 px sink). Spawn and exit markers use the same row. A column collidable from top to bottom is a wall, not floor. Steps are limited to `max_step_tiles` (2) × 16 px of walking-line difference.

### Layers and parallax

Draw order: `parallax_far` (0) → `parallax_mid` (1) → `backwall` (2) → `backwall_decal` (3) → `collision` (4) → `structure` (5) → `prop` (6) → actors (7) → `foreground` (8). Collision now draws before structure and props, so pier plinths and pipe collars sit on the deck. Every layer names a `light_group`.

Parallax layers have `kind: "parallax"` and a factor (far 0.35, mid 0.6). Their pieces declare `tile_x: true`. The layout places each strip with `parallax_layers.<name> = {piece, x_origin, y}`. A strip pixel at strip-local u is drawn at screen x = `x_origin + k × width + u − camera_x × parallax` for every integer k, snapped to whole virtual pixels. The sample's `x_origin` values make the primary view match the A1 frame.

### Arch windows

`arch_ring_*` declares `window: {arc_centre, arc_radius, rect}` in piece-local px: the circle above the arc centre line plus the rectangle down to the floor back line. Rule: no backwall tile or decal pixel inside a window; every transparent pixel of backwall + decals + collision + structure lies inside a window; the far strip covers every window vertically.

### Lights (presentation only)

Pieces carry `lights: [{id, offset, colour, radius, intensity, glow?, flicker?, project?, project_dy?, squash_y?, group}]`. Examples: the torch flame (radius 250, intensity 0.36, glow radius 44, flicker 1.0/0.93/1.07/0.97), the torch floor pool (`project: "walk_line"`, `project_dy: −6`, `squash_y: 0.32`), the arch gallery bounce and sky lights, and the gallery strip torches (which move with their strip). The layout's `lighting` profile sets per-group ambient (`parallax_far`, `parallax_mid`, `scene`, `actors`, `foreground`), haze with ground fog for the parallax groups, the actor `light_scale`, an actor contact shadow and bloom (emissive layers `prop`, `parallax_mid`, `parallax_far`). It must declare `presentation_only: true`. Lights never change collision or gameplay.

### Sample layout

`assets/levels/floor01/corridor_abc_sample.json`: 128×25 tiles (2048×400), segments A1 (0–512), B1 (512–816), C1 (816–1280), A2 (1280–1600), B2 (1600–2048). It has a left end wall, a continuous walkable floor with a 1-tile platform in A1 and a 2-tile platform in A2, two arch bays, five pooled wall torches, four pier torches, an unlit sconce, one copper run in C1, a door, two pilasters, two grime patches and two rubble silhouettes. View `abc_640x360` (x 328, y 20) reproduces the A1 composition with kit pieces; `abc_640x400` is the 16:10 framing.

## Lighting pass and previews

- `scripts/env_kit.py` validates contracts and layouts and renders **unlit** previews from the exported atlases (overview, views, contact sheets, unlit vs A1, actor check). `scripts/export_assets.sh` runs it.
- `scripts/env_kit_light.py` adds the A1 lighting model on the same layer images at an integer output scale, using the A1 compositor's falloff sprites, bloom and readability metric (`scripts/lookdev_compose.py`). Parallax glows and bloom are masked by nearer opaque layers. It writes the lit evidence and `measurements.json`. It is not run by the export (the evidence is committed).

## Validation

`python3 scripts/env_kit.py all` (part of the export) fails on, among others: wrong format version; a grid colour outside the kit palette or more than 63 colours per atlas; partial alpha; a non-opaque collision or backwall tile; a collidable `deck_back` or backwall tile; a collider that does not follow the walking line; a walkable tile without `deck_back` above it; a pit or a step over 32 px; broken panel continuity or a misplaced edge tile; a backwall tile or decal inside an arch window; transparent pixels outside every window; a far strip that does not cover a window; bad light metadata (colour, radius, group, flicker length); a parallax piece without `tile_x`; an off-grid piece or a `fixed_y` piece elsewhere; a marker off the walking line; an incomplete or non-presentation-only lighting profile. `tests/test_art_env_kit.py` (54 tests) proves each rule fires, plus deterministic unlit and lit renders, parallax motion at the declared factor and copper-only-in-C.

## Regenerating

```bash
python3 scripts/author_floor01_kit.py --force      # grids, .ase, manifests, contract, layout (overwrites hand edits)
ART_CLI=/path/to/aseprite-or-libresprite bash scripts/export_assets.sh
python3 scripts/env_kit_light.py --out docs/evidence/floor01-catacomb-kit
```

`scripts/author_floor01_kit.py` imports the A1 look-dev shading core (`scripts/lookdev_floor01_author.py`): light evaluation with a windowed falloff quantised onto the ramps, per-brick/per-block shading, bevels, pits and chips. The arch, piers, threshold and gallery strips re-render the A1 look-dev geometry as modules with a local light rig; the gallery strips use periods that tile (440 and 344 px). The committed grids are the source of truth.

## Self-review against the owner's five art criteria (agent, before human review)

Measured on the lit 640×360 frame (`measurements.json`), with the A1 frame and the same actor positions as the reference.

| Metric | Kit, lit | A1 target |
| --- | --- | --- |
| Carl silhouette edge mean \|ΔL*\| | **15.3** | 13.1 |
| Carl edge at jacket height (band 3 of 6) | **28.7** | 11.1 |
| Carl edge at head / feet (bands 0 / 5) | 14.9 / 10.4 | 9.9 / 12.0 |
| Carl mean ΔL* (actor − 6 px ring) / contrast ratio | 6.1 / 1.64 | 18.2 / 2.35 |
| Donut silhouette edge mean \|ΔL*\| | **33.5** | 33.0 |
| Donut mean ΔL* / contrast ratio | 11.5 / 2.04 | 16.3 / 2.00 |

1. **Mood match vs A1:** the side-by-side reads as the same world: warm pools on orange brick, cold violet wall top, glowing flames, hazy blue gallery with distant torch dots, copper pipe catching torchlight, lit deep floor. Differences: the kit's pools are slightly tighter (they are baked per torch and must not overlap), the area between pools is a little cooler, and there is no chain across the arch.
2. **Element likeability:** pipes are thick copper with brass fittings; brick is warm and hue-shifted; the arch background is the layered gallery. The door, pilaster and lichen are new and need owner review.
3. **Readability:** the weak edge named in the spike (Carl's dark jacket against a dark wall base) is fixed: jacket-height edge contrast rises from 11.1 to 28.7 and the mean silhouette edge from 13.1 to 15.3. The cost is a **lower mean ΔL*** for Carl (6.1 vs 18.2), because the lighter band behind actor height brings the surround close to Carl's mid-value average (dark jacket, light legs). The silhouette reads by local contrast, not by being darker or lighter overall. Feet are the weakest Carl edge (10.4); a presentation-only contact shadow helps.
4. **Grammar and locks:** A plain brick with pools, B arch bays with the gallery, C a single copper run with a valve. The floor is dry, clean and flat; there is no sewer, toxic, factory or TV look; copper appears only in C.
5. **Display check:** all previews use integer nearest scaling (1080p ×3, Deck 640×400 ×2). Panels join without seams. Repetition: five brick bands × six panels, two pool sizes, irregular torch spacing (304/480/256/240 px) and two arch-ring variants. Five pools along 2048 px still read as a regular torch rhythm. No Bevy or 4K hardware check was possible here.

**Iterations (round 3):**

1. First kit render: the face course under the lip was a bright orange band, grime patches read as holes inside pools, and only one distant torch showed through the arch. Fixed: darker face courses, grime moved out of pools and softened, more gallery torches and strip origins that match A1.
2. First lit render: Carl's head and jacket sat against dark brick (edge 11.7). The base course read grey-violet under the cold ambient. Distant gallery torches glowed and bloomed through the wall. Fixed: a `brick_low` band at actor head height and a paler base course, a contact shadow, and parallax glow and bloom masked by nearer layers.
3. Second lit render: the copper pipe over-brightened under the torch pass ("neon") and all pools had the same size. Fixed: darker baked copper and a softer valve rig; a second, smaller pool variant used irregularly.

## Open questions for the owner

1. Readability trade-off: keep the lighter band behind actor height (better silhouette edges, lower mean ΔL*), or go back toward A1's darker wall base?
2. Should a gallery chain or other dressing return in B, or stay clean?
3. Is the five-torch rhythm along the sample corridor acceptable, or should A use fewer torches with unlit stretches?
4. Platform faces show a darker block down to the bottom of the view. Is that acceptable, or should steps be shallower decks only?

## ENV-02 follow-ups (listed, not built)

- More brick panel variants and an extra pool size if Bevy shows visible repetition.
- Pier and arch variants for adjacent bays (a shared double-jamb pier).
- Bevy capture with the real lighting pass (COMP-01), 4K and Deck hardware checks.

## Rights and provenance

Original pixel art authored by Claude Opus with deterministic scripts; no image-model raster output, no crops, traces or resamples of concept images. The A1 look-dev frame (same project, same agent) is the target; the concept image `01-catacomb-environment-clean.png` was a mood reference for A1 only and is never read. Actor checks composite the approved Carl 52 and Donut 40 atlases under the owner-reported `ART-PUBLICATION-ALL-V1` scope in [`rights-and-release.md`](../04-production/rights-and-release.md). That scope is not independently verified, is not a public reuse licence and grants no game or asset-pack distribution right. Manifests stay `prototype_fixture_unapproved`.
