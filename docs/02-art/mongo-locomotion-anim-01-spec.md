# ANIM-MONGO-01: adult Mongo 76 px locomotion, design spec

**Status:** all 16 frames are authored. The keys (frames 4, 5, 6, 10, 12, 13, 14) are frozen (6, 10 and 14 were revised after the silhouette gate); the in-betweens 7, 8, 9, 11 and 15 are finished (see the [execution record](#execution-record)) and wait for the independent review before they are frozen. Nothing is accepted. This is art only: Mongo's gameplay (`charger`, WORK-017) comes after the slice, and no runtime, physics or Bevy file changes here.

**Scope:** the tags `idle` (the approved 76 px idle, pixels unchanged), `run` (6), `jump_start` (2), `jump_rise` (1), `jump_fall` (1) and `land` (2). The structure mirrors [ANIM-CARL-01](carl-locomotion-anim-01-run-spec.md) and [ANIM-DONUT-01](donut-locomotion-anim-01-run-spec.md): one locomotion grid per character, the idle re-padded unchanged, keys pinned by a recipe check.

**Inputs:** the [character bible](mongo-character-bible.md), the [scale study](mongo-scale-study.md) ("Run readiness"), the approved adult board in [`assets/concept/mongo/approved/`](../../assets/concept/mongo/approved/README.md) (viewed only, the "Correr", "Salto" and "Impacto" action thumbnails as pose intent; no pixel copied or traced) and the master [`mongo_adult_h76.grid.json`](../../assets/source/characters/mongo/mongo_adult_h76.grid.json).

## 1. Source of truth and files

| File | Role |
| --- | --- |
| `assets/source/characters/mongo/mongo_adult_h76_locomotion.grid.json` | editable pixel-grid-v1 source, 16 frames (source of truth once the executor starts) |
| `assets/source/characters/mongo/mongo_adult_h76_locomotion.ase` | `.ase` built from the grid by `scripts/pixel_grid_to_ase.py build` |
| `assets/manifests/mongo_adult_h76_locomotion.json` | export manifest (`prototype_fixture_unapproved`) |
| `assets/source/characters/mongo/mongo_adult_h76_locomotion.pixelart.json` | toolkit config: tick table, contact modes, validators, frozen keys |
| `tests/fixtures/anim-mongo-01_keys.json` | SHA-256 per frozen frame (idle 0-3 and the seven keys) |
| `scripts/author_mongo_locomotion.py` | deterministic authoring rig: `--write`, `--check-keys`, `--preview DIR` |
| `scripts/make_mongo_locomotion_evidence.py` | evidence in [`docs/evidence/anim-mongo-01/`](../evidence/anim-mongo-01/README.md) |
| `tests/test_art_mongo_locomotion.py` | identity, footfall, bob, contact and validator checks |

The master grid is never modified. `--check-keys` rebuilds the keys from `POSES` and compares every layer; it also checks that frames 0-3 are the master idle.

## 2. Frame, pivot, layers and palette

- **Canvas 128 x 88, pivot (76, 87), contact row 87.** The master is 108 x 80 with pivot (68, 79). It is placed at offset (8, 8), so the master pivot lands exactly on the locomotion pivot. The idle-to-run switch therefore moves nothing: feet, face and body stay where the idle put them. The extra 8 rows on top hold the jump rise crest (top row 6); the extra 20 columns hold the forward head (nose up to x 121) and the swept tail (back to x 7).
- **Layers:** the master's eight, same names and order: `tail, far_leg, far_arm, body, near_leg, head, crest, near_arm`.
- **Palette:** the master's 24 colours (`mongo-adult-v0`), identical list and order. No new colour. The far leg uses the master far-leg ramp (`s` lit edge, `z` base, `Z` outline); the far thigh is the near drumstick one step darker (`1,2 -> 3`, `3 -> 4`, `4,5 -> 5`). The lighter magenta outline (`o`) deviation from Carl and Donut stays as recorded in the scale study.
- **Facing:** authored facing right. The runtime mirrors around the pivot column.

## 3. Gait: a digitigrade heavy runner

Mongo runs like a big ground bird, not like a mammal. The design reads in four rules:

1. **Bird leg.** The feathered drumstick (femur) barely moves: it only translates 2-3 px forward or back. The motion is in the long shin (14 px, knee to hock) and the metatarsus (11 px, hock to ball). The hock always bends **backwards**, behind the knee-ball line. Only the toes touch the ground; the metatarsus never lies flat.
2. **Weight.** The body is lowest in the passing frame (weight over the support leg) and highest in the airborne frame: body bob 3 px (dy 1 / 3 / 0 below the idle). The whole run sits 0-3 px lower than the idle and 3 px further forward (lean-in).
3. **Head stabilised.** Like a bird, the head holds still while the body bobs: the neck absorbs the motion. Head bob is 1 px (head top-left y 23 / 23 / 22) against a 3 px body bob. The neck leans 7 px forward at the head (horizontal shear of the neck rows, head moved rigidly), and 1-3 neck rows are dropped so the head sits lower.
4. **Counterbalance.** The tail plume streams back (row shear: k 0.40-0.50, rows above the tail base shift back), with a slight swing per frame (Δk ≤ 0.10). The wing fan trails back (row shear 0.25-0.35, lower rows back). In the crouch and landing the tail lifts (column shear up) to balance the dropped head.

### Footfall (6-frame run, one step per 3 frames)

```
run frame     F0 (4)    F1 (5)    F2 (6)     F3 (7)    F4 (8)    F5 (9)
              KEY       KEY       KEY        rig       rig       rig
near leg      CONTACT   PASSING   push-off   trail     tuck      reach
far leg       trail     tuck      reach      CONTACT   PASSING   push-off
row 87        near      near      none       far       far       none
              84-103    68-87     (air)      84-103    68-87     (air)
body dy       +1        +3        0          +1        +3        0
```

Each foot is planted for two frames and moves back exactly **D = 16 px** between them (84 -> 68). The swing foot path, as ball positions: toe-off (50, 77) -> trail (60, 72) -> tuck (78, 70) -> reach (89, 78) -> contact (90, 83). F3-F5 are F0-F2 with the leg parameters swapped (same body, tail, wing and head).

## 4. Per-frame targets (keys as authored)

Canvas coordinates. "Body" is the master torso translation; "lean / drop" is the head's forward shift and the neck rows removed; "Head" is the head layer's top-left (idle: (85, 19)); "Back" is the top body row at x 60; "Row 87" lists the opaque runs on the contact row. Knee, hock and ball are rig joints (ball = bottom-centre of the metatarsus; the foot stamp hangs from it).

| Frame | Role | Body (dx, dy) | Lean / drop | Head | Back | Bottom | Row 87 | Near knee / hock / ball, foot | Far knee / hock / ball, foot |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 4 | run F0 contact | (3, 1) | 7 / 3 | (95, 23) | 46 | 87 | near 84-103 | (74.5, 66.5) / (79.6, 79.6) / (90, 83) plant | (68.5, 65.5) / (54.9, 62.3) / (60, 72) curl |
| 5 | run F1 passing | (3, 3) | 7 / 1 | (95, 23) | 48 | 87 | near 68-87 | (71.5, 68.5) / (63.4, 79.9) / (74, 83) plant | (72.5, 66.5) / (74.4, 80.4) / (78, 70) curl |
| 6 | run F2 airborne | (3, 0) | 7 / 3 | (95, 22) | 45 | 84 | none | (68.5, 65.5) / (54.6, 67.0) / (50, 77) toe | (74.5, 64.5) / (78.0, 78.1) / (89, 78) reach |
| 10 | J0 crouch | (1, 5) | 2 / 4 | (88, 28) | 50 | 87 | near 61-80, far 75-94 | (70.5, 70.5) / (58.0, 76.7) / (67, 83) plant | (71.5, 69.5) / (70.0, 83.4) / (81, 83) plant |
| 12 | jump_rise | (3, -6) | 9 / 0 | (97, 13) | 39 | 84 | none | (68.5, 59.5) / (58.9, 69.7) / (50, 80) plant | (73.5, 57.5) / (61.5, 64.7) / (72, 68) curl |
| 13 | jump_fall | (3, -2) | 5 / 1 | (93, 18) | 43 | 85 | none | (74.5, 63.5) / (76.6, 77.3) / (87, 81) reach | (73.5, 62.5) / (69.2, 75.8) / (80, 78) reach |
| 14 | L0 impact | (2, 6) | 4 / 6 | (91, 31) | 51 | 87 | near 86-105, far 54-73 | (69.5, 71.5) / (81.8, 78.2) / (92, 83) plant | (69.5, 70.5) / (55.7, 72.9) / (60, 83) plant |

Tail (row shear k, column shear k) and wing shear per key: 4 (0.45, -0.04, 0.30); 5 (0.40, -0.02, 0.25); 6 (0.50, -0.06, 0.35); 10 (0.15, 0.13, 0.10); 12 (0.55, -0.10, 0.45); 13 (0.20, 0.14, -0.05); 14 (0.05, 0.18, -0.15). Negative wing shear lifts the fan forward (fall and impact: the wing flares as he brakes).

Pose intent:

- **4 contact:** near leg reaches forward, almost straight, toes strike 8-27 px ahead of the pivot; far leg folded high behind with the foot curled (toes hooked down and back).
- **5 passing:** weight on the near leg, knee deeply flexed, hock low behind; body at its lowest. The far leg swings through with the knee forward and the curled foot tucked under the wing.
- **6 airborne (toe-off):** the near foot has just left the floor: heel up, metatarsus about 65 degrees from horizontal, toes hooked down and back with the claw leading (`toe` stamp), lowest pixel row 84. The far leg reaches forward with the toes spread, lowest row 82. Body at its highest, tail and wing streamed most. Both feet are clearly in the air, so the planted foot shows in exactly two frames per step (no third sliding frame).
- **10 crouch:** body 5 px below the idle (deeper than the idle-close first pass), feet together under the hips (14 px apart), deep knee flex, head pulled in (lean 2) and low, tail lifted as a counterweight. The wind-up before the jump.
- **12 rise:** body 6 px above the idle, neck stretched forward (lean 9), the near leg trailing extended down-back, the far leg tucked high under the belly. Tail and wing streamed back and down.
- **13 fall:** both legs reach forward and down, toes spread for landing; tail and wing float up.
- **14 impact:** the lowest pose, digitigrade: the near leg is braced straight forward (hock 4.8 px above the ball row, behind the knee-ball line) and only its toes touch the floor (row 87 run 86-105 = the foot stamp); the far leg is folded under the hips (back toes 54-73). Head dropped 12 px below the idle (drop 6), tail whipped up. Reads lower and wider than the crouch (top row 24 against 21; ball points 32 px apart against 14).

## 5. Rig rules (for anyone adding frames)

- Master parts are only translated or edited per whole row or column. No resampling, no rotation, no concept pixels. The head layer is a pixel-exact translation of the master head in every frame (tested).
- Shins and metatarsi are tapered capsules (radius 3.1 -> 2.6 and 2.6 -> 2.1) shaded with the master ramp by the side facing the upper-left light: lit edge `S`, base `s`, shade `z`, outline `Z` (far leg `s`/`z`/`Z`/`Z`).
- Feet: `plant` is the master idle foot (ball pad, toes, claw, raised sickle claw); `curl` (swing), `reach` (before contact) and `toe` (toe-off: toes hooked down-back, sickle claw raised) are hand-typed stamps in the script. All hang from the ball point.
- Digitigrade floor rule: in grounded frames only foot-stamp pixels may touch rows 86-87; the hock stays at least 3 px above the ball row. In airborne frames every foot's lowest row is 84 or higher.
- After assembly a deterministic `tidy` rule recolours seam orphans (a non-accent pixel with no same-key 8-neighbour takes the most common neighbour key, on its own layer). Eye, catchlight, teeth and claw rims (`x Y O y w G`) are never changed. Further hand fixes go in `PATCHES` with the expected old key.

## 6. Timing (60 Hz ticks)

| Tag | Frames | Ticks | ms | Playback | Contact |
| --- | --- | --- | --- | --- | --- |
| idle | 0-3 | 27, 18, 33, 10 | 450, 300, 550, 167 | loop | grounded |
| run | 4-9 | 5 each | 83 each (500 ms cycle) | loop | mixed (6 and 9 airborne) |
| jump_start | 10-11 | 6, 4 | 100, 67 | once | grounded |
| jump_rise | 12 | hold | (100 preview) | hold while rising | airborne |
| jump_fall | 13 | hold | (100 preview) | hold while falling | airborne |
| land | 14-15 | 5, 7 | 83, 117 | once | grounded |

- **Idle frame 3 (ruling):** 10 ticks (167 ms) is the canonical runtime value; the master's 160 ms cannot be represented at 60 Hz. The master grid's pixels and metadata stay untouched in this order.
- **Speed-linked run (for a later integration):** `ticks = clamp(round(60 * 16 / |vx|), 4, 8)`, advanced from simulation ticks, keeping the phase when the speed changes. At 185 px/s (`RUN_SPEED`): 5 ticks, ground 15.4 px per frame against the 16 px design, a 0.6 px backward drift per planted frame. At 145 px/s: 7 ticks, 0.9 px. Below 120 px/s the 8-tick cap applies and the planted foot visibly drifts back (2.7 px per frame at 100 px/s). **Ruling:** no walk cycle in this order. WORK-017 note: until a walk exists, companion-follow must use idle/run hysteresis and never play the run below about 120 px/s; if the charger's speed differs from 185 px/s, re-derive the ticks and stride.
- Animation never feeds back into physics: contact rows, the airborne frames and the hold frames are presentation only. Collision and charge windows belong to WORK-017 data.

## 7. Acceptance targets

1. `python3 scripts/author_mongo_locomotion.py --check-keys` passes; `python3 -m scripts.pixelart validate assets/source/characters/mongo/mongo_adult_h76_locomotion.pixelart.json` passes (V09 frozen keys included).
2. Idle frames 0-3 are the master idle pixel for pixel; the head layer is a rigid master copy in every frame; palette, layer names and order equal the master.
3. Grounded frames (0-5, 7, 8, 10, 11, 14, 15) end on row 87; frames 6, 9, 12 and 13 have no pixel on row 87.
4. Run row 87: frame 4 near 84-103, 5 near 68-87, 7 far 84-103, 8 far 68-87, 6 and 9 empty. No foot is planted in two frames at different x except by exactly 16 px. In frames 6 and 9 the lowest pixel of each leg layer is row 84 or higher (toe-off, tested).
4b. Frame 14: near-leg pixels on rows 86-87 are the foot stamp only (run 86-105) and the near hock sits at y ≤ 80 (tested).
5. Run head bob ≤ 1 px; body bob 3 px; back top within ±1 px of section 4 in the keys and drafts.
6. 0 isolated pixels in every frame; ≤ 2 orphans per frame (keys: 0-2 now).
7. At 1x, beside Carl 52 and Donut 40 ([lineup](../evidence/anim-mongo-01/scale_lineup_x1.png)): the run reads as a forward-leaning bird run (head forward, tail back), the two legs separate, and the crouch and the impact read as different poses.
8. Silhouette: the crest, mask, gold eye, blue wing band and fan tail stay readable in every frame (no frame hides the face or the wing band).

## 8. Known weaknesses (design, for the reviewer)

- **Tail is the idle fan swept back** (accepted by the silhouette gate for this order): on-model and reads as a counterbalance at 1x. Revisit only for a future charge or sprint tag if a game-feel review asks for it.
- **Thigh is rigid.** The drumstick only translates; in the push-off and rise frames a slight back-swing of the feather mass would help.
- **Far legs are small.** The far trailing and tucked legs are mostly behind the near thigh and the wing; they read at 3x, barely at 1x.
- **Crouch versus impact:** both are low, flexed poses; they differ by height (top row 21 against 24), stance (14 against 32 px between ball points), the braced front leg and the head height.
- **Crouch far hock:** the far (back) hock of the crouch sits low behind the near foot; it is mostly covered and was not flagged at 1x.
- No Donut-mounted check, no Bevy check, no human game-feel review. The lit Floor 1 scene is a Pillow composition with the A1 model, not a Bevy capture.
- **Frame 14 hock** (gate note, left as is): the braced near leg is almost straight; a 1-2 px backward hock kink would show the digitigrade joint better. Optional polish for the designer; it would change a frozen key.

## 9. Executor checklist

Execution work only; any design question goes back to the designer. Do not change the keys (4, 5, 6, 10, 12, 13, 14), the idle, the head pixels, the palette, the canvas, the pivot or the layer order.

1. **Frames 7, 8, 9 (run F3-F5):** the rig drafts are F0-F2 with the leg parameters swapped (`DRAFTS` in the script). Finish them by hand so the far leg is the planted leg:
   - row 87: frame 7 far 84-103, frame 8 far 68-87, frame 9 empty (every leg layer's lowest row 84 or higher; the far leg toes off with the `toe` stamp, as in key 6);
   - the planted far leg must read in front of the tail and behind the near thigh; where the near drumstick covers the far knee, keep at least 3 px of the far shin visible below the drumstick;
   - the near leg (trailing, tucked, reaching) keeps the near ramp and must not overlap the planted far foot;
   - tail, wing, head and crest identical to frames 4, 5, 6; the legs are swapped, and body and near arm are identical except 1-3 px of seam clean-up (tidy recolours) where the swapped legs meet them.
2. **Frame 11 (J1 push-off):** between crouch 10 and rise 12. Targets: body (2, 0) ±1, head top-left (93, 21) ±1, both feet on their toes behind the hips (row 87 within x 54-81), heels lifted, tail tip halfway between 10 and 12, wing shear about 0.30. The current draft is close; clean the legs by hand.
3. **Frame 15 (L1 recover):** between impact 14 and the idle. Targets: body (2, 3) ±1, head top-left (90, 24) ±1, feet planted (row 87 near 74-93, far 60-79 ±2; only foot-stamp pixels on rows 86-87, hocks at least 3 px above the ball row), tail settling (column shear ≤ 0.08). It must blend into idle frame 0 without a pop: head within 6 px of (85, 19).
4. Record every hand change as a `DRAFTS` recipe or a `PATCHES` entry (with the expected old key). Keep `--check-keys` passing; do not route the executor frames through it.
5. **Validators and tests:** keep `max_orphans_per_frame` 2 and 0 isolated pixels in all 16 frames. After review, 7, 8, 9, 11, 15 were added to `rules.frozen_keys` and re-frozen with `python3 -m scripts.pixelart freeze --force ...`.
6. **Rebuild and evidence:** `python3 scripts/author_mongo_locomotion.py --write` only if a recipe changed (it overwrites hand edits not recorded in the script), `python3 scripts/pixel_grid_to_ase.py build` for the `.ase`, then `python3 scripts/make_mongo_locomotion_evidence.py`. With the Floor 1 kit atlases exported (`bash scripts/export_assets.sh`), also run `python3 -m scripts.pixelart evidence` for the lit scene, and add a footfall diagram and a run onion skin (frames 4-9, back line and feet) like ANIM-DONUT-01.
7. **Docs:** add an ANIM-MONGO-01 line to `docs/02-art/animation.md`, update "Run readiness" in [mongo-scale-study.md](mongo-scale-study.md) to point here, fill the evidence README file table. Leave `docs/04-production/backlog.md` to the orchestrator.

## 10. Rulings and open questions

Rulings from the silhouette gate (independent top-tier check of the keys):

1. **Run tail:** the swept idle fan is accepted for run, jump and land.
2. **Idle frame 3:** 10 ticks (167 ms) is canonical; the master stays untouched.
3. **Walk:** not in this order; the foot drift below about 120 px/s is a known limitation logged for WORK-017 (idle/run hysteresis until a walk exists).

Still open: Mongo's actual run speed once `charger` gameplay exists (re-derive ticks and stride if it is not 185 px/s).

Living horizon: this order adds no playable move, ability or co-op behaviour; it fits [character-progression-horizon.md](../01-design/character-progression-horizon.md) as art groundwork for the later `charger` companion.

## Execution record

Executor pass (medium tier) over the checklist in section 9. The keys, the idle, the head, the palette, the canvas, the pivot and the layer order are untouched: `python3 scripts/author_mongo_locomotion.py --check-keys` passes and the fixture hashes of the keys and the idle still match.

- **Frames 7, 8, 9 (run F3-F5):** checked against the checklist and kept as the rig output (`DRAFTS` swap of 4, 5, 6); no `PATCHES` needed. Row 87: frame 7 far 84-103, frame 8 far 68-87, frame 9 empty; frame 9 lowest row 84 for both legs; the far shin is visible on at least 3 rows below the drumstick in 7 and 8; the near leg does not touch the planted far foot; tail, wing, head and crest equal frames 4, 5, 6 (tested); body and near arm differ from 4, 5, 6 only by 1-3 px of seam clean-up at the legs.
- **Frame 11 (J1 push-off):** both feet changed from the flat `plant` stamp to the `toe` stamp, so the heels are lifted and only the claws touch row 87 (near 56-57, far 68-69, inside x 54-81); balls (58, 80) and (70, 80). Body (2, 0), head top-left (93, 21), tail tip halfway between 10 and 12 (x 9 against 11 and 7), wing shear 0.30.
- **Frame 15 (L1 recover):** the near drumstick moved from thigh (1, 0) to (1, -3) so the near hock rises from y 82.3 to 79.4 (rule: at most 80, three px above the ball row); the far hock is at 74.9. Row 87 near 74-93, far 60-79, foot stamp pixels only on rows 86-87, body (2, 3), head (90, 24), tail column shear 0.06.
- **Recipes:** the only script change is the two `DRAFTS` entries (11 and 15), renamed without "draft". Orphans: at most 2 per frame in all 16 frames (now tested for every frame), 0 isolated pixels.
- **Timing:** full tick windows are in the toolkit config and the manifest (`locomotion.tick_windows`): idle 27, 18, 33, 10 (frame 3 = 10 ticks canonical), run 5 each, jump_start 6, 4, land 5, 7, jump_rise and jump_fall hold. At 185 px/s the planted foot slides 0.6 px per frame (tested, together with the exact 16 px stride and the loop seam 8 -> 9 -> 4).
- **Export:** `bash scripts/export_assets.sh` writes the atlas and metadata under `generated/art/mongo_adult_h76_locomotion.*` (not committed, like the Carl and Donut atlases); `tests/test_art_assets.py` passes once the export has run. A test compares every atlas pixel with the grid (24 colours, 16 frames of 128 x 88).
- **Scene config:** the toolkit scene shows Mongo unmirrored (a companion faces right like Carl) at idle, run F0, rise (lifted 20 px) and impact.
- **Not done:** no Bevy integration; no human game-feel review; the backlog is left to the orchestrator.

### Review follow-up

The independent top-tier review passed with execution items only and no design defect. Applied:

- Frames 7, 8, 9, 11 and 15 are in `rules.frozen_keys` and re-frozen (`python3 -m scripts.pixelart freeze --force`); `python3 scripts/author_mongo_locomotion.py --check-keys` still passes.
- Evidence: the run onion-skin legend now sits below the title and notes that frames 6 and 9 share one toe ring. The review-pack strip is named `03_jump_land_strip.png` (config `review.strip_name`) and now includes the jump_rise and jump_fall holds (12, 13).
- Ruling: the frame 14 hock kink is not needed; the straight braced near leg reads as bracing and the drop, stance and tail whip carry the impact. Revisit only if a game-feel review asks. The optional frame 11 shin polish is skipped.
- **WORK-017 note (later gameplay/art item):** there is no run-to-idle stop frame, so stopping snaps the head about 10 px back and 4 px up. Play land frame 15 (or a future stop frame) as a brake, or accept the snap under the idle/run hysteresis rule. The 167 ms jump_start wind-up may lag a player-synchronised jump; that is a gameplay timing question.

Evidence: [`docs/evidence/anim-mongo-01/`](../evidence/anim-mongo-01/README.md) (contact sheets, keys at 6x, 1x and 3x lineup with Carl 52 and Donut 40, footfall diagram, run onion skin, run and jump/land GIFs at x1 and x3 with an exact palette, lit Floor 1 scene at 640 x 360 and 1920 x 1080, review pack).

## Rights/Provenance

Fan-concept art under the owner-reported `ART-PUBLICATION-ALL-V1` scope (not verified consent, no reuse licence, no distribution rights), see [rights-and-release.md](../04-production/rights-and-release.md). All pixels come from the owner-approved 76 px master or from rules and hand-typed stamps in `scripts/author_mongo_locomotion.py`; no image-model raster, no resampling, no concept-board pixels. Design and keys: Claude Opus. Execution: Claude Sonnet (in-betweens 7, 8, 9, 11, 15, export, tests, evidence). Independent review: Claude Opus, passed with execution items (applied above).
