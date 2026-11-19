# GOB-02B-DEATH: goblin variant B death poses and death-parts sheet, design spec

**Status:** design stage complete: sources, manifests, validated export and evidence produced. Independent review pending. Nothing here is wired into Bevy. Intensity preset 4 is the default, as for A. Gameplay acceptance of any finish is still a separate owner decision.

This is a **variant spec**. Everything not listed here is the [GOB-02-DEATH A spec](gob-02-death-spec.md): canon and tone (section 1), frame, pivot and anchors (2), palette (3), pose list and ticks (4), presets (5), parts and metadata (7), spawner timing (8), and the "never hide Carl" rules. Variant B relates to the A death set the way [GOB-01](gob-01-goblin-spec.md) variant B relates to A (GOB-01 section 9, last row): **A's cels unchanged plus an `accessory` layer**.

Sources: [`goblin_b_death_h36.grid.json`](../../assets/source/characters/goblin/death/goblin_b_death_h36.grid.json) and [`goblin_b_death_parts.grid.json`](../../assets/source/characters/goblin/death/goblin_b_death_parts.grid.json), written by [`scripts/author_gob02b_death.py`](../../scripts/author_gob02b_death.py) from the committed A sources. Evidence: [docs/evidence/gob-02b-death/](../evidence/gob-02b-death/README.md).

## 1. What changes from A

- **Identity cues (GOB-01 B):** the red rag bandana (a 2-row `R`/`B` band, a knot at the back of the skull, two tails), the ear notch (2 `K`), the missing front upper tooth (`o`) and the 3 x 3 stitched vest patch (`uuu` / `uKu` / `uKu`).
- **New layer:** `accessory`, between `weapon` and `gore`. Layer order: `body`, `weapon`, `accessory`, `gore`. Gore stays on top, so every cut face, jet and pool covers the cloth as in A.
- **Unchanged, byte for byte (tested):** the `body`, `weapon` and `gore` cels of all 6 poses and 26 parts; frame sizes (64 x 56 poses, 32 x 32 part cells), pivots (28, 55) and (16, 16), tags, tick windows and durations, part ids and indices, anchors, `frame0_origin`, classes, preset filters, `pool_3` pair data and `spawner_hints`. The DEATH-C spawner reads the B manifests with the A code path; only the atlas and manifest paths differ.

**Refinement of A ruling 13.1 (flagged, not silently changed).** Ruling 13.1 put the B overlay on the head-bearing frames only ("headless frames and parts are shared"). The bandana, notch and tooth do leave with the head. However, the vest patch sits on the torso, so dropping it on the headless frames would make the goblin change vests at the moment of the cut. B therefore keeps the patch on poses 3, 4 and 5 and on `torso_upper`, and the ear notch on the `ear` part. Every other part (limbs, club, gibs, blood, decals) has an empty `accessory` layer.

**Ruling 13.1a (accepted by review).** Variant B keeps the vest patch on the headless poses (3, 4 and 5) and on `torso_upper`, and the ear notch on the `ear` part. The bandana, notch and tooth leave with the head, but the vest does not, so dropping the patch at the cut would be a costume swap. To be recorded in the A spec's rulings when B merges.

## 2. Accessory overlays per frame

| Frame | Overlay | Rule |
| --- | --- | --- |
| pose 0 `crush_contact` (key) | bandana, knot, both tails, notch, tooth, patch | **new literal rows** (`CRUSH_ACC`). The band is pressed flat under the stone line: `R` on row 34, `B` on row 35, x 23 to 34, ending at the bulging eye. The knot is at the back of the skull (x 19 to 23, rows 35 and 36). The `R` tail is squeezed flat back along the ear's underside (row 37, x 14 to 18), and the `B` tail hangs over the vest's top edge (x 21, rows 37 to 39). The notch is a 2 px `K` break in the ear's lit rim at x 7 (rows 34 and 35), 4 px from the tip. The front upper tooth at (45, 41) becomes `o`. The patch is at x 14 to 16, rows 41 to 43, on the squashed vest |
| pose 1 (in-between) | as 0 | pose 0's overlay moved by the A flatten rule (rows 33 to 45 down 3) |
| pose 2 `crushed_remains` (key) | bandana, tooth, patch | GOB-01 B frame 25's head overlay (the A heap reuses frame 25's head), mirrored with the A heap rule x' = 59 - x, **cut at row 52** (review fix E1: the tails below it are dropped), with a 2 px `K` cap at x 48 and 49, row 53; the patch at x 26 to 28, rows 49 to 51, on the pressed rag mass. No ear notch, as on GOB-01 B frame 25 |
| pose 3 `decap_contact` (key) | patch | GOB-01 B frame 16's patch (rows 37 to 39); the A pose keeps frame 16's body there |
| pose 4 (in-between) | patch | pose 3's patch moved by the A buckle rule (down 4, left 2) |
| pose 5 `headless_collapse` (key) | patch | literal rows (`HEADLESS_PATCH`, review fix E2): the 3 x 3 patch at x 32 to 34, rows 51 to 53, on the corpse's dark `r` rag mass, every pixel inside A's body silhouette and off the floor row. GOB-01 B frame 25 puts it on a dark rag flap on the floor row, where it read as a light peg under the corpse |
| part 0 `head`, 3 `head_dead` | bandana, knot, tails, tooth | GOB-01 B frame 0's overlay through the A head mask and shift (`HEAD_ROWS`, `HEAD_SHIFT`); `head_dead` is identical |
| part 1 `torso_upper` | patch | GOB-01 B frame 0's patch through the A torso mask and shift |
| part 11 `ear` | notch | GOB-01 B frame 0's two notch pixels through the A ear rectangle and shift |

