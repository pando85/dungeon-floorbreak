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
| `collision` | `floor01_terrain` | `assets/source/terrain/floor01/terrain.{grid.json,ase}` | 32 opaque tiles: `floor_top_a/b/c/d`, `floor_top_end_l/r`, `floor_fill_a/b`, `floor_fill_side_l/r`, `floor_inner_corner_l/r` and `floor_step_base_a/b` (footing under a raised block), `ceiling_edge_a/b/c`, `ceiling_fill_a/b`, `ceiling_corner_in_l/r`, `ceiling_corner_out_l/r`, `wall_fill_a/b`, `wall_face_l/r` and `wall_face_l_b/r_b` (stone quoins, two phases), `wall_cap_l/mid/r` |
| `backwall` | `floor01_backwall` | `assets/source/terrain/floor01/backwall.{grid.json,ase}` | 64 opaque tiles: six 4×2 brick panels `brick_backwall_a1`…`f8`, the under-ceiling strip `brick_backwall_top_1..4`, the plinth strip `brick_backwall_base_1..4`, and eight `recess_*` blue-depth tiles |
| `structure` | `floor01_structure` | `assets/source/terrain/floor01/structure.{grid.json,ase}` | `arch_ring_r72` (11 voussoirs, inner radius 72 px, outer radius 96 px), `pier_capital`, `pier_shaft` (repeats vertically), `pier_base`, and `far_arcade_a` (three far openings) / `far_arcade_b` (two wider openings) for the depth inside an opening |
| `prop` | `floor01_props` | `assets/source/props/floor01/props.{grid.json,ase}` | `pipe_h_a`, `pipe_h_rust`, `pipe_h_flange`, `pipe_h_bracket`, `pipe_v_a`, `pipe_v_bracket`, `pipe_elbow_r_d`, `pipe_elbow_l_u`, `valve_wheel`, `pipe_v_floor_collar`, `pipe_v_ceiling_collar`, `pipe_wall_collar_r` (round face-on wall collar), `maintenance_box`, `door_plank` (64 px timber door in a stone frame), `torch_wall_unlit` and the optional `lichen_crust` |
| `backwall` (decals) | `floor01_props` | same | `torch_glow` (minimal warm light on brick faces and top edges) and `grime_patch_a/b` (soot on a few whole bricks), both following the brick coursing |
| `prop` (animated) | `floor01_torch` | `assets/source/props/floor01/torch.{grid.json,ase}` | `torch_wall`: four frames, tag `burn`, 120/140/100/130 ms, forward loop |

Collidable tiles are fully opaque. Decor pieces use 1-bit alpha only, with no partial alpha anywhere. Only `collision`-role tiles are collidable. Arches, piers, pipes, torches and decals never are.

## Palette `floor01-catacomb-v1`

