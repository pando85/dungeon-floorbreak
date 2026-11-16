# Floor 01 catacomb environment kit (ENV-01)

**Status:** proposed, awaiting human art review. Nothing here is approved production art, level geometry or Bevy evidence.
**Work order:** ENV-01 in the [local-agent handoff](../07-agents/floor-01-opening-encounter-handoff.md). The next step is COMP-01, which renders this kit in Bevy with the approved Carl 52 px and Donut 40 px atlases.
**Binding direction:** [Floor 01 opening](floor-01-opening-catacomb-goblins.md): environment grammar A/B/C and its environment locks.
**Evidence:** [`docs/evidence/floor01-catacomb-kit/`](../evidence/floor01-catacomb-kit/README.md)

## Purpose

This is a small, modular, editable pixel-art kit for the calm first corridors of Floor 1. It contains:

- a plain brick corridor (A);
- occasional masonry arches and piers with cold blue depth (B);
- one restrained maintenance corner (C).

All three modules use one material family and one palette. The kit is authored at the native virtual resolution (640×360 and 640×400 views) on the provisional 16×16 tile grid, so actors sit in front of it without scaling.

The kit does **not** contain the special entrance landmark (fish-demon relief, giant doors, wrought-iron stairs, marble landing), goblins, the Dozer, a goblin workshop, gameplay, or Rust/Bevy code.

## Contents

| Layer role | Atlas (manifest) | Editable source | Contents |
| --- | --- | --- | --- |
| `collision` | `floor01_terrain` | `assets/source/terrain/floor01/terrain.{grid.json,ase}` | 16 opaque tiles: `floor_top_a/b/c`, `floor_top_end_l/r`, `floor_fill_a/b`, `floor_fill_side_l/r`, `ceiling_edge_a/b/c`, `ceiling_fill_a/b`, `wall_fill_a/b` |
| `backwall` | `floor01_backwall` | `assets/source/terrain/floor01/backwall.{grid.json,ase}` | 64 opaque tiles: six 4×2 brick panels `brick_backwall_a1`…`f8`, the under-ceiling strip `brick_backwall_top_1..4`, the plinth strip `brick_backwall_base_1..4`, and eight `recess_*` blue-depth tiles |
| `structure` | `floor01_structure` | `assets/source/terrain/floor01/structure.{grid.json,ase}` | `arch_ring_r72` (11 voussoirs, inner radius 72 px, outer radius 96 px), `pier_capital`, `pier_shaft` (repeats vertically), `pier_base`, and `far_arcade` (distant silhouettes inside an opening) |
| `prop` | `floor01_props` | `assets/source/props/floor01/props.{grid.json,ase}` | `pipe_h_a`, `pipe_h_rust`, `pipe_h_flange`, `pipe_h_bracket`, `pipe_v_a`, `pipe_v_bracket`, `pipe_elbow_r_d`, `pipe_wall_entry_r`, `valve_wheel`, `pipe_v_floor_collar`, `maintenance_box` and the optional `lichen_patch` |
| `backwall` (decal) | `floor01_props` | same | `torch_glow`: warm light on brick faces, following the brick coursing |
| `prop` (animated) | `floor01_torch` | `assets/source/props/floor01/torch.{grid.json,ase}` | `torch_wall`: four frames, tag `burn`, 120/140/100/130 ms, forward loop |

Collidable tiles are fully opaque. Decor pieces use 1-bit alpha only, with no partial alpha anywhere. Only `collision`-role tiles are collidable. Arches, piers, pipes, torches and decals never are.

## Palette `floor01-catacomb-v1`

One shared palette of 47 opaque colours, embedded with names in every grid and `.ase` file and in the contract. Each atlas therefore stays under the 64-colour check in `scripts/validate_assets.py`.