**Outline closure:** where a derived overlay hangs past the A sprite, the generator adds `K` on `accessory` only (`close_acc`). This now affects only the gap between the two tails on the loose head (that gap is open in GOB-01 B frame 0). With gore hidden (preset 1), body + weapon + accessory stays a closed sprite (tested).

Frozen keys: poses 0, 2, 3 and 5 and parts 0, 1 and 2, all four layers (`tests/fixtures/gob-02b-death_keys.json`, `gob-02b-death-parts_keys.json`).

## 3. Palette and presets

- Same palette `goblin-death-floor01-v0` (20 keys). No new colour. On `accessory`, `R` and `B` are the GOB-01 B bandana (cloth), exactly as in GOB-01 B. `P` (wet blood highlight) stays **gore-only** (tested), so the bandana never carries a wet glint. Blood is the only red with `P`.
- Presets are the A switches, applied to `gore` by layer name. `accessory` is never hidden or remapped: at preset 1 the B goblin keeps its bandana, notch, tooth and patch, and at preset 2 only the gore darkens. The bandana is costume, not blood, so preset 1 still draws no blood sprite.

## 4. Acceptance targets

Automated (`python3 -m scripts.pixelart validate` on both B configs, PASS; `tests/test_art_gob02b_death.py`):

- B `body`/`weapon`/`gore` equal to A on every frame. Same canvas, pivot, palette, tags and durations. The B manifests' `death_poses.frames`, `spawner_hints` and `death_parts.parts` equal A's.
- Poses 0, 1 and 2 carry at least 6 `R` and 2 `B` of bandana and the missing tooth. Poses 3, 4 and 5 carry no `R`, `B` or `o` on `accessory` (the bandana left with the head). `head` and `head_dead` carry the bandana (at least 10 `R`) and the tooth.
- The vest patch is on all six poses and on `torso_upper`. The notch is in pose 0 and on `ear`. Only `head`, `head_dead`, `torso_upper` and `ear` have an accessory.
- The derived overlays equal the GOB-01 B overlays moved by the A rules, except the two review fixes: the remains' band stops at row 52 (the tails are dropped) and pose 5's patch is a literal 3 x 3 on the rag mass (it intentionally no longer equals GOB-01 B frame 25).
- On the headless poses (3, 4, 5) the accessory adds no pixel outside A's body and weapon silhouette. Pose 5's patch lies inside A's body, off the floor row.
- Composited at preset 4, no visible bandana `R`/`B` pixel is 4-adjacent to a gore `R`/`B`/`P` pixel on any frame (the cloth and the blood share the `R`/`B` keys, so only shape separates them).
- `accessory` uses only `K R B u o` and body tidy keys, never `P`. At most 20 colours per atlas. Preset 1 stays closed.
- Export: both atlases export with LibreSprite and are pixel-identical to the grid composites; `validate_assets.py` reports VALID.
- Every frame of every evidence GIF keeps the exact goblin skin `#7a9e36` and blood red `#b8322e`. The GIFs use an exact palette (the sequences) or the toolkit's adaptive palette with fewer than 64 colours, so nothing is quantised.

Review (human eye, at 1x and 4x in [`a_vs_b_x4.png`](../evidence/gob-02b-death/a_vs_b_x4.png) and [`scenes_1x.png`](../evidence/gob-02b-death/scenes_1x.png)):

