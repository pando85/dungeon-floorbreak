# WORK-007 evidence: native-resolution fidelity sample (phase 2)

Review renders for the [WORK-007 fidelity sample spec](../../02-art/work-007-fidelity-sample-spec.md) (issue #15). Status: **phase 2 drawn and composed (Claude Sonnet) on the Opus keys; independent Opus review done (PASS WITH FIX-NOW), fix-now items applied (Claude Sonnet); owner art review at the end. Nothing here is approved.** Design, spec and the nine keys by Claude Opus (claude-opus-5-5), execution and the review fixes by Claude Sonnet, review by a separate Claude Opus agent. See [Review](#review).

The frames are **Pillow compositions** from the exported atlases: the unlit layers come from `scripts/env_kit.py`, the lighting pass from `scripts/env_kit_light.py` (profile `floor01_torchlit_a1`, read unchanged from the ENV-01 sample layout), and nothing is wired into a Bevy binary. They are not Bevy captures and not gameplay evidence. See [What was not run](#what-was-not-run).

## Regenerating

```sh
python3 scripts/author_work007_materials.py derive     # frames 9 to 21 of the grid (0 to 8 untouched)
python3 scripts/author_work007_materials.py contract   # floor01_materials_kit.json
bash scripts/export_assets.sh                           # .ase, atlas, validators, previews
python3 scripts/work007_sample.py all                   # scene file, frames, both sheets
(cd docs/evidence/work-007 && sha256sum *.png > SHA256SUMS)
```

`SHA256SUMS` lists every PNG here and `python3 scripts/check_docs.py` verifies it. Every 2x and 3x frame is a nearest-neighbour copy of its 1x frame (the light is computed once, at 1x), tested pixel for pixel.

## Frames (scene file `assets/levels/floor01/work007_fidelity_sample.json`, `work007-sample-v1`)

| State | Beat | Carl (approved 52 px atlas) | Effects |
| --- | --- | --- | --- |
| F0 anticipation | every cell intact | `run` atlas frame 6, facing left, pivot (256, 307) | VFX-01 `bomb_flight` frame 0 at (400, 300) |
| F1 peak | the bottom two rows of the screen and of the boards crack, the three plates over the burst are scorched, stone holds | `run` atlas frame 8, facing left, pivot (176, 307), 232 px from the burst | VFX-01 `burst` atlas frame 5 at (408, 284) plus `timber_splinter_burst` frame 1 at (392, 276) |
| F2 two routes | lower breach open (edge remnants at the jambs), the cracked screen fallen out, iron and masonry unchanged | `idle` atlas frame 0, facing right, on the perch, pivot (336, 227) | VFX-01 `burst` smoke fade (atlas frame 10) at (408, 260); one GOB-01 h36 goblin in the lane, pivot (488, 307), facing left (the spec's optional enemy-readability actor: it does not crowd the breach, so it stays) |

For each state and view (`640x360` = `[0, 20, 640, 360]`, `640x400` = `[0, 0, 640, 400]`): `*_unlit_1x_*.png`, `*_lit_1x_*.png`, `*_lit_2x_*.png` (1280x720 and 1280x800), plus `*_lit_3x_640x360.png` (1920x1080).

| File pattern | What it shows |
| --- | --- |
| `work007_f0_anticipation_*`, `work007_f1_peak_*`, `work007_f2_two_routes_*` | the three states, per the table above |
| `work007_materials_sheet.png` | every material tile at 4x with its 1x tile beside it, each full key tiled 3x3 (shown at 2x), states left to right (intact, cracked or damaged, broken), then the dim, mid and lit tone rows |
| `work007_animation_sheet.png` | Carl idle 4 and run 6, VFX-01 `bomb_flight` 3 and `burst` 9, `timber_splinter_burst` 4, at 1x and 3x, frames labelled with their atlas index |

## What was added in phase 2

- `assets/source/terrain/floor01/materials.grid.json` and `.ase`: frames 9 to 21 (frames 0 to 8 are byte-identical to the frozen keys, checked against `tests/fixtures/work-007-materials_keys.json`): `timber_boards_broken_r` (redrawn, stubs 5, 3 and 7 px, nails at x 14) and the `_dim` / `_lit` variants of frames 0, 1, 2, 3, 6, 7 by a one-step ramp shift inside each palette ramp (accent keys unchanged; a shift never lands on an accent colour of its own ramp, so `iron_05` stays `iron_05` in the lit iron tiles; lit glass turns only the top pixel of each `gal_10` glint streak into `gal_warm_03`, 2 pixels in frame 18 and 3 in frame 19). All 22 frames are frozen in the fixture. 42 palette entries, all `floor01-catacomb-v2`; the atlas holds 42 opaque colours (limit 63); 0 isolated pixels; 3 to 19 orphans per frame (limit 20, accent keys exempt).
- `assets/manifests/floor01_materials.json` (`material_tileset`, `prototype_fixture_unapproved`), so `scripts/validate_assets.py` covers the atlas, and `assets/source/terrain/floor01/floor01_materials_kit.json`, an `env-kit-v2` extension of the ENV-01 contract with a `material_tiles` section (material `reinforced`, `glass` or `wood`, state, tone, collider `[0, 0, 16, 16]` for full tiles, none for edge remnants) and `state_groups` per family.
- `scripts/env_kit.py`: `resolve_extends` merges `material_tiles`, and `_check_material_tiles` validates them (frame-per-tile atlas, rect equals the exported frame, full tiles opaque and collidable, remnants 1-bit alpha without collider, one entry per frame). See [Findings](#findings-for-the-designer-and-the-reviewer).
- `scripts/author_work007_materials.py` (derivation and contract, deterministic, `check` fails when stale) and `scripts/work007_sample.py` (scene file, frames, sheets).
- `tests/test_art_work007_materials.py`: frozen keys, derived frames, palette, lint, opaque outlines, remnant opacity, lattice and board continuity, contract and validator, scene geometry (perch 80 px, 64 px clear, lower and upper routes, stone and iron unchanged), frame sizes, exact upscales, reproducibility, checksums.

## Acceptance targets (spec section 5)

| # | Target | Status |
| --- | --- | --- |
| 1 | Material identity at 1x and in greyscale | Met at 1x by my own check (brick running bond, orange-brown horizontal boards, grey riveted plate with a cold glint, the blue diamond lattice). **Greyscale: borderline for wood against brick** (see findings). The independent review decides. |
| 2 | Tiling 3x3, lattice phase and board rows continue | Met (tested; sheet shows the 3x3 blocks) |
| 3 | State read | Met by my own reading of the sheet and F1/F2 (plate dent reads as "hit, still solid"; cracks read as "about to go"; F2 openings read as passable); the reviewer confirms |
| 4 | Palette and lint | Met (42 colours, 0 isolated, at most 19 orphans). Remnant opacity limit is **50 %** for all four remnants after the review ruling Q1 (was 45 %): measured 47.7 % (`glass_leaded_broken_top`), 43.4 %, 35.2 % and 25.0 %; asserted uniformly in the tests. |
| 5 | Staging | In F1 no VFX pixel overlaps Carl (tested on the sprite alpha boxes); Carl stays readable at 1x. Not checked on a real 7-inch handheld (no device). In F2 the smoke sits over the middle column but both jamb remnants stay visible. |
| 6 | Geometry | Met (tested from the scene file) |
| 7 | Exact frames | Met (tested; 3x is 1920x1080 from 640x360) |
| 8 | Mood against VT-001 | Mood and readability only: cold stone, warm torch and blast, sparse machinery. Judged by the reviewer and the owner. |

## Findings for the designer and the reviewer

1. **E0 silhouette gate was my own check, not an independent gate.** I rendered the nine keys at 1x in lit F1/F2 before drawing E1 (with a mirrored `broken_l` standing in for the not yet drawn `broken_r`) and found no reason to change a key. The independent top-tier gate and review are still required.
2. **The 45 % rule was not met by the frozen key `glass_leaded_broken_top` (47.7 %).** Resolved by the review (Q1): the limit is 50 % for all four remnants, the key stays as drawn.
3. **Wood against brick in greyscale is close.** The boards are thin horizontal runs with few vertical joints and brick is a running bond, but in a greyscale copy of F1 the dark boards read like small bricks at a glance. Hue and pattern at 1x are clear. If the reviewer disagrees, a design change (for example wider light board tops) is a designer decision.
4. **`_lit` iron: `iron_05` shifted to `iron_06`, the rivet-head accent colour.** Resolved by the review (Q3): a tone shift never lands a non-accent key on an accent colour of its own ramp, so `iron_05` stays `iron_05` in the lit iron tiles (16 and 19 px) and the rivets keep their pop. The tone variants are not placed in the sample frames (the scene uses the mid keys under the lighting pass), so only the materials sheet changed.
5. **Tooling gap closed in `scripts/env_kit.py`.** The spec's E2 asks for tiles with `material`, `state` and `collidable`/`collider` in an `env-kit-v2` extension, but the contract's `tiles` must come from a single-frame `tileset` atlas and be fully opaque, so the partial broken states and the frame-per-tile atlas cannot be listed there. I added a separate `material_tiles` section with its own validator (the existing `tiles` rules are unchanged, and the ENV-01 and ENV-02 contracts validate as before). If the integrator prefers another shape, only that section and its validator change.
6. **The sample scene is not an `env-kit-layout`, as the spec says (W007-D8).** The composer builds an in-memory layout dict for the A1 renderer: the divider cells go into the `structure` piece layer as pixel-placed pieces, the parallax layers are dropped (this frame has none), and the lighting profile is read from `corridor_abc_sample.json` unchanged.
7. The base corridor is generated from ENV-01 tile ids (grammar A, 40 columns: the left wall end and end caps as in the ENV-01 sample layout, deck tone by distance to the torch). The left torch pool decal starts at x -32 (flame at x 88, as specified); it is clipped at the layout edge.

## What was not run

- **E6, the Bevy capture (`floor01_scene --work007-sample`): not built.** `floor01_scene` is the whole game scene (heroes, combat, HUD, audio, recorder). It has no path that places a posed Carl atlas frame, a mirrored goblin or free VFX frames at pixel positions without the simulation owning them, and its `setup` builds the kit from an `env-kit-layout`, which the sample is not (W007-D8). A static spawn therefore needs more than sprite placement (a new kit builder from `work007-sample-v1`, flip support for atlas pieces, and gating the hero, HUD and combat spawns), so I recorded the blocker instead of wiring it. The Pillow frames are the review evidence.
- No GIF (the spec asks for one only if any).
- No owner review, no handheld check (the independent review is recorded below).
- `bash scripts/export_assets.sh`, `python3 scripts/validate_assets.py`, `python3 scripts/check_docs.py` and `pytest tests/test_art_work007_materials.py tests/test_art_env_kit.py tests/test_art_env02_opening.py tests/test_art_assets.py tests/test_art_pixel_grid.py` were run on the final tree; results are in the PR body.

## Review

Independent art-director review by a separate Claude Opus agent (claude-opus-5-5), one round: **PASS WITH FIX-NOW**, no blockers. Rulings: Q1 remnant limit 50 % for all four remnants; Q2 wood against brick in greyscale accepted for this sample (follow-up U-1); Q3 a lit tone shift never lands on its own ramp's accent (`iron_05` stays in lit tiles); Q4 E6 deferred, accepted; Q5 `material_tiles` contract section accepted.

Fix-now applied by Claude Sonnet: F-1 lit glass warms only the top pixel of each glint streak (frames 18 and 19 re-derived: the lit glass keeps its `gal_10` highlight); F-2 lit iron keeps `iron_05` (frames 16 and 17); F-3 uniform 50 % remnant limit (measured values above); F-4 frames 9 to 21 frozen in `tests/fixtures/work-007-materials_keys.json`; F-5 records refreshed (PR title and body, spec section 9, this README); F-6 the 1x tile now sits beside every tile in the tone rows of the materials sheet. Regenerated evidence: `work007_materials_sheet.png` and `SHA256SUMS` (the three scene states and the animation sheet are unchanged, byte for byte); a second export run is byte-identical.

Open follow-ups, not done here: U-1 wood greyscale read (top-tier design, frozen-key change); U-2 Bevy capture as its own work order; U-3 light the showcase (design, owner game feel); U-4 upper opening visual clearance; U-5 warm glint value; U-6 F0 parked bomb read; U-7 7-inch handheld check (owner).

## Rights and provenance

Original pixel art placed on the 16 px grid from written rules and hand-set coordinates; no image-model raster output, and nothing copied, cropped, traced or resampled from the two mood images or any third-party asset. All colours are entries of `floor01-catacomb-v2`. The frames that show Carl use the approved Carl atlas under the owner-reported `ART-PUBLICATION-ALL-V1` scope in [rights-and-release.md](../../04-production/rights-and-release.md); that scope is not independently verified, is not a public reuse licence and grants no right to distribute a game or an asset pack. The goblin is the GOB-01 h36 candidate, the burst and bomb are VFX-01 and the splinters are ENV-02, all unchanged.

Models: design Claude Opus (claude-opus-5-5); execution and review fixes Claude Sonnet (claude-sonnet-5-5); independent review a separate Claude Opus agent (claude-opus-5-5), PASS WITH FIX-NOW.