| Family | Keys | Role |
| --- | --- | --- |
| Brick (6) | `m a b c d e` | Muted red-brown backwall: mortar and a five-step face ramp. Deliberately darker and lower in contrast than actors and the floor edge |
| Torchlit (3) | `f g h` | Restrained warm light on brick faces near torches only |
| Stone (8) | `s t u v w x y z` | Gray-neutral edges: ceiling course, arches, piers, floor lip (`y`) and glint (`z`, accent) |
| Floor surface (3) | `D E F` | Dry walking surface and the floor front face, warmed by torchlight |
| Depth (6) | `0 1 2 3 4 5` | Cold blue recesses and far arcade, darkest at the vault and lightening toward a hazy far floor |
| Flame (4) | `R O Y W` | Torch flame and two 2 px distant torches |
| Iron (6) | `k i j l n o` | Sconce, pipes, valve hub and box (`o` is an accent glint) |
| Rust / paint (5) | `q r p P T` | Sparse rust, valve wheel red and a blank ochre plate on the box |
| Lichen (3) | `G H L` | The single optional accent patch (`L` is an accent glow pixel) |

## A/B/C usage rules

- **A: plain fabric.** Brick panels between the top strip and the plinth strip, with `floor_top_a/b/c` over `floor_fill_*`. Place a `torch_wall` with its `torch_glow` roughly every 10 to 16 columns, above head height (row 10 of the sample places the flame about 150 px above the floor). Leave long stretches empty.
- **B: rhythm.** Stack `pier_base`, 3× `pier_shaft` and `pier_capital`. Pier centres sit 192 px apart (12 columns), and one `arch_ring_r72` springs from the capital tops. The backwall cells inside the ring's mid-radius, and the opening below the springing line, use `recess_*` bands from top to bottom: `vault` → `vault_fade` → `wall_a/b` → `wall_fade` → `low` → `haze` → `floor`. `far_arcade` sits on the opening's floor line. Use B for junctions, landmarks and orientation, not as wallpaper.
- **C: special function, used sparingly.** Use one pipe run (elbow, straight runs, one bracket or flange, a wall entry), one `valve_wheel` and one `maintenance_box`, with a conduit going into the floor through `pipe_v_floor_collar`. `lichen_patch` is optional and only belongs next to a damp source, such as below the valve. Do not repeat C dressing along ordinary corridors and do not grow it into a workshop.
- **Panels:** within a row, a brick panel cell must be followed by its right-hand neighbour in the same panel. A panel's last column may be followed by column 0 of any panel in the same family. Any vertical order is seamless. The joints are the same everywhere (even courses at x ≡ 15, odd courses at x ≡ 7 mod 16), which is why the grid-aligned `torch_glow` matches every panel.
- **Contact zones:** the plinth strip directly above the floor is plain and dark on purpose, so that feet and the floor lip read cleanly. Keep props out of the 64 px band above the floor, except a single vertical conduit.

## Integration contract (for COMP-01)

- **Contract:** [`assets/source/terrain/floor01/floor01_catacomb_kit.json`](../../assets/source/terrain/floor01/floor01_catacomb_kit.json) (`env-kit-v1`).
- **Sample layout:** [`assets/levels/floor01/corridor_abc_sample.json`](../../assets/levels/floor01/corridor_abc_sample.json) (`env-kit-layout-v1`).

Neither file lives in `assets/manifests/`, because every JSON file there is treated as an export manifest.

### Coordinates

- Tile size is 16. The origin is the **top-left** corner, x points right, y points **down**, and rows are ordered top to bottom.
- A layout pixel is `(col * 16, row * 16)`.
- Atlas rects are `[x, y, w, h]` in exported atlas pixels, measured from the top left.
- Bevy is y-up, so COMP-01 must flip y when converting.

### Contract schema