- B reads as the B goblin from the bandana alone on the crush, the remains and the flying head (GOB-01 R9).
- The bandana reads as cloth, not as a wound: it sits on the skull top, its tails are outlined and it has no `P`. The neck cut on `head` stays a separate shape below the tails.
- The headless frames read as the same goblin through the patch, without a costume pop at the cut.

## 5. Evidence

All in [docs/evidence/gob-02b-death/](../evidence/gob-02b-death/README.md): `a_vs_b_x4.png` (A above B at presets 4 and 1, plus a 1x strip), `frames_x8.png` (the B keys and both head parts enlarged), `preset_poses_x4.png`, `preset_parts_x3.png`, `scenes_1x.png` (lit Floor 1 with Carl and Donut), `storyboard_crush.png`, `storyboard_decap.png`, `gif_crush_sequence_x1/x3.gif`, `gif_decap_sequence_x1/x3.gif`, and the toolkit's `poses/` and `parts/` contact sheets and pose GIFs. The sheets use the same seeds and placements as the A evidence.

## 6. Files

| File | What |
| --- | --- |
| `scripts/author_gob02b_death.py` | authoring record: the A sources plus the accessory rules and literal rows; `keys [--check]`, `manifests` |
| `assets/source/characters/goblin/death/goblin_b_death_h36.grid.json` / `.ase`, `goblin_b_death_parts.grid.json` / `.ase` | B sources |
| `assets/source/characters/goblin/death/goblin_b_death_*.pixelart.json` | toolkit configs (the A rules, frozen keys over four layers) |
| `assets/manifests/goblin_b_death_h36.json`, `goblin_b_death_parts.json` | manifests: the A contract with `variant: "B"`, `layers`, `variant_of` |
| `tests/fixtures/gob-02b-death_keys.json`, `gob-02b-death-parts_keys.json` | frozen keys |
| `scripts/gob02b_death_evidence.py` | evidence: the A evidence pipeline on the B sprites, plus the A-vs-B sheets |
| `tests/test_art_gob02b_death.py` | B = A + accessory, overlay derivation, closure, manifests, GIF colours |

## 7. Open questions for review

1. **Bandana next to blood on the remains.** Resolved by review fix E1: the band stops at row 52 with a dark cap and the tails are dropped, so no cloth pixel touches the seep (tested).
2. **No loose bandana part.** In a decapitation or explosion the bandana stays on the head. A separate `gib_bandana` cloth part would read well in an explosion, but appending it is a part-index contract change. Proposed as a later option, not added here.
3. **Ruling 13.1 refinement** (section 1): accepted by review, recorded as ruling 13.1a below.

**Living horizon:** art variant only. It adds no move, unlock or ability, so it does not change the [character progression horizon](../01-design/character-progression-horizon.md).

## 7a. Review fix round

The independent review passed with two execution items; both are fixed in `scripts/author_gob02b_death.py` and the sources, exports, manifests, frozen key 2 and 5, and evidence were regenerated.

- **E1, `crushed_remains`:** the bandana band stopped at row 52 with a 2 px dark `K` cap (x 48 and 49, row 53), and the tails at rows 53 to 55 (and the `K` under the gore seep) were dropped. The band keeps 6 `R` and 4 `B`. New test: no visible bandana `R`/`B` is 4-adjacent to a gore `R`/`B`/`P` pixel on any frame.
- **E2, `headless_collapse`:** the vest patch moved off the dark rag flap on the floor row onto the rag mass at x 32 to 34, rows 51 to 53, inside A's silhouette. New tests: the patch pixels lie inside the A body, off the floor row, and no headless accessory pixel is outside A's silhouette.
- Non-blocking notes N1 (pose 0 and 1 red seam length) and N2 (shared `R`/`B` keys) stay open for the Bevy capture review.

## 8. Rights and provenance

Fan-concept art under the owner-reported `ART-PUBLICATION-ALL-V1` scope; no public reuse licence; no game or asset-pack distribution right. Every pixel comes from the committed GOB-02-DEATH A sources, the GOB-01 B accessory overlays (frames 0, 16 and 25) or the literal rows in `scripts/author_gob02b_death.py`. Nothing was traced or sampled from concept boards, the Death Lab or any image-model output.

### Rights/Provenance

- Design (overlay rules, pose 0 literal rows, spec), sources, manifests, export and evidence: Claude Opus; review fixes: Claude Sonnet.
- Independent review: pending, separate top-tier agent.
