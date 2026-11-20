# ART-GOB-CF02 evidence: goblin acting and slammed sheets

Executor pass of ART-GOB-CF02 for [the CF02 acting spec](../../02-art/cf02-acting-spec.md) (sections 5, 6 and 9), the work order row in [combat-fun-02-workorder.md](../../07-agents/combat-fun-02-workorder.md). Every image here is a still or GIF composed from the committed pixel-grid sources. None is a Bevy capture or a feel approval; nothing is wired into the game. Independent review: Claude Opus, passed with fix-now items; the fix round is applied (stomped_rest band offset, wall_rest contact column x16, four key pinholes filled, text fixes).

| Sheet | Source (`.grid.json`, `.ase`, `.pixelart.json`) | Manifest | Frames |
| --- | --- | --- | --- |
| `goblin_a_h36_cf02` | `assets/source/characters/goblin/` | `assets/manifests/goblin_a_h36_cf02.json` | 5: `hesitate`, `panic` (2), `taunt` (2) |
| `goblin_b_h36_cf02` | `assets/source/characters/goblin/` | `assets/manifests/goblin_b_h36_cf02.json` | the same 5 plus the headband layer |
| `goblin_a_slammed_h36` | `assets/source/characters/goblin/death/` | `assets/manifests/goblin_a_slammed_h36.json` | 6: `slammed_stomped`, `slammed_wall`, `slammed_flung` (impact and rest each) |
| `goblin_b_slammed_h36` | `assets/source/characters/goblin/death/` | `assets/manifests/goblin_b_slammed_h36.json` | the same 6 plus the headband layer |

5 acting + 6 slammed = 11 frames per goblin (the Q73 cap, one A and one B). All four sheets are new files: the approved GOB-01 and GOB-02 sheets are untouched. 64 x 56 canvas, pivot (28, 55), every frame faces right.

## How the frames were made

`python3 scripts/author_goblin_cf02.py all` writes (and `--check` compares) the four grids, the `.ase` rebuilds, the manifests, the toolkit configs and the two frozen-key fixtures.