| Field | Content |
| --- | --- |
| `palette`, `palette_id` | The 47 named colours, identical to every grid palette (validated). |
| `layers` | Draw order: `backwall` tiles (0), `backwall_decal` pieces (1), `structure` (2), `prop` (3), `collision` tiles (4), actors (`actor_draw_order` 5), `foreground` (6, empty in ENV-01). |
| `tiles[]` | Each tile has a stable IP-neutral `id`, `atlas`, `rect`, `grid_index` (row-major in its tileset, matching the manifest `tiles` list), `role` (`collision` or `backwall`), explicit `collidable`, `grammar` (A/B/C hints), and optional `walkable_top`, `panel {id, family, cell, size}` and `recess`. |
| `pieces[]` | Each piece has `id`, `atlas`, `rect`, `role` (`backwall`, `structure`, `prop` or `foreground`), `collidable: false`, `anchor` plus `anchor_name`, and `grid_snap` (top-left must be a multiple of 1, 8 or 16). Optional fields: `arch {radius_inner, radius_outer, voussoirs, backwall_inside}`, `repeat`, `decal`. |
| `pieces[].animation` | Only `torch_wall` has one. It is the Aseprite json-array shape: `tag {name, from, to, direction}` plus `frames[] {frame {x, y, w, h}, duration}`, checked against the exported metadata. No frame is rotated or trimmed. |
| `floor_contact` | The collider top is the top edge of a `walkable_top` tile. Rows 0 to 4 of `floor_top_*` are the walking surface. The proposed visual foot sink is 2 px (the actor pivot row drawn 2 px below the collider top). COMP-01 decides the final value. |

### Sample layout

- **Size:** 128 × 25 tiles (2048 × 400 px), so both 640×360 and 640×400 views have full wall coverage.
- **Rows:**
  - rows 0 to 2: ceiling (edge on row 2);
  - row 3: the under-ceiling brick strip;
  - rows 4 to 18: brick panels;
  - row 19: plinth;
  - row 20: floor top (`floor_top_y` = 320);
  - rows 21 to 24: floor fill.
- **Floor:** the floor is collidable and continuous from end to end with no pits. One raised block, one tile high, sits at columns 30 to 37, which exercises the end and side tiles. The validator enforces "no pit" and "step ≤ 2 tiles" per column.
- **Segments** (`x` in px): `A1` 0 to 896, `B1` 896 to 1136 (one arch), `C1` 1136 to 1408, `A2` 1408 to 1600, `B2` 1600 to 2048 (two arches).
- **Markers:** `player_spawn` (x 72), `exit_east` (x 1896, inside the far B2 arch, with a trigger rect). The marker `y` is the actor pivot row, 319.
- **Views:** `abc_640x360` = `[736, 32, 640, 360]` and `abc_640x400` = `[736, 0, 640, 400]`. Both read A → B → C from left to right.
- **Animation offset:** each torch has a `frame_offset` so the six torches do not flicker in sync.

### Behaviour change to `scripts/export_assets.sh`

After the existing validation, contact-sheet and GIF steps, the script now runs `python3 scripts/env_kit.py all`. That command:

1. finds every `env-kit-v1` layout under `assets/levels/`;
2. validates the kit contract against the exported atlases, manifests and grids;
3. validates the layout against the contract;
4. renders the previews into `generated/art/env_kit/<layout_id>/`.

A contract or layout error fails the export. No other existing export behaviour changed. COMP-01 can depend on `generated/art/floor01_*.{png,json}` existing after `bash scripts/export_assets.sh`.

## Validation

`scripts/env_kit.py validate` (run inside the export) together with `tests/test_art_env_kit.py` covers the following:

- **Palette:** every opaque pixel is in the kit palette; there is no partial alpha; each grid's palette equals the contract palette; the palette has at most 64 colours.
- **Tiles:**
  - tile rects are 16-aligned 16×16 cells inside the atlas;
  - `grid_index` matches both the cell and the manifest `tiles` order;
  - every tileset cell is covered exactly once;
  - collision and backwall tiles are fully opaque;
  - only collision tiles are collidable;
  - brick panels are complete.
- **Pieces:** pieces are non-collidable, use alpha and are non-empty; static rects do not overlap; anchors lie inside their piece.
- **Animation:** the torch tag, frame rects and durations equal the exported metadata.
- **Manifests:** each manifest's `palette_id`, output paths and `grid_source` match the contract; no manifest claims approval.
- **IDs:** all IDs are IP-neutral (franchise names are rejected).
- **Layout:**
  - layer roles match;
  - pieces stay on their snap grid and inside the layout bounds;
  - brick panels stay continuous;
  - arch openings show recess inside the ring and never outside it;
  - segments are contiguous and cover the width;
  - spawn and exit markers exist;
  - views lie inside the layout;
  - the floor is continuous with steps of at most 2 tiles.
- **Determinism:** the committed `.ase` files equal a fresh build from their grids, and renders are byte-identical across runs.

