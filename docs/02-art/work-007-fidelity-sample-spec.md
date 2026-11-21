# WORK-007: native-resolution art fidelity sample (design spec)

**Status:** design and keys frozen (phase 1); phase 2 executed (tiles, contract, scene, frames, sheets; the Bevy capture E6 is a recorded blocker, see the evidence README); independent Opus art review passed with fix-now items, which were applied by Sonnet (section 9). The nine 16×16 material keys and the thirteen derived tiles exist as an editable pixel grid and `.ase`; all 22 frames are frozen in the fixture. The owner's art review comes at the end. Nothing is wired into a Bevy binary yet.
**Issue:** [#15](https://github.com/pando85/dungeon-floorbreak/issues/15) (WORK-007). **Visual target:** [VT-001 Inspection Corridor](visual-target-inspection-corridor.md) (mood and composition only).
**Builds on:** the owner-approved [ENV-01 kit](floor-01-catacomb-kit.md) (palette `floor01-catacomb-v2`, A1 frame of [ADR-0007](../05-decisions/ADR-0007-rendering-resolution-lighting.md)), the owner-approved [ENV-02 opening pieces](env-02-opening-spec.md), [VFX-01](vfx-01-spec.md), the approved Carl 52 px locomotion atlas ([ANIM-CARL-01](carl-locomotion-anim-01.md)) and the CF02-F microarena perch values ([CF02-F evidence](../evidence/cf02/cf02-c2f/README.md)). Material classes follow [destruction.md](../03-tech/destruction.md).
**Routing:** difficulty score 1 (new design; 16 px, simple construction, no VFX), class **Easy**, route **C with the silhouette gate** ([routing by difficulty](../07-agents/pixel-art-pipeline.md#routing-by-difficulty)): the design step writes this spec and the keys; phase 2 draws the derived tiles and the evidence; a fresh gate checks the keys at 1× in the lit scene before the derived tiles are drawn.

## 1. Gap table: issue #15 against what exists (2026-11-21)

| #15 checkbox | Exists (PR) | Gap | Closed by |
| --- | --- | --- | --- |
| Editable 16×16 tiles: **cold stone** | ENV-01 (#26, owner-approved): brick height bands with the cold `brick_top` band, `ceiling_fill`, `cornice`, `fill_*`, `wall_end_*`, the pale `base_course`; ramps `stone_cold`, `brick_cold` | none for the sample | reuse |
| Editable 16×16 tiles: **cracked/breakable wooden barrier** | ENV-02 (#48, owner-approved): raking shore (4 states), timber sets (2), full-height barricade (2). These are **pieces** (80 to 136 px), not 16×16 cells | no 16×16 wood **cell** family for the destruction grid | this spec: `timber_boards_*` keys (W007-D3) |
| Editable 16×16 tiles: **reinforced metal** | WORK-008 (#13) placeholder fixture only (`assets/source/terrain/dungeon/materials.ase`, palette `dungeon-proto-v0`); ENV-02 `maint_hatch` is a 32×32 prop | no production tile | this spec: `iron_plate_*` keys (W007-D2) |
| Editable 16×16 tiles: **glass** | WORK-008 (#13) placeholder only | no production tile; plain glass does not fit the catacomb canon | this spec: `glass_leaded_*` keys (W007-D1) |
| One 48 to 56 px character, 4-frame idle, 6-frame run, no shotgun | Carl 52 px: idle 4 frames (atlas 0 to 3), run 6 frames (4 to 9) in `carl_h52_locomotion.grid.json` (#34, integrated in #42); no weapon | the "IP-neutral stand-in" wording is superseded by `ART-PUBLICATION-ALL-V1` | reuse (W007-D6) |
| 8 to 12-frame explosion with anticipation, peak, debris, smoke | VFX-01 (#76): 12 frames, `bomb_flight` 3 (anticipation: lit fuse) and `burst` 9 (flash, fireball, peak, cooling, smoke fade); thrown debris chunks inside the burst | no separate debris layer at a breach | compose ENV-02 `timber_splinter_burst` (4 frames) over the burst (W007-D7) |
| One lower breach + upper route, 640×360, conservative parallax | Pieces exist: ENV-02 breakables, CF02-F perches (#158: slab top 80 px, the upper route in the microarena). No single frame shows both routes | composed sample missing | section 4 composition plan, phase 2 |
| Export through WORK-008 CI; exact 1× and 2× frames; small animation contact sheet | pipeline exists (#13: `.ase` sources, `scripts/export_assets.sh`, `scripts/validate_assets.py`) | no export, frames or contact sheet for the sample | phase 2, tasks E2 to E6 |
| Human artistic review before calling any art final | owner reviews at the end ([AGENTS.md](../../AGENTS.md#model-routing-for-art-work)) | pending | after phase 2 and the independent review |
| No source art copied from the two concept illustrations | all kit art is rule-drawn or hand-placed on the grid | none | holds for this spec (section 7) |

## 2. Adopted defaults

| ID | Decision | Why |
| --- | --- | --- |
| **W007-D1** | Floor 1 **glass** is **leaded quarry glass**: a lattice of 2 px lead came holding diamond quarries, one quarry per 16 px tile (vertices on the tile edge midpoints, quarter quarries in the corners), standing as a glazed screen in a stone opening (a lamp-room partition of the upper gallery). Panes use the cold `gal` ramp. | Plain sheet glass has no place in a dry brick catacomb. Leaded lights are period-plausible (chapel and lamp-room glazing). The `gal` blue is the same cold light the arch windows show, so a pane reads as cold air behind. Green bottle glass was rejected because it reads as slime, against the Floor 1 "no sewer, no toxic" lock. An 8 px quarry pitch was drawn first and rejected: it reads as chain-link at 1×. |
| **W007-D2** | **Reinforced metal** is **riveted iron plate** (the `maint_hatch` iron family): warm-grey `iron` ramp, a 1 px lit top-left bevel, a dark shared seam at the bottom and right, four rivets, one short cold specular streak (`stone_cold_01/02`). It armours critical geometry: perch slabs, lintels, sills. | Reads as metal by pattern (bevel, rivets) and by hue (the only grey, with a cold sheen) against warm brick and orange wood. The streak follows the art direction's "cold blue metal". |
| **W007-D2a** | `iron_plate_damaged` is **cosmetic only**: a blast dent (shadowed upper wall, lit lower lip), heat-blued temper tint (`copper_cold_01`), soot fanned upward, one popped rivet, two bright shrapnel scratches. Its opaque outline is the intact one, 16×16 full. | Reinforced is immune in v1 ([destruction.md](../03-tech/destruction.md#material-profile--qualitative-initial-values)). The scorch tells the player "the blast reached this and did nothing". |
| **W007-D3** | The breakable wooden barrier gets a 16×16 **cell** family: three nailed boards per tile (4, 5, 4 px with 1 px dark gaps), butt joints with lit end grain, nails, grain, one dry-rot spot. | The destruction grid needs a wood cell class. The ENV-02 pieces stay the authored props; the cells serve boarded openings. |
| **W007-D4** | **Broken** states are **edge remnants** with 1-bit alpha: `glass_leaded_broken_top` (the two corner quarries held by the top channel, jagged shards under the came V), `glass_leaded_broken_sill` (the bottom channel, the came V and standing shards), `timber_boards_broken_l` (board stubs nailed to the left jamb). The **inside cells of a broken opening are empty** (no tile). | VT-001: the damaged barrier must show what is now traversable. A full-cell "broken" tile repeated over the opening would still read as a barrier. |
| **W007-D5** | Keys are drawn at the **mid** tone. `_dim` and `_lit` variants are derived by a one-step ramp shift (rule in task E1). | Same tone-level scheme as the ENV-01 deck and brick bands. |
| **W007-D6** | The sample's character is the **approved Carl 52 px** (idle 4, run 6), not an IP-neutral stand-in. | `ART-PUBLICATION-ALL-V1` covers production art of the fan-concept characters in this repo ([rights register](../04-production/rights-and-release.md)). Engine and data contracts stay IP-neutral. |
| **W007-D7** | The **debris phase** is VFX-01 `burst` plus ENV-02 `timber_splinter_burst`. No new VFX. | VFX-01 deliberately left breach debris out of scope; the splinter burst is the approved wood-debris sprite. |
| **W007-D8** | The composed sample is a **scene composition** (`work007-sample-v1`, pixel rects), **not** an `env-kit-layout-v2`. | The layout validator allows one walking surface per floor column. The sample needs two (lane and perch), as the CF02 microarena does. |
| **W007-D9** | Perch slab: top 80 px above the walking line (CF02-Q45), **16 px** thick, so the lane under it is 64 px clear (CF02-F: 68). Carl's 52 px still clears by 12. | The slab is one row of `iron_plate` tiles; a 12 px slab would need a cut tile. |

## 3. The keys (frozen)

Source: [`assets/source/terrain/floor01/materials.grid.json`](../../assets/source/terrain/floor01/materials.grid.json) (`pixel-grid-v1`, canvas 16×16, one frame per tile, layer `main`; tags `iron_plate` 0 to 1, `glass_leaded` 2 to 5, `timber_boards` 6 to 8) and its `.ase` built by `scripts/pixel_grid_to_ase.py build`. Frozen hashes: [`tests/fixtures/work-007-materials_keys.json`](../../tests/fixtures/work-007-materials_keys.json) (`pixelart-keys-v1`, one SHA-256 per frame: frames 0 to 8 are the Opus keys, frames 9 to 21 are the derived tiles frozen after the independent review, section 9). Phase 2 must not change frames 0 to 8; a design problem goes back to design.

| Frame | Tile id | Material | Opaque | Collidable when placed | Notes |
| --- | --- | --- | --- | --- | --- |
| 0 | `iron_plate_intact` | reinforced | 16×16 full | yes, `[0, 0, 16, 16]` | tiles seamlessly in 2D: the seam is the bottom row and right column (`iron_00`) |
| 1 | `iron_plate_damaged` | reinforced | 16×16 full | yes, same rect | cosmetic only (W007-D2a) |
| 2 | `glass_leaded_intact` | glass | 16×16 full | yes | lattice phase: came where `(x + y − 8) mod 16 ∈ {0, 15}` or `(x − y − 8) mod 16 ∈ {0, 15}`; a glint streak (`gal_10`, `gal_08`) in the upper half of each quarry |
| 3 | `glass_leaded_cracked` | glass | 16×16 full | yes | star crack from (9, 7); the lower point of the centre quarry is gone (`gal_01` void with a lit break edge); a hairline in the top-right quarry |
| 4 | `glass_leaded_broken_top` | glass | partial, rows 0 to 9 | no | top row of a broken screen |
| 5 | `glass_leaded_broken_sill` | glass | partial, rows 6 to 15 | no | bottom row of a broken screen; it stands on the sill |
| 6 | `timber_boards_intact` | wood | 16×16 full | yes | boards at y 0–3, 5–9, 11–14; joints at x 11 (top board) and x 4 (bottom board) |
| 7 | `timber_boards_cracked` | wood | 16×16 full | yes | the middle board splits along the grain (pale `stone_06` fibres), a chip torn from the bottom board, a pulled nail lifts the top board's end |
| 8 | `timber_boards_broken_l` | wood | partial, columns 0 to 9 | no | stubs at the left jamb, pale fracture ends, one nail each |

**Palette** (28 colours, all `floor01-catacomb-v2` entries, no new colours): iron `iron_00..06`, `stone_cold_01/02/04`, `copper_cold_01`; glass `gal_01/03/04/05/06/08/10` plus the `iron_01..03` came; wood `wood_00..05`, `floor_07`, `stone_06`, `stone_08`, `mortar_02`. Accent keys (exempt from the orphan rule): `iron_06` rivet heads, `stone_cold_04` scratches, `gal_10` glints, `stone_08` fresh splinter tips. Lint at freeze: 0 isolated pixels in every frame; orphans 4 to 19 per frame.

**Material read** (the four classes side by side at 1×): brick = warm red-brown courses; wood = orange horizontal boards with dark gaps; iron = flat grey plate, rivets, one cold glint; glass = the bluest, brightest surface, a diamond lattice. Each pair differs in **pattern and hue**, as the art rules require.

## 4. Composition plan (the sample frames)

Coordinates are layout px, origin top-left, y down. Layout 640×400; views: `640x360` = `[0, 20, 640, 360]`, `640x400` = `[0, 0, 640, 400]`. Base walking line y 308 (ENV-01).

**Base corridor:** ENV-01 grammar A over the full width: ceiling rows 0 and 1, brick bands, base course (row 17), deck rows 18 and 19, face rows 20 and 21. Left wall torch with its flame at x 88 and `torch_pool_a`; right wall torch with its flame at x 560 and `torch_pool_b_m` (472 px apart). No arch, no copper. Parallax: none in this frame (no arch window), which keeps the parallax budget at zero.

**The divider** (x 384 to 432, three columns): one wall with two stacked openings and an iron walkway between them. Everything below the masonry sits on a 4 px row offset, so the boards end exactly on the walking line.

| y | Content | Tiles | F0 | F1 | F2 |
| --- | --- | --- | --- | --- | --- |
| 32–148 | masonry pier | ENV-01 `wall_end_l`, `wall_fill`, `wall_end_r` on rows 2 to 8, plus a 4 px course cut from the same tiles at y 144–148 | intact | intact | intact |
| 148–164 | lintel | 3 × `iron_plate` | intact | intact | intact |
| 164–228 | **upper opening**: glazed screen, 3 × 4 cells, 64 px clear | `glass_leaded_*` | intact | top two rows intact, bottom two rows `cracked` | row 1 `broken_top`, rows 2 and 3 empty, row 4 `broken_sill` |
| 228–244 | **perch walkway** x 272 to 528 (16 plates; the left end is cantilevered from the divider) | `iron_plate` | intact | the 3 plates at x 384 to 432 `damaged` | same as F1 (damage persists, nothing opened) |
| 244–308 | **lower opening**: boarded, 3 × 4 cells, 64 px clear | `timber_boards_*` | intact | rows 3 and 4 `cracked`, rows 1 and 2 intact | column 1 `broken_l` ×4, column 2 empty, column 3 `broken_r` ×4 (task E1) |

**Actors and effects:**

| Frame | Beat | Carl (approved 52 px atlas, pivot on the surface − 1) | Effects |
| --- | --- | --- | --- |
| F0 anticipation | Carl has left a lit bomb at the boards and flees his own fuse | `run` atlas frame 6, facing left, pivot (256, 307) | VFX-01 `bomb_flight` frame 0, pivot (400, 300) on the deck |
| F1 peak | one blast: wood cracks, glass cracks, iron scorches, stone holds | `run` atlas frame 8, facing left, pivot (176, 307): 232 px from the burst | VFX-01 `burst` peak (atlas frame 5) centred (408, 284); `timber_splinter_burst` frame 1 centred (392, 276) |
| F2 two routes | a moment later: the lower breach is open, the cracked screen has fallen out, the iron and masonry are unchanged; Carl has taken the upper route | `idle` atlas frame 0, facing right, on the perch, pivot (336, 227) | VFX-01 `burst` smoke fade (atlas frame 10) centred (408, 260) |

Optional in F2, for the enemy readability question of VT-001: one GOB-01 goblin (h36) in the lane beyond the breach, pivot (488, 307), facing left. Leave it out if it crowds the breach at 1×.

**Lighting:** the ENV-01 `floor01_torchlit_a1` profile of the sample layout, unchanged. No runtime light on the burst (VFX-01 section 5: the sprite reads on its own).

**Route check** (both must hold, phase 2 tests it): the lower route runs along the lane at y 308 through x 384 to 432 once the boards are broken (64 px clear, Carl 52); the upper route is a jump from the lane onto the perch at x 272 or later (80 px, inside Carl's 92 px apex), then along y 228 through the broken screen (64 px clear) to x 528 and down to the lane. Stone and iron cells never change.

## 5. Acceptance targets (independent review)

1. **Material identity at 1×:** in the F1 and F2 frames at 640×360, each of the four materials is named correctly from the 1× frame alone, and again from a greyscale copy (pattern, not only hue).
2. **Tiling:** every full key tiles 3×3 without an unplanned seam; the glass lattice phase and the board rows continue across tile edges (tested).
3. **State read:** `iron_plate_damaged` reads as "hit, still solid"; `glass_leaded_cracked` and `timber_boards_cracked` read as "about to go"; the F2 openings read as passable, the iron and masonry as unchanged.
4. **Palette and lint:** all colours are `floor01-catacomb-v2` entries, at most 63 per atlas; 0 isolated pixels; at most 20 orphans per tile (accent keys exempt); each broken-state edge remnant is **at most 50 % opaque** (review ruling Q1, section 9; measured 47.7, 43.4, 35.2 and 25.0 %).
5. **Staging:** in F1 no VFX pixel overlaps Carl, his silhouette stays readable at 1× and on a 7-inch handheld; the burst does not hide the breach outline in F2.
6. **Geometry:** clear heights and the perch height of section 4 hold (tested from the scene file).
7. **Exact frames:** each 2× frame is a nearest-neighbour ×2 of its 1× frame, pixel for pixel (tested); 3× is 1920×1080 from 640×360.
8. **Mood:** compared with VT-001 for mood and readability only (cold violet stone, warm torch and blast, grimy machinery kept sparse), not for pixel similarity.

## 6. Execution brief (phase 2)

No design decisions are needed below; anything that seems to need one goes back to design. Keep scratch files in your own subfolder. Do not edit `docs/04-production/backlog.md`.

- **E0 Gate first.** Render the nine keys at 1× in the lit F1/F2 frames (tasks E3 can run on the keys alone) and request the silhouette gate before drawing E1.
- **E1 Derived tiles** (same grid, new frames after frame 8; the frozen frames 0 to 8 stay byte-identical):
  - `timber_boards_broken_r`: stubs at the right jamb, **redrawn, not mirrored** (light still from the top-left: lit board tops, fracture faces on the left end of each stub, nails at x 14). Stub lengths 5, 3 and 7 px.
  - Tone variants `_dim` and `_lit` of frames 0, 1, 2, 3, 6, 7: shift every non-accent key one index down (`_dim`) or up (`_lit`) **within its own ramp**, clamped at the ramp ends; accent keys unchanged. **A tone shift never lands a non-accent key on an accent colour of its own ramp: that key keeps its mid index** (review ruling Q3, section 9; for example `iron_05` stays `iron_05` in the lit iron tiles, so the rivet heads `iron_06` keep their pop). Exception: `_lit` glass turns **only the top pixel of each glint streak** into `gal_warm_03` (a torch reflection), nothing else; a streak is an 8-connected group of `gal_10` pixels, and the corner glint that continues into the neighbouring tile (col 15 row 13 with col 0 row 12) counts as one streak.
  - Record every new frame in the frozen-key fixture only after the independent review passes (done after the review fixes: frames 9 to 21 are in the fixture).
- **E2 Contract and export:** an `env-kit-v2` extension contract `assets/source/terrain/floor01/floor01_materials_kit.json` with `"extends"` set to the ENV-01 contract (same rules as ENV-02: no redeclared palette or layers, new atlas name `floor01_materials`). Tiles carry `material` (`reinforced`, `glass`, `wood`), `state` and `collidable`/`collider` as in section 3; a `state_groups` entry per family. A manifest `assets/manifests/floor01_materials.json` (`review_status: prototype_fixture_unapproved`, `grid_source`, provenance as section 7). Export through `bash scripts/export_assets.sh`; validate with `python3 scripts/validate_assets.py`. The WORK-008 fixture `assets/source/terrain/dungeon/materials.ase` stays untouched (it is the pipeline fixture).
- **E3 Scene file and composer:** `assets/levels/floor01/work007_fidelity_sample.json` (`work007-sample-v1`): the base corridor tiles, the divider cells per frame, actor and VFX placements and the two views, all from section 4. Compose F0, F1, F2 with the existing A1 tools (`scripts/env_kit.py`, `scripts/env_kit_light.py`, the `scripts/pixelart` scene helpers), no bespoke lighting code.
- **E4 Frames:** per frame and view: unlit 1×, lit 1×, lit 2× (1280×720 / 1280×800) and lit 3× of the 640×360 view (1920×1080), all nearest-neighbour from the 1× frame.
- **E5 Contact sheets:** (a) the materials sheet: every tile at 1× and 4×, each full key also 3×3 tiled, states left to right (intact → cracked/damaged → broken); (b) the small animation sheet: Carl idle 4 and run 6, VFX-01 `bomb_flight` and `burst`, `timber_splinter_burst`, at 1× and 3×.
- **E6 Bevy capture:** a presentation-only `floor01_scene --work007-sample` flag that spawns the scene file statically (no gameplay, no physics change; every golden stays byte-identical) and the captures at 640×360, 640×400 and 1920×1080 through the existing capture scripts. If this needs more than static sprite placement, record the actual blocker in the PR and keep the Pillow frames as the review evidence.
- **E7 Tests:** `tests/test_art_work007_materials.py`: frozen keys; palette subset of the kit and at most 63 colours; 0 isolated pixels; iron intact and damaged both 16×16 opaque with identical alpha; broken tiles at most 50 % opaque (ruling Q1); glass lattice and board-row continuity across tile edges; the scene file's clear heights, perch height and the stone/iron cells unchanged between F0 and F2; 2× equals nearest ×2 of 1×.
- **E8 Evidence and records:** `docs/evidence/work-007/README.md` (frame table, contact sheets, validator report, what was not run), `SHA256SUMS` over the committed evidence files, the new rows in the art asset tables (the floor01 kit doc's contents and the evidence index). Tick the VT-001 review-status boxes only for evidence that exists. Run `python3 scripts/check_docs.py`, `python3 scripts/validate_assets.py`, `pytest tests/test_art_work007_materials.py tests/test_art_env_kit.py`.

## 7. Rights and provenance

Original pixel art, placed pixel by pixel on the 16 px grid from written rules and hand-set coordinates; no image-model raster output, and nothing copied, cropped, traced or resampled from the two mood images or any third-party asset. All colours are entries of the project's own `floor01-catacomb-v2` palette. The leaded-glass lattice, riveted plate and nailed boards are generic period construction, not a copy of any game's tiles. Frames that show Carl use the approved Carl atlas under the owner-reported `ART-PUBLICATION-ALL-V1` scope in [rights-and-release.md](../04-production/rights-and-release.md); that scope is not independently verified, is not a public reuse licence and grants no right to distribute a game or an asset pack. Manifests stay `prototype_fixture_unapproved`.

Design, spec and the nine keys: Claude Opus (claude-opus-5-5), 2026-11-21. Execution (phase 2): Claude Sonnet (claude-sonnet-5-5), 2026-11-21; results, findings and what was not run: [evidence](../evidence/work-007/README.md). Independent review: a separate Claude Opus agent (claude-opus-5-5), 2026-11-21, verdict PASS WITH FIX-NOW (section 9); fix-now items applied by Claude Sonnet (claude-sonnet-5-5). Owner art review: at the end.

## 8. Open questions (non-blocking; defaults above apply)

1. **Gameplay material mapping.** `src/prototype.rs` has `Stone`, `Steel`, `Catwalk`, `Timber`; destruction.md lists `Glass`, `Metal`, `Reinforced`. The tiles carry an art-side `material` tag only. Which gameplay class `iron_plate` maps to (`Steel` or a new `Reinforced`) and when `Glass` enters the code are integrator decisions.
2. **Warm glass.** Should lit glass near a torch take a warm reflection (the E1 `_lit` exception), or stay cold everywhere? Even with only the top pixel of each glint warmed, `gal_warm_03` is darker than the `gal_10` glint it replaces; if warm glass stays, the designer should pick a warm entry of matching value or drop the exception (review follow-up U-5).
3. **Boards versus the ENV-02 barricade.** Which one the opening route uses for its lane blocker stays the integrator's call; both are approved or proposed art of the same timber.

## 9. Independent review (PR #163, 2026-11-21)

Reviewer: a separate Claude Opus agent (claude-opus-5-5), one round, against sections 3 to 5. Verdict **PASS WITH FIX-NOW**, no blockers. The executor's open questions were ruled as follows; the spec text above already carries the rulings.

- **Q1, remnant limit (design ruling):** the 45 % figure was a heuristic to make remnants read as edges, not as a barrier. The limit is **50 % for all four remnants** (measured 47.7, 43.4, 35.2, 25.0 %). `glass_leaded_broken_top` is accepted as drawn; both glass remnants keep a full-width lead channel row that is frame, not glass. No redraw.
- **Q2, wood against brick in greyscale:** accepted for this sample; a top-tier design follow-up (U-1) before wood cells enter a real level.
- **Q3, lit iron (design ruling):** a tone shift never lands a non-accent key on an accent colour of its own ramp; the key keeps its mid index. `iron_05` stays `iron_05` in `_lit` iron (16 and 19 px), and the rivet heads keep their pop.
- **Q4, Bevy capture E6 deferred:** accepted (E6 allows the Pillow fallback); follow-up U-2.
- **Q5, `material_tiles` contract section:** accepted as an additive section, no ADR needed.

Fix-now items applied by Claude Sonnet: F-1 lit glass warms only the top pixel of each glint streak; F-2 the Q3 rule in `_dim` and `_lit`; F-3 the 50 % remnant limit in the tests; F-4 frames 9 to 21 frozen in the fixture; F-5 records refresh; F-6 the 1x tile beside each tone-row tile on the materials sheet. Non-blocking follow-ups U-1 to U-7 are listed in the PR body and are not done here.