- **Keys, copied pixel for pixel** from the frozen `cf02_keys` (a test compares every layer): `hesitate`, `panic_flee`, `taunt_thump` and the six slammed keys.
- **`panic_flee_b`** (derived): the `panic_flee` key with the legs of GOB-01 walk frame 8 (the first leg row copied one row up to close the gap under the hem), the whole upper block and the trailing hand with the dragged club 1 px higher (club top row 40, below the shoulder line), and the raised empty arm 2 px lower (two rows of its forearm are deleted and the hand and upper forearm move down 2 and 1 px left, so the arm stays one limb).
- **`taunt_beckon`** (derived): the `taunt_thump` key with the head and neck 1 px forward and the near arm re-drawn: the upper arm hangs as before and the forearm goes forward with the palm up, a hand 6 px long and 4 px tall, fingers together, no thumb, no spur above the knuckles (a test checks width, height, a flat top run and no skin above it).
- **Goblin B**: the A body, weapon and gore cels byte for byte (tested) plus an `accessory` layer. The GOB-01 B accessory (red headband with knot and tails, ear notch, missing front tooth, 3 x 3 vest patch) moves with each head and the vest: mirrored with the head on `hesitate` and `panic`, shifted with the head on `panic_flee_b`, `taunt` and `beckon`, moved from GOB-01 B frames 18 and 19 on `wall_impact` and `flung_impact` and from the corpse frame 25 on the stomped frames and `flung_rest`; `wall_rest` is hand-painted (the dead head is lolled on the shoulder). Orphan pixels that a headband left on the body are repainted on the accessory layer in the neighbours' key (no body edit, no budget cost).
- **Background holes** (scan added after the Donut review #142 found an outline slit): every frame of the four sheets was scanned for enclosed transparent pixels (no 4-connected path to the canvas border) and one-pixel slits. The derived frames are sealed with outline `K`: `panic_flee_b` has none (the lowered arm had left 13 more pockets by the head than the key), `taunt_beckon` has only the two hem gaps its key has; the B accessory adds no hole or slit that A lacks (new ones are closed with `K` on the accessory layer). A test checks it.
- **Headband and blood**: the headband's `B` shares its RGBA with the gore layer's `B` blood_dark (its `R` is a different red from blood_wet `P`). On every B frame no headband pixel touches (8-neighbourhood) or overlaps a gore pixel; a test checks it.

## Files

| File | Content |
| --- | --- |
| `contact_sheet_acting_x1.png`, `_x3.png` | A above B, every acting frame with tag, name and ticks (x3), pivot marked |
| `contact_sheet_slammed_x1.png`, `_x3.png` | A above B at presets 4, 3, 2 and 1 (preset 1 hides the gore layer, preset 2 remaps it to dark red, presets 3 and 4 as authored; the headband stays at every preset) |
| `ab_pairs_x4.png` | A above B at x4, acting and slammed |
| `hesitate_check_1x.png`, `_3x.png` | the unlabelled idle-strip check of spec section 5: GOB-01 idle frames 0 to 3 and `hesitate` in a seeded shuffle (answer in `hesitate_check_answer.txt`) |
| `beckon_palm_1x.png` | `taunt_thump` and `taunt_beckon` of A and B at 1x, then the beckon hand of A and B at x8 (the open-palm check) |
| `blind_check_rests_a_1x.png`, `_3x.png`, `blind_check_rests_b_1x.png`, `_3x.png` | the three slammed rest frames next to `crushed_remains` and the routine corpse in a seeded shuffle, no labels (answers in `blind_check_rests_*_answer.txt`; do not read them before judging) |
| `scene_acting_a_1x.png`, `_3x.png`, `scene_acting_b_1x.png` | lit Floor 1 at gameplay scale with Carl and Donut: GOB-01 idle, hesitate, panic x2, taunt x2. The panic goblins face their flee direction (right), the others face Carl (mirrored, the runtime rule) |
| `scene_slammed_a_1x.png`, `_3x.png`, `scene_slammed_b_1x.png` | the six slammed frames; the two wall frames stand against a placeholder wall stub on their contact column (x16 for both), unflipped |
| `timing_diagram.png` | the spec section 7 tick windows of the three reactions and the three finishes |
| `gif_reactions_scene_x1.gif` | hesitate (20 ticks), panic (6 + 6 alternating) and taunt (9 + 9 alternating) in the lit scene at tick timing, unlit, exact palette |
| `toolkit/{acting_a,acting_b,slammed_a,slammed_b}/` | toolkit output: contact sheets x1 and x4, one GIF per tag at x1 and x3 (tick timing), the tick strip |

Regenerate (the lit scenes need the Floor 1 kit atlases under `generated/art`, which are git-ignored: run `bash scripts/export_assets.sh` once):

```text
python3 scripts/goblin_cf02_evidence.py
python3 -m scripts.pixelart evidence assets/source/characters/goblin/goblin_a_h36_cf02.pixelart.json --out docs/evidence/cf02-goblin/toolkit/acting_a
```

(the same for `goblin_b_h36_cf02`, `death/goblin_a_slammed_h36`, `death/goblin_b_slammed_h36`; delete the toolkit's `scene_*.png`, which are not kept). `SHA256SUMS` covers every file in this folder; `toolkit/*` carry their own lists.

## Validators

`python3 -m scripts.pixelart validate <config>` on the four configs: PASS (12 pass, 0 fail, 4 skipped) each (structure, tags, tick table against the manifest, palette, isolated and orphan pixels 0, grounded contact row 55, frozen keys, canvas margin, unique frames, bounding boxes, non-empty frames). `python3 scripts/pixel_grid_to_ase.py lint --strict` is clean on all four grids. `bash scripts/export_assets.sh` exports and validates all four manifests. `tests/test_art_goblin_cf02.py` (22 tests) checks the generator output, keys pixel for pixel, no new background holes, the derived frames, the clubs below the shoulder line, the palm, the wall contact columns, the separate gore layer, B equals A plus the accessory, the headband away from blood, the tooth gap, the manifests and the toolkit validators.

## Presenter contract (also in the manifests)

- Impact frames hold 8 ticks (after any taunt hold); rests are state-driven holds. Frame and tick table: spec section 7.
- `wall_impact` is lowest-row anchored (lowest opaque row 55, airborne): the presenter puts sprite column x16 on the wall face; `wall_rest` puts column x16 (the slumped back) on the wall face and its gore smear (x14-15, gore layer only) draws over the wall tile (review ruling, amends spec section 5 row 9). For facing left the column mirrors with the sprite (x' = 56 - x). No opaque pixel of `wall_impact` lies left of x16; in `wall_rest` only gore-layer pixels do.
- The gore layer is separate and a preset-4 reference only; preset 1 draws no blood sprites. Gib quarters and separated parts are particles (CF02-D), not frames here.

## Design questions for top-tier review

1. **`panic_flee_b` raised arm.** "2 px lower" is applied as absolute (hand and upper forearm move down 2 and 1 px left, two forearm rows are deleted) while the torso rises 1, so the arm sits 3 px lower relative to the body and the hand now touches the back of the head. The alternative (the whole arm 2 px lower with the shoulder end following) leaves a hole at the shoulder. Is the compressed arm acceptable, or should the hand clear the head?
2. **`taunt_beckon` forearm height.** The upper arm still hangs and the forearm leaves the elbow horizontally, so the open palm is at belly-high (rows 36 to 39) rather than strictly chest height (the chest is rows 31 to 38). Raising it needs a shorter upper arm, a body edit the spec did not call for.
3. **Goblin B on frames that cannot show every cue.** The missing front tooth is dropped on both panic frames (the mirrored mouth shows a single tooth, so no gap can read) and on `stomped_*` and `wall_impact`; the vest patch is dropped on `stomped_impact`, `stomped_rest` and `flung_rest` (no vest rag survives there). The headband is on every B frame; `stomped_rest`'s band is 10 px after the review fix (the skull sits at the corpse-25 offset (0, 0)).
4. **Blood and headband on B.** The one-pixel gap rule removed a few band pixels next to gore (`stomped_rest`), which stops the `stomped_rest` band two rows above the pool. The alternative is to move the gore on the B sheet, which the spec forbids ("the gore layer is separate", frozen keys).
5. **`wall_impact` contact tag.** The frame is tagged `grounded` in the toolkit config because its lowest opaque row is the pivot row (the spec's "lowest-row anchored"); the manifest records it as airborne with the presenter contract.
6. **Hesitate is a `once` tag of 20 ticks**, the Q61 stand-still, not a loop. The presenter owns what follows it.
7. **Per-frame bounding boxes** in the configs are the measured values of the frozen keys. They equal the spec table's key bounding boxes except `taunt_thump`'s right edge: the table says +15 (x43), the committed key ends at x42.

8. **Enclosed background pixels in the frozen keys** (coordinates are absolute x, y; they are the same on A and B except where B's accessory sealed them): `hesitate` (23,46), (32,46) between the hem pieces and (31,32), (32,31), (32,32), (33,30), (33,31), (33,32), (34,29), (34,32), (32,36), (32,37), (32,38), (40,32), (41,30), (41,31) between the mirrored head, the raised hand and the arm; `panic_flee` (31,35), (32,35), (33,34), (34,33), (34,34), (38,32), (39,30), (40,30), (40,31), (41,29), (42,29) between the mirrored head and the raised arm; `taunt_thump` (23,46), (32,46) between the hem pieces; (`stomped_rest` (19,45); `wall_impact` (28,37); `flung_impact` (31,42), (32,42): filled, see below.) `stomped_impact`, `wall_rest` and `flung_rest` have none. Review ruling: the `hesitate`, `panic_flee` and hem-gap pockets are intentional negative space (kept). The four ringed pinholes `stomped_rest` (19,45), `wall_impact` (28,37) and `flung_impact` (31,42), (32,42) are filled with `K` on the body layer: **key revision by ART-GOB-CF02 review** (key file, sheets and fixture updated; B inherits it). The same pinhole in the approved GOB-01 corpse 25 at (19,43) is left for a GOB-01 touch-up PR.

## Rights/Provenance

Designed by Claude Opus (ACT-DESIGN, [PR #133](https://github.com/pando85/dungeon-floorbreak/pull/133): the spec, the keys, the redesigns and rulings). Executed by Claude Sonnet (the derived frames, the B accessory layer, the sheets, manifests, configs, tests and this evidence). Independent review: Claude Opus, passed with fix-now items (fixes applied). Published under the owner-reported `ART-PUBLICATION-ALL-V1` scope ([rights register](../../04-production/rights-and-release.md)); no reuse licence, no game or asset-pack distribution right. No image-model output, no tracing, no third-party pixels.
