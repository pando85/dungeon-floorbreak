# AREA-GYM-01: the gym kit of the Floor 01 boss arena (room 5), design spec

**Status:** design spec and mirror-bay hero key frozen (silhouette gate PASS on the second key, section 15); kit, arena dressing and Bevy wiring executed (section 15); independent review pending; not owner-approved. Launch: `floor01_scene --scene boss_f01`, or room 5 of the playtest (`--scene f01_late --f01-room juicer`).

**Route:** difficulty score 1 (new design, a kit extension of approved ENV-01 pieces), class Easy, route **C + silhouette gate on the hero key** ([routing by difficulty](../07-agents/pixel-art-pipeline.md#routing-by-difficulty)). Work order: AREA-GYM-01 (art audit 2026-11-24, order 1), [area kits plan](../07-agents/floor-01-area-kits-plan.md) row `AREA-GYM-01` (the dedicated standalone concept exists).

**Authorities:** mood and props from the owner-selected [standalone gym panorama](../../assets/concept/floor-01/juicer-gym/floor01-juicer-gym-standalone-panorama-approved.png) and [atlas panel 09](../../assets/concept/floor-01/atlas-v1/09-juicer-gym-atlas-excerpt.png), **mood only** ([concept notes](floor-01-juicer-gym-standalone-concept.md)); no pixel is cropped, traced or resampled from either. Geometry: [boss design](floor-01-boss-design.md) section 2 (one locked 640 px screen, walls at x 0..32 and 608..640, the ceiling collider 224 px above the walking line, the walls "faced with cracked mirrors (cosmetic)" and the crash surfaces), `src/boss_scene.rs`, `assets/data/bosses/brute_boss_f01.json`. The crack it anchors: `boss-f01-vfx` `mirror_crack` (pivot (76, 24), drawn at the wall x and 40 px above the walking line). The gate it caps: [danger FX spec](floor01-danger-fx-spec.md) 2.4 (`dfx_gate`; the independent review of #263 noted as its finding F3 that the grate ends against open brick 224 px up and that a lintel or header cap would sell "no way over"). Environment language: the [ENV-01 kit](floor-01-catacomb-kit.md) contract and palette. Pipeline: [art direction](art-direction.md) ("some fluorescent/accented light **only here**", [atlas row 09](floor-01-art-atlas-v1.md)), [asset pipeline](asset-pipeline.md), [antihero tone](../01-design/antihero-tone.md).

## 1. Canon and tone constraints

- A **1990s strip-mall gym squatting in a catacomb**: cheap aluminium-framed mirrors bolted to old brick, a battered reception counter with a beige terminal, a coin-op turnstile, chipped chrome dumbbells, a red vinyl bench with split seams, black rubber mats, a cable tower and a stack of tractor tyres, a steel girder carrying buzzing tube lights. Grimy, used, a little ridiculous; nothing new or glamorous.
- Dark comedy lives in the wear (hand smears on the mirrors, tape on the vinyl, a taped mat seam), never in text: **no words, logos, brand marks or posters of real people**; the gym banner of the concept is not drawn (no lettering at this scale).
- The gym is **separate from the Ball of Swine arena**: no rings, pig emblems or audience stands.
- Fluorescent light is used **only in this kit** (art atlas row 09) and stays restrained: the warm amber torches of the catacomb remain the key light; the tubes add a cold fill and one magenta accent.
- Engine and data ids stay IP-neutral (`gym_*`, `floor01_gym_*`); the boss name appears nowhere in the kit.

## 2. Scale and coordinates

Carl 52 px is about 1.8 m, so 1 m is about 29 px. All coordinates are **arena layout pixels** (top-left origin, y down), as composed by `camera_lab::layout` for `boss01_arena.room.json`:

| Line | y | Note |
| --- | --- | --- |
| Walking line `W` | 420 | `floor 416` rounds to the 16 n + 4 deck line |
| Floor back line `FB` | 400 | top of the `deck_back` row: props stand here |
| Floor front edge | 424 | the deck lip |
| Ceiling collider underside `C` | 196 | `W - ceiling_px` (224); every hero top stays at or below it |
| 640 x 360 view | 132..492 | HUD band covers y 132..192 |
| 640 x 400 view | 112..512 | |
| Ground arch window (gallery) | x 192..400, y 192..416 | the one bay at x 160 |
| Mirror crack, right / left | x 597..624 / 16..43, y 362..395 | frame 2 content (largest); pivot at the wall x, `W - 40` |
| Gate (`dfx_gate`) | x 0..28, y 196..420 | |
| Left rack footprint / strut / bait | x 83..163 / 192..208 / 170..196 | must stay readable |
| Kit exit rect | x 612..640, y 324..420 | heroes reach it once the right wall is released |

## 3. Room layout (zones, left to right)

1. **A, threshold (x 0..160):** the gate under a steel **lintel cap**; the **left mirror bay** on the back wall from x 28 (the left crack lands on its glass at x 29..43); the **reception desk** (x 46..106) and the **turnstile** (x 110..130) stand in front of the mirror's lower half, clear of the crack's x range.
2. **B, fighting lane (x 160..528):** the catacomb arch stays (the gallery shows through it); on the floor back line the **dumbbell rack**, the **flat bench** and the **plate tree** sit under the arch, the **cable tower** stands at the right pier; black **rubber mats** cover the floor surface from x 224 to 512. Nothing stands on the walking line.
3. **C, crash wall and exit (x 528..640):** the **right mirror bay** runs to the end-wall column (glass to x 622, so the right crack at x 597..622 always lands on glass); the **tyre stack** at its foot; the **exit door** in the end-wall strip (x 620..640), closed while the boss lives, open once the wall is released.
4. **Overhead:** a riveted **ceiling girder** on the ceiling collider line (bottom row y 195) from the lintel to the right edge, so the jump ceiling is visible, and three **tube fixtures** hanging under it (two cold, one magenta).

The reused catacomb `door_timber` (x 216) and the reused `guild_portal_closed` exit marker are **not** drawn in the arena (the portal reads as the Guild door and stood where the right crack lands); the gym exit door replaces it. Torches, piers, arch, chain, grime and pools of the composition stay as they are.

## 4. Piece list

All pieces are decor: `collidable: false`, no tiles, 1-bit alpha. Anchors are the placement's top-left (the layout convention); `y` is the top row.

| Piece | Atlas | Size | Layer | Placed at (x, y) | Design |
| --- | --- | --- | --- | --- | --- |
| `gym_mirror_bay` (**hero key**) | `floor01_gym_wall` | 96 x 74 | `backwall_decal` | (28, 326), (528, 326) | two silvered panels butted at a hairline seam in a thin aluminium frame, glass down to the rubber skirting; section 10 |
| `gym_gate_lintel` | `floor01_gym_wall` | 48 x 18 | `structure` | (0, 180) | riveted cold-steel header box over the gate, the gate slot shadow on its bottom rows, two angle brackets into the brick |
| `gym_girder` | `floor01_gym_wall` | 32 x 10 | `structure` | x 48..592 every 32 (to the end-wall column at 624), y 186 | rolled I-beam: lit top flange, dark web with rivet pairs, lit bottom flange; one rust streak per two spans |
| `gym_fluoro_cold` | `floor01_gym_fixture` | 40 x 10 | `prop` | (232, 196), (360, 196) | two chains, sheet-steel housing, a pale tube; cold-white light |
| `gym_fluoro_accent` | `floor01_gym_fixture` | 40 x 10 | `prop` | (520, 196) | the same fixture with a warmer tube; magenta light (the accent) |
| `gym_reception_desk` | `floor01_gym_props` | 60 x 32 | `prop` | (46, 368) | scuffed timber counter, lighter top slab, beige terminal with a cold screen, a paper sign pinned on the front (no text), a mug |
| `gym_turnstile` | `floor01_gym_props` | 20 x 30 | `prop` | (110, 370) | steel post, rotor hub, three arms (one toward the viewer), a dull red lamp on top |
| `gym_dumbbell_rack` | `floor01_gym_props` | 56 x 28 | `prop` | (228, 372) | two-tier steel rack, six chrome-and-iron dumbbells, one gap |
| `gym_bench` | `floor01_gym_props` | 44 x 18 | `prop` | (296, 382) | flat bench, red vinyl pad with a taped split, steel legs |
| `gym_plate_tree` | `floor01_gym_props` | 28 x 36 | `prop` | (352, 364) | post on a cross foot, four round iron plates face-on with hubs |
| `gym_cable_tower` | `floor01_gym_props` | 44 x 104 | `prop` | (440, 296) | two uprights, top beam with two pulley wheels, cable, a selector weight stack, a hanging handle |
| `gym_tyres` | `floor01_gym_props` | 34 x 18 | `prop` | (490, 382) | two stacked tractor tyres, treads catching the light |
| `gym_mat_a`, `gym_mat_b` | `floor01_gym_floor` | 48 x 22 | `structure` | x 224..512 every 48, y 401 (a, b, a, b, a, b) | rubber tile with moulded studs over the floor surface (rows 401..422), one `iron` step lighter than the first cut so the lane reads as floor; 1 px seam; the far row and the front edge lit, the front face darker; `b` has a grey duct-taped seam (no yellow: it must not read as hazard striping) and a scuffed corner |
| `gym_exit_closed`, `gym_exit_open` | `floor01_gym_exit` | 20 x 96 | `prop` | (620, 324) | state pair (`state_group` `gym_exit`): a riveted steel door in a narrow frame with a push bar under a dull red 8 x 3 lamp / the leaf swung in against the jamb, a **warm lamp-lit passage** behind it and the lamp lit brass with a halo (the same "open = warm lit passage" cue as the rooms 2/4/6 exits of `dfx_exit`) |

## 5. Animation

None. Every piece is a still. The exit door is a **state pair**: the presenter (`src/gym_art.rs`) shows `closed` while `Boss::released()` is false and `open` after; it reads the boss state only.

## 6. Palette

**`floor01-catacomb-v2`, no extension** (an extension contract may not redeclare the palette). Ramps used: `stone_cold` and `gal` (mirror glass, aluminium, steel), `iron` (rubber, girder, weights), `brick` (red vinyl, the turnstile lamp), `wood` and `stone` (desk, terminal, paper), `copper` and `brass` (rust, tape, lamp), `flame`/`copper_08` (tube cores), `floor_cold` (the open doorway). The magenta of the accent tube is **light data only** (its glow colour); its pixel tube is a warm cream (`copper` 8 / `brass` 6), the cold fixtures' tube is near-white (`flame` 5 / `gal` 10). Each atlas stays at or below 63 colours.

## 7. Lighting (presentation only)

| Emitter | Piece | Offset | Colour | Radius | Intensity | Extra |
| --- | --- | --- | --- | --- | --- | --- |
| `tube` | `gym_fluoro_cold` | (20, 8) | `#cfdcff` | 150 | 0.26 | glow 16 / 0.10 |
| `tube_floor` | `gym_fluoro_cold` | (20, 8) | `#bccbff` | 120 | 0.16 | `project: walk_line`, `project_dy` -6, `squash_y` 0.3 |
| `tube` | `gym_fluoro_accent` | (20, 8) | `#ff62cf` | 150 | 0.30 | glow 18 / 0.18 |
| `tube_floor` | `gym_fluoro_accent` | (20, 8) | `#ff7ad6` | 110 | 0.12 | `project: walk_line`, `project_dy` -6, `squash_y` 0.3 |
| `door_lamp` | `gym_exit_open` | (10, 3) | `#ffc27a` | 60 | 0.16 | glow 8 / 0.08 (always on, GYM-AD8) |
| `door_leak` | `gym_exit_open` | (10, 94) | `#ffc27a` | 90 | 0.22 | `project: walk_line`, `squash_y` 0.3 (always on, GYM-AD8) |

The torches stay the key light. Total emitters in the arena: 8 torch + 6 tube + 2 exit = 16 (pass limit 64).

## 8. Collision and state

Presentation only. The kit adds no tile and no collidable piece; the arena's solid rectangles, spawn, exit, views, walking line and blink barriers are **identical** with and without the dressing (unit test), and every boss fixture hash and golden is byte-identical. The exit door state is a pure function of `Boss::released()`.

## 9. Readability rules

1. The crack zones (section 2) contain only mirror glass or frame: no prop is placed over them (the crack draws over props).
2. Nothing stands on the walking line; props stand on the floor back line and are at most 36 px tall, except the cable tower (104 px) behind the right pier, kept one value step darker than the actors.
3. The mats keep the stone lip row (y 423) and the floor's back row (y 400) visible, so the floor still reads as floor, not a pit or a hazard strip.
4. The girder's bottom row is the jump ceiling (y 195); tubes hang below it inside the play space but behind the actors.
5. The left rack footprint, the strut and the bait spot keep a clear background above the floor back line: only the desk (32 px) and the turnstile (30 px) are near them, both below the hanging rack by more than 100 px.

## 10. Hero key: `gym_mirror_bay`

Source: [gym_wall.grid.json](../../assets/source/props/floor01/gym_wall.grid.json) (the `gym_mirror_bay` rect), written by `scripts/author_gym01.py` (deterministic, rule-based, quantised to the kit palette).

| Part | Local rows / columns | Design note (frozen) |
| --- | --- | --- |
| Frame | row 0, row 70; cols 0 and 95 | thin brushed aluminium (`stone_cold`): lit top row, dark bottom row; iron clips at each panel corner; the two outer top corners rounded (transparent) |
| Glass | rows 1..69; cols 1..46 and 48..94 | silvered blue-grey (`stone_cold` 1..3), a smooth ramp lighter toward the top with a 2 x 2 ordered dither at each step (no block noise); lit top row; local rows 29..55 take every other pixel one step toward the warm room (`gal_warm` 1..2) |
| Mirrored brick | local rows 30..55 | faint courses of this room: a 1 px `brick` 2 line every 5 rows, staggered `gal_warm` 1 joints every 12 px |
| Seam | col 47 | a dark hairline where the two panels butt (no mullion) |
| Reflection | rows 57..69 | the mirrored torch-lit floor: a warm band (`gal_warm` 1..3) with a brighter top row |
| Mirrored lights | two per panel, never level (local rows 9, 24, 27, 12) | warm glow points (`gal_warm` 6 core, 4..5 ring, a dithered `gal_warm` 3 halo) at different heights so the panels do not repeat |
| Reflected room | rows 45..56 | a dim reversed silhouette of a bench and a plate stack (`stone_cold` 0) |
| Sheen | two streaks per panel at 40 degrees | one 2 px `gal` 10 core with a 1 px `stone_cold` 4 edge, one 1 px `gal` 9 line; offsets differ per panel |
| Grime | rows 34..45 | hand smears (one step darker clusters) at 1 m to 1.5 m, a strip of brown tape on the right panel's lower corner |
| Skirting | rows 71..73 | black rubber skirting, a lit top row |

**Silhouette gate:** a fresh reviewer compares the key at 1x in the lit arena with the concept's mirror wall and answers "does it read as a gym wall mirror, and does the crack land on something that reads as glass?". Result in section 15.

## 11. Acceptance targets

1. Every piece of section 4 exists as a pixel-grid-v1 source plus `.ase`, on `floor01-catacomb-v2`, 1-bit alpha, each atlas at most 63 colours, and validates through `scripts/env_kit.py validate --contract assets/source/terrain/floor01/floor01_gym_kit.json`.
2. Native sizes match section 4 (test).
3. The arena composition with the gym dressing has the same solids, spawn, exit, views, walking line and blink barriers as without it (Rust test), and every boss fixture hash and golden is unchanged (the existing suites).
4. The right crack zone (x 597..621, y 362..395) lies on mirror glass and the visible left crack zone (x 30..43) too; no prop rectangle overlaps either crack zone (test).
5. No standing piece crosses the walking line; the mats lie on rows 401..422 and leave rows 400 and 423 uncovered (test).
6. Real Bevy captures at 640 x 360 and 1920 x 1080 (window hash equals headless hash; two passes byte-identical): the room reads as a gym at 1x (mirrors, weights, bench, mats, tubes, turnstile); the boss, Carl and Donut stay the brightest, highest-contrast shapes; a cracked mirror after a crash reads as a cracked mirror; the gate reads capped; the exit door is closed during the fight and open after the defeat.
7. No text, logos or third-party designs; no pixel derived from the concept images.

## 12. Execution list

1. Draw the remaining pieces of section 4 in `scripts/author_gym01.py` (atlases `floor01_gym_wall`, `floor01_gym_props`, `floor01_gym_fixture`, `floor01_gym_floor`, `floor01_gym_exit`), sources plus `.ase`.
2. Contract `assets/source/terrain/floor01/floor01_gym_kit.json` extending the opening kit (which extends ENV-01): atlases, pieces, lights (section 7), the `gym_exit` state group and the `arena_dressing` block (removals and placements of section 4).
3. Manifests `assets/manifests/floor01_gym_*.json` (decor_atlas, `prototype_fixture_unapproved`, provenance).
4. `src/gym_kit.rs`: build the arena kit from the gym contract with the dressing applied; `boss_scene::load_arena` uses it. `src/gym_art.rs` + the Bevy half: the exit state pair.
5. Tests: Rust (solids identical, crack zones, walking line, exit state), Python (determinism, sizes, palette, contract validation).
6. Evidence under `docs/evidence/area-gym-01/`: contact sheet, enlarged pieces, real Bevy captures at 1x and 3x (fight, crash crack, defeat with the open exit), README, `SHA256SUMS`.

## 13. Files

| Path | Content |
| --- | --- |
| `scripts/author_gym01.py` | authoring record of every piece (hero key included) |
| `assets/source/props/floor01/gym_*.grid.json` / `.ase` | sources |
| `assets/source/terrain/floor01/floor01_gym_kit.json` | kit contract and arena dressing |
| `assets/manifests/floor01_gym_*.json` | manifests |
| `src/gym_kit.rs`, `src/gym_art.rs`, `src/bin/floor01_scene/gym_art.rs` | arena dressing and exit presenter |
| `scripts/make_gym01_evidence.sh`, `scripts/make_gym01_sheets.py` | evidence |
| `docs/evidence/area-gym-01/` | evidence |

## 14. Open questions and adopted defaults

| ID | Question | Adopted default |
| --- | --- | --- |
| GYM-AD1 | Where do the mirrors go? | On the back wall at both ends, reaching each crash wall, because the `mirror_crack` VFX is anchored at the wall x and 40 px up; the centre keeps the catacomb arch (the gallery) as the "stone arch transition" of zone C |
| GYM-AD2 | The reused `guild_portal_closed` exit marker | Not drawn in the arena; a gym exit door in the end-wall strip (x 620..640), closed until `Boss::released()`, open after. It is narrower than the 28 px exit rect so the largest crack frame (to x 624) touches it by 4 px at most |
| GYM-AD3 | Magenta without a palette colour | The accent is light data (glow and pool colour); the pixel tube stays near-white. The kit palette is not extended |
| GYM-AD4 | The ceiling | A riveted girder on the collider line makes the jump ceiling visible and carries the tubes; the lintel cap is its left bearing over the gate (the #263 review's gate-top finding, see Authorities) |
| GYM-AD5 | Torches in a gym | Kept: the catacomb's amber torches stay the key light (the panorama's amber lamps); the tubes are the gym's own cold fill |
| GYM-AD6 | Banner, posters, punching bag, lockers of the concept | Not drawn: lettering does not survive 1x, posters risk real likenesses, a static bag would read as interactive. A later order can add them |
| GYM-AD7 | The door state | A pure function of `Boss::released()` in a presenter; no animation (the sim keeps no release tick for a swing) |
| GYM-AD8 | The exit lights | The lighting pass reads no piece state, so the warm `door_lamp` and `door_leak` floor pool on the exit are always on (the same choice as the rooms 2/4/6 exits, EXIT-AD4); the passage art carries the open/closed difference |

## 15. Execution record

**Silhouette gate (two rounds, fresh reviewer each time).** Key 1 (dark cold glass with a heavy frame and mullion): **FAIL**; it read as a window or display case, its sheen vanished at 1x, and the crack looked like a spark. The key was redesigned as above: silvered glass, mirrored torch glows, a warm floor band, a reflected silhouette, bold sheens, a thin frame and a hairline seam. Key 2: **PASS** (borderline). Its two fixes were applied before freezing: the streaks dropped to light blue-grey (two per panel), and the glass base moved from the saturated `gal` ramp to the greyer `stone_cold` ramp so it separates from the night-blue gallery in the arch. Reviewer note, out of scope here: the `mirror_crack` sprite itself (boss VFX order) reads more as a spark than a spiderweb. A radial redraw is a follow-up for that order.

**Review round 1 (independent, PASS-with-fixes).** Two design defects, fixed and sent to a narrow re-review. (1) The glass's block noise drew a night-city skyline and read as a window again. The glass is now a smooth dithered ramp with faint mirrored brick of this room, the lower two thirds warmed toward the room, and glows that are never level. (2) The open exit could not be told from the closed one at 1x. The open state is now a warm lamp-lit passage with a bigger lit lamp, and a warm floor leak light marks the exit (GYM-AD8). Fix-forward items also applied: grey tape on the mats and a lighter mat base, a stronger magenta accent (0.30 / glow 0.18), the comments, and the gate-top finding reference.

**Atlases** (all `floor01-catacomb-v2`, 1-bit alpha, every piece with at least one transparent pixel as the kit validator requires): `floor01_gym_wall` 144 x 84 (23 colours), `floor01_gym_fixture` 80 x 10 (10), `floor01_gym_props` 200 x 140 (29), `floor01_gym_floor` 96 x 22 (8), `floor01_gym_exit` 40 x 96 (18). `scripts/env_kit.py validate --contract assets/source/terrain/floor01/floor01_gym_kit.json` passes.

**Wiring.** `boss_scene::load_arena` builds the arena through `gym_kit::build_arena_kit` (the gym contract, the catacomb composition, then the `arena_dressing` block). `boss_scene::load_arena_undressed` is the plain composition the tests compare against. The binary marks the two exit door pieces at spawn, and `gym_art::sync_gym_art` shows the one that `gym_art::piece_visible` names for the boss state.

**Tests.** Rust: the dressing leaves the solids, spawn, exit, views, walking line, blink barriers, tiles and actor z identical. The crack zones lie on glass, and no prop covers them (the exit door may touch the largest frame by 4 px at most, GYM-AD2). Props stand on the floor back line, the mats stay on the floor surface, and the girder's bottom row is the ceiling line. The light budget holds. The dressed and plain arenas run `boss01_full_clear_solo` and `boss01_party_defeat_retry` to the same state hash on every tick, with the presenter reading the boss each tick, and the door opens on the release tick. Python (`tests/test_art_gym01.py`): sources, contract and manifests are deterministic; sizes, palette, decor-only and neutral ids hold; the dressing layers and lines hold; lights sit on the fixtures only; the contract validates. Every existing boss golden is unchanged.

**Evidence:** [docs/evidence/area-gym-01/](../evidence/area-gym-01/README.md).