One shared palette of 47 opaque colours, embedded with names in every grid and `.ase` file and in the contract. Each atlas therefore stays under the 64-colour check in `scripts/validate_assets.py`. Revision 1 set the ramps by CIE L*: brick faces sit at L* 27.5–32.6 (mortar 23.1), clearly above the darkest actor masses (Carl's jacket and hair are L* 17.7), and the actor-height depth haze is L* 21–26.

| Family | Keys | Role |
| --- | --- | --- |
| Brick (6) | `m a b c d e` | Muted, low-saturation red-brown backwall: mortar one step below the darkest face and a five-step face ramp (L* 23–35). Lower in contrast than actors and the floor edge |
| Torchlit (3) | `f g h` | Restrained warm light on brick faces near torches only |
| Stone (8) | `s t u v w x y z` | Neutral stone warmed toward the brick hue but less saturated, so piers and arches read as the same material family: ceiling course, arches, piers, quoins, floor lip (`y`) and glint (`z`, accent) |
| Floor surface (3) | `D E F` | Dry walking surface and the floor front face, warmed by torchlight |
| Depth (6) | `0 1 2 3 4 5` | Cold blue recesses and far arcade: dark vault (L* 7) to actor-height haze (L* 21–26) and a far floor line (L* 32) |
| Flame (4) | `R O Y W` | Torch flame and two 2 px distant torches |
| Iron (6) | `k i j l n o` | Sconce, pipes, valve hub and box (`o` is an accent glint) |
| Rust / paint (5) | `q r p P T` | Sparse rust, valve wheel red and a blank ochre plate on the box |
| Timber (3) | `U V X` | The plain door's planks |
| Lichen (3) | `G H L` | The single optional crust (two greens plus one `L` glow pixel) |

## A/B/C usage rules

- **A: plain fabric.** Brick panels between the top strip and the plinth strip, with `floor_top_a/b/c/d` over `floor_fill_*`. Place a `torch_wall` with its `torch_glow` at uneven spacing (roughly every 10 to 20 columns) above head height; row 10 of the sample places the flame about 150 px above the floor. An occasional `torch_wall_unlit`, `door_plank` or `grime_patch_a/b` breaks repetition. Leave long stretches empty.
- **B: rhythm.** Stack `pier_base`, 3× `pier_shaft` and `pier_capital`. Pier centres sit 192 px apart (12 columns), and one `arch_ring_r72` springs from the capital tops. The backwall cells inside the ring's mid-radius, and the opening below the springing line, use `recess_*` bands from top to bottom: `vault` → `vault_fade` → `wall_a/b` → `wall_fade` → `low` → `haze` → `floor`. `far_arcade_a/b` sit on the opening's floor line; alternate them between neighbouring openings. Use B for junctions, landmarks and orientation, not as wallpaper.
- **C: special function, used sparingly.** Use one pipe run (elbows, straight runs, one bracket or flange), one `valve_wheel` and one `maintenance_box`. Every pipe end terminates in a collar or the box, never in mid-air: `pipe_v_floor_collar` into the floor, `pipe_v_ceiling_collar` into the ceiling course, or `pipe_wall_collar_r` into the wall. `lichen_crust` is optional and only belongs next to a damp source, such as below the valve. Do not repeat C dressing along ordinary corridors and do not grow it into a workshop.
- **Corridor ends and raised blocks.** Close a corridor with `wall_face_l/r` (stack with `wall_face_*_b` for the second quoin phase), `ceiling_corner_in_l/r` where the ceiling meets the wall, `wall_cap_l/mid/r` on top of low walls and `ceiling_corner_out_l/r` where a ceiling course ends. Under a raised block that stands on the floor, use `floor_inner_corner_l/r` at its ends and `floor_step_base_a/b` between them, so the block's lit side edge stops at the walking line. The sample layout does not end the corridor (its floor must stay continuous); the contact sheet shows these tiles.
- **Panels:** within a row, a brick panel cell must be followed by its right-hand neighbour in the same panel. A panel's last column may be followed by column 0 of any panel in the same family. Any vertical order is seamless. The joints are the same everywhere (even courses at x ≡ 15, odd courses at x ≡ 7 mod 16), which is why the grid-aligned `torch_glow` matches every panel.
- **Contact zones:** the plinth strip directly above the floor is solid dark stone (no high-frequency detail), and so is the contact shadow under the ceiling course, so feet, the floor lip and collision edges read cleanly. Keep small props out of the 64 px band above the floor; only a single vertical conduit and the plain door reach down to it.

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
| `floor_contact` | The collider top is the top edge of a `walkable_top` tile. Rows 0 to 4 of `floor_top_*` are the walking surface. **Recommended visual foot sink: 0 px**, per the owner decision of 2026-11-16 (COMP-01 draws the actor pivot row directly on the collider top). The independent art-director review suggested 2 px; that is recorded as an open question. |

### Sample layout

- **Size:** 128 × 25 tiles (2048 × 400 px), so both 640×360 and 640×400 views have full wall coverage.
- **Rows:**
  - rows 0 to 2: ceiling (edge on row 2);
  - row 3: the under-ceiling brick strip;
  - rows 4 to 18: brick panels;
  - row 19: plinth;
  - row 20: floor top (`floor_top_y` = 320);
  - rows 21 to 24: floor fill.
- **Floor:** the floor is collidable and continuous from end to end with no pits. One raised block, one tile high, sits at columns 30 to 37; it uses `floor_top_end_l/r` on top and `floor_inner_corner_l/r` plus `floor_step_base_a/b` as its footing. The validator enforces "no pit" and "step ≤ 2 tiles" per column.
- **Dressing:** torches at columns 7, 18, 33, 50, 82 and 94 (uneven spacing), a burnt-out sconce at column 42, a door at columns 12–13, five grime patches, and in C a pipe run that rises into the ceiling course at column 85 and feeds the box and a floor conduit.
- **Segments** (`x` in px): `A1` 0 to 896, `B1` 896 to 1136 (one arch), `C1` 1136 to 1408, `A2` 1408 to 1600, `B2` 1600 to 2048 (two arches).
- **Markers:** `player_spawn` (x 72), `exit_east` (x 1896, inside the far B2 arch, with a trigger rect). The marker `y` is the actor pivot row, 319.
- **Views:** `abc_640x360` = `[736, 32, 640, 360]` and `abc_640x400` = `[736, 0, 640, 400]`. Both read A → B → C from left to right.
- **Animation offset:** each torch has a `frame_offset` so the six torches do not flicker in sync.
- **Far arcades:** B1 uses `far_arcade_a`; B2 alternates `far_arcade_b` and `far_arcade_a`.

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

5. **Revision round 1 (independent Claude Opus art-director review, "needs revision before human review").**
   - *Value separation (blocker):* brick faces were L* 17–24, the same as Carl's jacket (17.7). The brick ramp now sits at L* 23–35 with lower saturation, mortar is one step below the faces, and the actor-height recess bands are L* 21–26. Measured on `actor_check_640x360.png`, torso band behind the actors: A brick median L* 30.0 (p10 23.1), B far arcade median 25.9 (p10 21.1), C brick median 30.0.
   - *Far arcade (blocker):* it read as tombstones (light rounded shapes on dark). Figure and ground are inverted: mid-blue far wall, spandrels and piers, dark arch heads, lighter floor-level haze with a 1 px haze floor line, a straight far cornice. A second variant `far_arcade_b` (two wider openings) breaks repetition in B2.
   - *Dither:* the 1 px checker rows in `recess_vault_fade`, `recess_wall_fade` and the far arcade are gone, replaced by straight bands with one staggered 3 px notch per tile. Most 1 px dashes in `recess_wall_*` were dropped. A first stepped attempt undulated and read as a water line, so it was flattened.
   - *Torch glow:* now three bands over about 44 px with a per-brick radius jitter, lighting only brick faces and top edges; the stray lit brick-top segment below the torch is gone.
   - *Collision edges:* the plinth (`brick_backwall_base_*`) and the under-ceiling contact shadow (`brick_backwall_top_*`) are solid, with no tooth rows.
   - *Stone pocks:* the uniform single-pixel pock on every stone block is removed; about a third of blocks (voussoirs, piers, `ceiling_edge_b`, `floor_top_b/d`, `floor_fill_b`, `ceiling_fill_b`) carry a 3 px chip cluster instead.
   - *Arch:* only the upward-facing edge of each voussoir is lit (the crown voussoirs' top edge), the inner and outer rims are gone, a 3.5 px darker soffit band runs inside the ring, and a 1 px contact shadow sits outside the extrados.
   - *Stone warmth:* the stone ramp is warmed toward the brick hue at much lower saturation.
   - *Raised block:* new `floor_inner_corner_l/r` and `floor_step_base_a/b` stop the block's lit side edge at the walking line.
   - *Pipe ends:* the run now rises through `pipe_elbow_l_u` into the ceiling course with `pipe_v_ceiling_collar`; `pipe_wall_entry_r` was replaced by the round face-on `pipe_wall_collar_r`.
   - *Modularity essentials:* wall faces with quoins, wall caps, ceiling inner/outer corners, `door_plank`, `pipe_elbow_l_u` and `torch_wall_unlit`.
   - *Nice-to-have:* the lichen became a flat 8×3 crust on a mortar joint, `grime_patch_a/b` decals add soot over a few bricks in A, torch spacing varies, and the torch has a stronger leaning frame plus one ember pixel.

## ENV-02 follow-up (listed, not built)

- A lower-ceiling corridor variant.
- A blind (filled) arch for walls without openings.
- A wall pilaster.
- A smaller arch radius for narrow passages.
- A pipe T-junction.

## Open questions for human art review

1. **Brightness and warmth.** The kit is intentionally darker, cooler and less saturated than the concept illustration, to keep actors and the floor edge dominant. Should it be warmer or brighter overall, or should more warmth be left to runtime lighting in COMP-01?
2. **Brick scale.** Bricks are 16×8 px (Carl is about 6.5 courses tall). Is that right, or should bricks be larger blocks like the concept's?
3. **Arch material.** The arches and piers are gray-neutral dressed stone. The concept's voussoirs are warmer, brick-coloured blocks. Which reads better as "one coherent material family"?
4. **Foot sink.** The owner chose 0 px for COMP-01 (feet on the collider top, at the back edge of the 5 px walking surface). The independent review suggested 2 px so feet sit inside the surface. Keep 0 px after seeing it in Bevy, or switch?
5. **Torch glow.** The baked `torch_glow` is deliberately minimal. A Bevy additive light is the intended richer option; is the decal still wanted as a fallback?
6. **Lichen.** Keep the single lichen crust, or drop it from the opening corridors?
7. **Far arcade.** Does the revised arcade (dark arch heads, lighter floor-level haze, darker piers) add depth without busying the actor zone in B?
8. **Door.** Is a plain timber door right for module A, or should doors stay out of the first corridors?

## Rights and provenance

- **Authorship:** original environment pixel art authored by an AI agent (Claude Opus) through the deterministic `scripts/author_floor01_kit.py`, and revised after an independent art-director review pass, also by Claude Opus. That script places brick coursing, stone edges and voussoirs by rule (with a fixed seed for per-brick variation) and uses hand-drawn pixel strings for the flame, sconce, valve, box, door and lichen. There is no image-model raster output.
- **Concept use:** `01-catacomb-environment-clean.png` was viewed for mood and palette direction only. It was never cropped, traced, sampled or resampled into the kit. The side-by-side evidence image only display-scales it for comparison.
- **Manifests:** every manifest records provenance and `review_status: prototype_fixture_unapproved`. Evidence images are checksummed in `SHA256SUMS`.
- **Rights scope:** the kit contains no fan-concept characters. The actor contrast check composites the approved Carl and Donut atlases under the owner-reported `ART-PUBLICATION-ALL-V1` scope ([rights register](../04-production/rights-and-release.md)). That scope is not independently verified, is not a public reuse licence, and does not permit game, trailer or standalone asset-pack distribution.
