# AREA-REFUGE-01: Mordecai's Tutorial Guild room kit, design spec

**Status:** design spec and hearth hero key (frame 0 of `burn`) frozen; silhouette gate on the hero key pending; every other prop, the room JSON and the Bevy scene are execution work (section 12). Nothing here is wired into Bevy yet.

**Route:** difficulty score 1 (new design), class Easy, route **C + silhouette gate on the hero key** ([routing by difficulty](../07-agents/pixel-art-pipeline.md#routing-by-difficulty)). Work order row: [floor-01-area-kits-plan.md](../07-agents/floor-01-area-kits-plan.md) (`AREA-REFUGE-01`: modest hearth, bed, shelves and functional door; no combat systems, no unapproved character animations).

**Authorities:** room concept [02-tutorial-guild-old-guide.png](../../assets/concept/floor-01/atlas-v1/02-tutorial-guild-old-guide.png) for **mood only** ([atlas row 02](floor-01-art-atlas-v1.md)); no pixel is cropped, traced or resampled from it. Environment language: ENV-01 kit and ENV-02 guild portal ([guild_exit grid](../../assets/source/props/floor01/guild_exit.grid.json), [floor01_guild manifest](../../assets/manifests/floor01_guild.json), [ENV-02 evidence](../evidence/env-02/README.md)). Pipeline: [art direction](art-direction.md), [asset pipeline](asset-pipeline.md), [pixel-art pipeline](../07-agents/pixel-art-pipeline.md), [antihero tone](../01-design/antihero-tone.md).

Hero-key evidence: [docs/evidence/area-refuge-01/](../evidence/area-refuge-01/README.md).

## 1. Canon and tone constraints

- The room is a **modest, warm, slightly lived-in refuge** of an old guide: one bed, one shelf, one fire, one work desk. It is a safe room, not a throne room or a treasure hall. Nothing gilded, no banners, no trophies, no weapons rack.
- Tone ([antihero tone](../01-design/antihero-tone.md)): practical and worn. Furniture is plain timber with wear, the blanket is patched, the maps are curled. Warmth comes from the fire, not from ornament.
- The central floor stays **open** for the cast (atlas row 02). No prop stands on the walk line.
- No text, logos, readable writing or recognizable third-party designs on any prop (maps are abstract lines, books have blank spines).
- The room is IP-neutral in data: ids say `refuge_*`, not character names. Mordecai is a fan-concept layer placed by the scene, not by the kit.

## 2. Scale decision

Carl idle is 52 px, taken as 6 ft, so **8.7 px per foot**. Every prop is drawn at that scale (native 1x, 640 x 360 virtual layer).

| Prop | Native size (w x h) | Real size at 8.7 px/ft | Why |
| --- | --- | --- | --- |
| Hearth (hero key) | 80 x 80 | 9.2 x 9.2 ft incl. hood; opening 36 x 38 px = 4.1 x 4.4 ft | Generous old stone hearth; the fire must read at 1x behind the cast |
| Bed | 64 x 28 | 7.4 ft long, 3.2 ft to the headboard top | Single cot, low |
| Bookshelf | 36 x 56 | 4.1 x 6.4 ft | Shorter than Carl plus 4 px, never towers |
| Desk | 56 x 26 | top at 22 px = 2.5 ft | Table height for a seated old man |
| Stool | 14 x 18 | 2 ft | |
| Barrel | 22 x 28 | 3.2 ft | |
| Door (inside face) | 112 x 136 | same opening as `guild_exit` | Must match the corridor portal exactly |

## 3. Room layout (640 x 360 view)

One fixed screen, no scroll. Coordinates are **view pixels** (x 0..639, y 0..359). The room JSON uses the ENV-01 layout convention: `pixel_size` [640, 400], view `refuge_640x360` = [0, 20, 640, 360], so **layout y = view y + 20**.

```
y   0 |== ceiling string course (ENV-01 kit, y 0..9) ===================================|
      |pier|     ( window A )            ##flue##          ( window B )            |pier|
  32  |    |    apex y 32               ## 48w ##          apex y 32                 |    |
 120  |    |    sill y 120               ##     ##          sill y 120               |    |
 132  |    | .--door--.                  ##     ##                                   |    |
 160  |    | | 112x136|  T(140)   [map]  ##     ##      [portrait][landscape] T(576) |    |
 188  |    | |  arch  |              .---hood----.                                   |    |
      |    | |        | [bed  ][shelf]| mantel   |        [desk  ][st][barrel]       |    |
 268  |====|=|========|===============|==hearth==|==================================|====|  floor back line
 288  |    D226  C258 .......... open floor ........ M380 ........................... |    |  walk line
 360  |____ floor face / lower fill (ENV-01 deck tiles) _______________________________|____|
      0   16      128   148  216  252 280      360 380  424  480 484 500 520 544  624 639
```

| Band | View y | Content |
| --- | --- | --- |
| Ceiling | 0..9 | ENV-01 string course tiles, unchanged |
| Upper wall | 10..131 | ENV-01 brick backwall, two arched windows (section 4) |
| Wall | 132..267 | brick backwall; door, wall art, torches |
| Floor back strip (depth) | 268..287 | ENV-01 `deck_back` tiles; prop bases sit on **y 267** (pivot row), so they stand 20 px behind the walk line |
| Walk line | 288 | ENV-01 `deck_front` walking line; actors' pivot row is **y 287** |
| Floor face and fill | 292..359 | ENV-01 deck front, face and lower fill (deep floor) |

Wall piers: x 0..15 and 624..639 use the ENV-01 pillar or a 16 px brick return; they are the room's side walls.

Brick: the ENV-01 backwall tiles with their four height bands and base course, the same as the ENV-02 corridor, so the room reads as the space behind the guild door.

## 4. Prop list

Pivot convention: **bottom centre on the floor back line** (pivot row = last opaque row, placed on view y 267) for floor props; **top-left** for wall pieces placed by absolute position. All props are `decor` (no collider, section 8). Silhouettes are described as the outline the prop must keep at 1x.

| Id | Native size | Pivot | Placement (view) | Silhouette and construction | Palette ramps | Author |
| --- | --- | --- | --- | --- | --- | --- |
| `refuge_hearth` | 80 x 80, layers `surround`, `fire` | (40, 79) | centre x 320, base y 267 | **Frozen hero key, section 10.** Stepped corbelled hood (44, 52, 60 px wide), full-width mantel shelf at local y 22..27, lintel with raised key block, two ashlar jambs, sooted brick firebox, two logs on andirons with brass knobs, hearthstone slab. Mantel dressing: clay jug left, tin mug right, red drape over the right end | stone, mortar, brick, wood, iron, brass, copper, flame | key frozen; frames 1-3 execution |
| `refuge_hearth_flue` | 48 x 16, vertical repeat tile | (0, 0) | x 296..343, stacked from y 10 to y 187 | Chimney breast continuing the hood's top step to the ceiling string course; stone coursing 7 px, running bond offset 8, 1 px mortar contour on both long edges, soot plume darkening down the centre (strongest at the bottom tile, fading by y 60) | stone, mortar | execution |
| `refuge_bed` | 64 x 28 | (32, 27) | centre x 180 | Low timber cot: headboard post at the left (24 px tall), footboard at the right (16 px), mattress slab 6 px, pillow at the head, a patched red blanket hanging 6 px over the front rail with a frayed edge; a small trunk under the foot end (16 x 8) | wood, stone (linen 7-9), brick (blanket 4-6), iron (trunk bands) | execution |
| `refuge_bookshelf` | 36 x 56 | (18, 55) | centre x 234 | Open timber frame, 4 shelves; uneven blank-spine books (2-3 px wide, 7-11 px tall), one leaning row, a rolled scroll bundle on the bottom shelf, a stacked pair of boxes; top board overhangs 1 px each side | wood, brick, copper, stone | execution |
| `refuge_desk` | 56 x 26 | (28, 25) | centre x 452 | Trestle desk, top 3 px thick, two splayed legs with a cross stretcher; on top: an unrolled map (no text), a candle stub in a brass dish (candle light, section 7), an ink pot | wood, stone (paper 7-10), brass, iron, flame (candle tip only) | execution |
| `refuge_stool` | 14 x 18 | (7, 17) | centre x 492 | Three-legged round stool, splayed legs | wood | execution |
| `refuge_barrel` | 22 x 28 | (11, 27) | centre x 532 | Upright barrel, 3 iron hoops, a 2 px bulge, lid edge catching the torch | wood, iron | execution |
| `refuge_wall_map` | 32 x 24 | (0, 0) | top-left (164, 160) | Curled parchment pinned at 2 corners, abstract route lines and a single circle mark, no letters | stone 7-10, mortar, brick 5 (route ink) | execution |
| `refuge_frame_portrait` | 24 x 28 | (0, 0) | top-left (436, 164) | **Owner direction 2026-11-21: the framed picture shows two humanoid bird-people, Mordecai's original race, not humans.** Spec for the medium tier: a 2 px timber frame (wood 3, inner bevel wood 4, one brass 3 corner nail top-centre) around an 18 x 22 picture area. **Figures:** two original humanoid bird-people standing side by side, slightly turned toward each other (three-quarter view, so each beak shows in profile): the left one 14 px tall, the right one 12 px tall. Each head is a rounded 4 x 4 to 5 x 5 mass with a clear **2 px beak** pointing toward the other figure and a 1 px eye; the taller one has a 2 px swept-back crest, the shorter one a round head with no crest, so the pair reads as two individuals. Bodies: simple coats or robes, sloped shoulders, arms down or one hand on the other's shoulder; no weapons, no armour, no wings spread, no heroic stance. **Design them yourself:** do not copy or evoke any franchise or third-party bird character. **Palette and age:** room palette only, muted: background stone 6-7 (sepia wash), figures stone 3-5 and mortar 4-6, a single brick 4 accent at most; corners one step darker (vignette), 3-5 single-pixel scratches in stone 8. It is a quiet personal keepsake, not a heroic portrait. **1x check:** both beaks and both heads must read in the 640 x 360 lit capture, or the figures grow to the 18 x 22 limit before anything else changes | wood, brass, stone, mortar, brick (one accent) | execution |
| `refuge_frame_landscape` | 28 x 20 | (0, 0) | top-left (466, 168) | Small frame (2 px timber) with a faded ruined-arcade landscape, stone 5-8 sepia, no figures | wood, stone, mortar | execution |
| `refuge_satchel` | 12 x 18 | (0, 0) | top-left (130, 196) | Leather bag hanging from a wall peg by its strap | wood, copper, iron | execution (optional; drop if the colour budget is tight) |
| Wall torches | reuse `floor01` torch (16 x 56, 4-frame `burn`) | as the torch piece | flames at (140, 160) and (576, 160) | **No new drawing**: the existing kit torch piece and its emitter | as kit | placement only |
| `refuge_door_inside` | 112 x 136, frames `closed`, `open` | (0, 0) | top-left (16, 132) | **The inside face of the ENV-02 guild portal.** Same opening geometry as `guild_exit`: width 112, height 136, arch centre local x 56, spring local y 44 (`PORTAL_W`, `PORTAL_H`, `PORTAL_CX`, `PORTAL_SPRING` in `scripts/author_floor01_opening.py`); same voussoir count and jamb courses, mirrored so the hinge side is on the room's left. Closed: the same plank leaf and iron strap language as the portal leaf, seen from the back (battens and a drop bar instead of the outer studs). Open: leaf swung inward against the left jamb, the opening shows the corridor beyond in cold dim values (brick_cold, floor_cold, gal 2-4), no warm interior glow. No sign (the sign hangs outside) | stone, mortar, wood, iron, brick_cold, floor_cold, gal | execution |
| `refuge_window` | 96 x 100 | (0, 0) | top-left (176, 20) and (384, 20) | Round-arched window high on the wall: voussoir ring 10 px (ENV-01 arch language), opening 76 wide, spring local y 52, sill 6 px stone at local y 94..99. Behind it: a dark far gallery (gal 1-5) with an iron railing at local y 70..74 and a sagging chain inside the arch top. Cold, quiet, never brighter than gal 5 | stone, mortar, gal, iron | execution |

The door is **functional**: `refuge_door_inside` and the corridor's `guild_exit` portal are two faces of one door. Their state ids match (`closed`, `open`) and the room's exit trigger (section 8) opens it.

## 5. Animation list

| Piece | Tag | Frames | Playback | Timing | Author |
| --- | --- | --- | --- | --- | --- |
| `refuge_hearth` | `burn` | 4 | loop | 120 ms per frame (same as the kit torch) | frame 0 frozen key; 1-3 execution |
| Wall torch | `burn` | 4 (existing) | loop | existing | none |
| `refuge_door_inside` | `closed`, `open` | 1 each | state | switched by the exit trigger | execution |
| Desk candle | none | 1 | static | the light flickers, the sprite does not | execution |

Hearth frames 1-3: the `surround` cel is **byte-identical** to frame 0 in every frame. Only the `fire` cel changes: each tongue of `TONGUES` in `scripts/author_refuge01.py` varies its height by -4..+3 px and its lean by up to 2 px, out of phase (no two tongues peak in the same frame); the centre tongue never drops below 20 px; core pixels (flame 4-5) stay inside local x 33..47 and y 46..66; 1-3 sparks per frame, never in the same pixel twice in a row.

## 6. Palette

**`floor01-catacomb-v2`, no extension.** Every ramp the room needs already exists in the kit palette (`scripts/lookdev_floor01_author.py` `RAMPS` plus `wood` and `lichen` in `scripts/author_floor01_kit.py`):

| Use | Ramp and indices |
| --- | --- |
| Hearth stone, window and door stone | stone 1-9, mortar 0-3 |
| Firebox back, blanket, drape, books | brick 1-8 |
| Furniture, frames, logs | wood 0-5 |
| Hardware, andirons, hoops | iron 2-6, brass 3-5 |
| Parchment and linen | stone 7-10 |
| Fire, embers, candle | flame 0-5, copper 4-7 (glowing log edges) |
| Window gallery, cold corridor | gal 1-5, brick_cold, floor_cold |

Per-atlas budget: at most 63 colours (`MAX_ATLAS_COLOURS`). The hearth key uses 46. If the room atlas exceeds 63, split it into `floor01_refuge_props` and `floor01_refuge_wall`, do not add or merge colours.

## 7. Lighting plan (full-res lighting layer)

Emitters are declared as `lights` on the kit pieces (the ENV-01/ENV-02 convention read by `scripts/env_kit_light.py` and the Bevy lighting adapter). Offsets are sprite-local from the piece's top-left. The hearth values are the ones used in the hero-key check render.

| Emitter | Piece | Offset | Colour | Radius | Intensity | Extras |
| --- | --- | --- | --- | --- | --- | --- |
| `hearth_fire` | `refuge_hearth` | (40, 58) | #ffb060 | 230 | 0.50 | flicker [1.0, 0.93, 1.05, 0.96] (one value per `burn` frame), glow radius 26, additive 0.12 |
| `hearth_floor` | `refuge_hearth` | (40, 79) | #ffb060 | 150 | 0.32 | `project: walk_line`, `project_dy: -4`, `squash_y: 0.3` |
| Torch L, torch R | kit torch | existing | existing | existing | existing | unchanged |
| `desk_candle` | `refuge_desk` | candle tip | #ffc27a | 44 | 0.12 | flicker [1.0, 0.95, 1.03, 0.97], glow radius 6, additive 0.05 |
| `window_cold` | `refuge_window` | (48, 60) | #6a8bb5 | 110 | 0.07 | none; a cool rim, never a light shaft |
| `door_cold` | `refuge_door_inside` (`open` only) | (56, 120) | #42557a | 90 | 0.06 | `project: walk_line`, `squash_y: 0.3` |

Lighting profile: reuse the `lighting` block of `opening_route_proposal.json` (ambient, groups, haze). The hearth is the brightest source in the room; the floor in front of it (x 240..400) is the brightest floor area; the room corners stay at brick 2-4.

## 8. Collision and walkable floor

- Walkable floor: ENV-01 `deck_front` tiles from x 16 to 623, walking line view y 288 (layout y 308). Single walk line, no steps, no platforms.
- Solids: the floor deck and the two side piers (x 0..15, x 624..639, full height). Nothing else.
- **All props are decor, without colliders.** They stand on the back strip (y 268..287), behind the walk plane, so actors pass in front of them. No prop sprite box becomes a collider (no inferred bounding-box colliders).
- Door trigger `exit_guild_return`: rect x 40..104, y 228..287 (view). Entering it plays `closed -> open` and returns to the corridor's `exit_guild` position. Execution decides only the wiring, not the rect.
- No hazards, no spawns, no enemies, no breakables.

## 9. Cast marks

Walking-line x positions (pivot column on y 287):

| Mark | x | Facing | Note |
| --- | --- | --- | --- |
| `mark_donut` | 226 | right | in front of the bookshelf |
| `mark_carl` | 258 | right | left of the hearth, clear of the fire opening (Carl's box ends at about x 269; the opening starts at x 302) |
| `mark_mordecai` | 380 | left | right of the hearth, host by his fire; ~44 px tall (MORDECAI-01 proposal), box about x 368..392, clear of the hearth (ends at x 359) and the desk (starts at x 424) |

The hearth is framed between the guests and the host. Marks are room data (`marks`), not kit data.

## 10. Hero key: `refuge_hearth` frame 0

Source: [refuge_hearth.grid.json](../../assets/source/props/floor01/refuge_hearth.grid.json) and `refuge_hearth.ase`, written by `scripts/author_refuge01.py` (deterministic, rule-based shading quantised to the palette; no image-model output, nothing traced from the concept).

| Part | Local rows | Design note (frozen) |
| --- | --- | --- |
| Hood | 0..21 | three corbel steps 44/52/60 px wide, 7 px courses, soot plume down the centre |
| Mantel | 22..27 | full 80 px; lit top row, fire-lit underside, 1 px shadow line on the lintel |
| Lintel | 28..35 | two caps and a centre block with a raised key (x 37..42) |
| Jambs | 36..73 | 4 ashlar courses each, x 6..21 and 58..73; inner edges catch the fire, outer edges fall off |
| Firebox | 36..73, x 22..57 | angled reveals 4 px, small sooted brick, brightest at the fire centre (40, 62) |
| Fire | 39..66 (`fire` layer) | five tongues; core flame 4-5 low and central, flame 1 tips; 3 sparks |
| Logs and andirons | 60..73 | back and front log, glowing copper top edge, brass knobs at x 24 and 55 |
| Hearthstone | 74..79 | full width, lit top rows, 3 vertical joints |
| Silhouette | | 1 px mortar contour on the outer stone edge so the grey-warm stone separates from the red-orange brick |

The lit check composes the key in the ENV-02 `s6_guild` view at 1x with Carl and Donut ([evidence](../evidence/area-refuge-01/README.md)). It reads as a fireplace at 1x: the fire is the brightest spot of the frame, the stone separates from the brick, and Carl stands clear of the opening.

## 11. Acceptance targets

1. Every prop of section 4 exists as a pixel-grid-v1 source plus `.ase`, on `floor01-catacomb-v2`, 1-bit alpha, each atlas at most 63 colours.
2. Native sizes and pivots match section 4 exactly (validator-checked).
3. Hearth `surround` cel byte-identical across the 4 `burn` frames; frame 0 `fire` cel byte-identical to the frozen key.
4. `refuge_door_inside` opening geometry equals `guild_exit` (112 x 136, arch centre 56, spring 44); a test compares them.
5. The room JSON validates against the ENV-01 kit contract and layout validators; the only solids are floor and piers; the walk line is continuous from x 16 to 623.
6. With the three cast members on their marks, no actor box overlaps the hearth opening (x 302..337) or the desk; the floor between x 200 and 420 is free of props on the walk line.
7. Lit 640 x 360 and 1920 x 1080 captures: the hearth is the brightest region; the two windows read cold and stay at or below gal 5; the corners are dark; the portrait's two bird-people read as bird-people (beaks visible) at 1x.
8. No text, logos or third-party designs on any prop; no pixel derived from the concept image.
9. The room does not enter the f01_late playtest manifest; no existing pin, golden or replay changes.

## 12. Execution list

For the execution tier; no design decisions needed. Questions that are design go back to the designer.

1. **Hearth frames 1-3** in `scripts/author_refuge01.py` per section 5; extend the `burn` tag to frames 0-3 and add the `hearth_fire` flicker values.
2. **Draw the remaining props** of section 4 in the same script (or a sibling `scripts/author_refuge01_props.py`), as pixel-grid-v1 sources plus `.ase` via `scripts/pixel_grid_to_ase.py build`: `refuge_hearth_flue`, `refuge_bed`, `refuge_bookshelf`, `refuge_desk`, `refuge_stool`, `refuge_barrel`, `refuge_wall_map`, `refuge_frame_portrait` (**owner direction 2026-11-21: two original humanoid bird-people, section 4 row**), `refuge_frame_landscape`, `refuge_satchel` (optional), `refuge_door_inside` (`closed`, `open`), `refuge_window`. Sources under `assets/source/props/floor01/refuge_*.grid.json` / `.ase`.
3. **Kit contract** `assets/source/terrain/floor01/floor01_refuge_kit.json`, extending the ENV-01 kit the way `floor01_opening_kit.json` does (the ENV-01 contract and grids are not modified): pieces, anchors, `lights` from section 7.
4. **Manifests** `assets/manifests/floor01_refuge*.json` modelled on `floor01_guild.json` (decor_atlas, palette `floor01-catacomb-v2`, provenance with `generated_art: true`, `review_status: prototype_fixture_unapproved`), and SHA-256 sums; run `bash scripts/export_assets.sh` and the existing asset validators (`scripts/validate_assets.py`, `python3 -m scripts.pixelart` validators). No bespoke validators.
5. **Room layout** `assets/levels/floor01/refuge_guild.json` in the corridor layout format (sections 3, 7, 8): tile layers, piece layers, view `refuge_640x360`, lighting profile, the `exit_guild_return` trigger and the `marks` of section 9.
6. **Bevy room**: a separate room file (`assets/levels/floor01/refuge_guild.room.json` or the schema the Bevy room loader expects) and a launchable scene or room option (for example a `--room refuge_guild` flag or a scene id) that renders the room lit with Carl, Donut and, once MORDECAI-01 lands, Mordecai on their marks. Not in the f01_late playtest manifest; register new data in `data_pins()` only if the convention requires it, without moving existing pins.
7. **Evidence** under `docs/evidence/area-refuge-01/`: empty-room contact sheet (1x and 2x, unlit and lit), per-prop 1x/3x sheet, hearth `burn` GIF at 2x, lit 640 x 360 and 1920 x 1080 captures with the cast on marks, a real Bevy capture, the 3-image review pack (`python3 -m scripts.pixelart review`), README and `SHA256SUMS`.

## 13. Files

| Path | Content |
| --- | --- |
| `scripts/author_refuge01.py` | authoring record: hearth key (and later frames), lit check scene |
| `assets/source/props/floor01/refuge_hearth.grid.json` / `.ase` | hearth source |
| `docs/evidence/area-refuge-01/` | hero-key lit check (this phase); full evidence (execution) |
| `assets/source/props/floor01/refuge_*.grid.json` / `.ase` | other props (execution) |
| `assets/source/terrain/floor01/floor01_refuge_kit.json` | kit contract (execution) |
| `assets/manifests/floor01_refuge*.json` | manifests (execution) |
| `assets/levels/floor01/refuge_guild.json`, `refuge_guild.room.json` | room layout and Bevy room (execution) |

## 14. Open questions and adopted defaults

| ID | Question | Adopted default |
| --- | --- | --- |
| REF-Q1 | Mordecai's mark: 380 (host side of the hearth) or closer to Carl? | x 380, facing left; re-check in the lit scene once MORDECAI-01's idle key exists |
| REF-Q2 | Does the hood end in a flue to the ceiling? | yes, `refuge_hearth_flue` repeated to the string course |
| REF-Q3 | Door side and swing | left wall, hinge on the room's left, swings inward; corridor cold beyond |
| REF-Q4 | One fixed screen or a short scroll? | one fixed 640 x 360 screen |