The negative tests mutate in-memory copies, so each rule is proven to fire.

## Regenerating

```sh
# Pixel grids, contract and layout were first written by the authoring script.
# The committed grids are the source of truth. Re-running overwrites hand edits:
python3 scripts/author_floor01_kit.py --force   # only when deliberately regenerating
# Rebuild .ase from grids, export atlases with LibreSprite/Aseprite, validate, render previews:
ART_CLI=/path/to/libresprite bash scripts/export_assets.sh
python3 -m unittest discover -s tests -p 'test_art_*.py' -v
```

After hand-editing a grid, run the export and commit the regenerated `.ase`. CI fails when `assets/source` differs after an export.

## Self-review log (agent, before human review)

1. **First pass.** The backwall was too bright and saturated. Carl's brown hair and dark jacket partly merged with it.
   - Distant recess tiles drew continuous dashed lines that read as scanlines.
   - The torch glow used a checker ring that looked stamped.
   - The lichen floated free on the wall like an insect.
   - `pipe_v_a` repeated the same rust dots every 16 px down the conduit.
   - The floor surface was a thin gray strip.
2. **Second pass.** These fixes went in:
   - darkened and compressed the brick ramp;
   - shortened the bevel highlights;
   - made the far-masonry marks sparse and irregular;
   - removed the dither ring from the glow, made it slightly taller than wide, and kept the outer falloff on brick top edges only;
   - moved the lichen so it hugs the pipe below the valve, along a mortar joint;
   - removed the rust from the vertical pipe;
   - made the walking surface 5 px with three warm floor tones.
3. **Third pass.** In the 2048 px overview the wall showed a faint diagonal "chevron" pattern. The top-left highlight and bottom-right shadow of every brick lined up across the courses of the running bond.
   - Bevel runs now start at varied positions.
   - The right-edge shade is rarer.
   - Tone weights are narrower.
   - The panel count rose from four to six.
   - The pattern is no longer visible at 1×.
4. **Fourth pass.** The floor front face was warmed (`D/E`), so the walkable band reads as one dry, lit stone surface against the darker plinth and fill.

## Open questions for human art review

1. **Brightness and warmth.** The kit is intentionally darker, cooler and less saturated than the concept illustration, to keep actors and the floor edge dominant. Should it be warmer or brighter overall, or should more warmth be left to runtime lighting in COMP-01?
2. **Brick scale.** Bricks are 16×8 px (Carl is about 6.5 courses tall). Is that right, or should bricks be larger blocks like the concept's?
3. **Arch material.** The arches and piers are gray-neutral dressed stone. The concept's voussoirs are warmer, brick-coloured blocks. Which reads better as "one coherent material family"?
4. **Floor.** The floor uses a 5 px walking surface plus a lip. Is the foot line readable, and is the proposed 2 px visual foot sink right?
5. **Torch glow.** Is the baked glow decal acceptable, or should torch light become a runtime additive light in Bevy?
6. **Lichen.** Keep the single lichen accent, or drop it from the opening corridors?
7. **Far arcade.** Do the distant arcade silhouettes add useful depth, or are they too busy behind actors in B?

## Rights and provenance

- **Authorship:** original environment pixel art authored by an AI agent (Claude Opus) through the deterministic `scripts/author_floor01_kit.py`. That script places brick coursing, stone edges and voussoirs by rule (with a fixed seed for per-brick variation) and uses hand-drawn pixel strings for the flame, sconce, valve, box and lichen. There is no image-model raster output.
- **Concept use:** `01-catacomb-environment-clean.png` was viewed for mood and palette direction only. It was never cropped, traced, sampled or resampled into the kit. The side-by-side evidence image only display-scales it for comparison.
- **Manifests:** every manifest records provenance and `review_status: prototype_fixture_unapproved`. Evidence images are checksummed in `SHA256SUMS`.
- **Rights scope:** the kit contains no fan-concept characters. The actor contrast check composites the approved Carl and Donut atlases under the owner-reported `ART-PUBLICATION-ALL-V1` scope ([rights register](../04-production/rights-and-release.md)). That scope is not independently verified, is not a public reuse licence, and does not permit game, trailer or standalone asset-pack distribution.
