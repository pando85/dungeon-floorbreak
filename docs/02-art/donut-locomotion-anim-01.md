# ANIM-DONUT-01: Princess Donut locomotion candidate (40 px)

**Status:** draft, awaiting owner art review. **Round 3 complete:** after the round-2 independent Opus review, Opus redrew keys 6, 8, 12 and wrote the round-3 spec (keys 4 and 10 unchanged), and Sonnet redid in-betweens 5, 7, 9, 11 and the frame-13 tail per the [run spec](donut-locomotion-anim-01-run-spec.md). Nothing here is accepted. The approved master [`donut_h40.grid.json`](../../assets/source/characters/donut/donut_h40.grid.json) and its manifest are unchanged; the candidate lives in its own source and manifest until a human approves it. Runtime integration (state machine, Bevy atlas swap) is a separate work order.

Round 1 authoring and the round-1 self-review were done by Claude Opus. Rounds 2 and 3: **design Opus** (keys, run spec, round-3 redraw of keys 6, 8, 12), **execution Sonnet** (in-betweens, polish, tests, evidence), **review pending independent Opus** (AGENTS.md "Model routing for art work"). Owner diagrams: [`run_footfall_diagram.png`](../evidence/anim-donut-01/run_footfall_diagram.png) and the back-line/rump onion [`run_onion_backline_rump_16x.png`](../evidence/anim-donut-01/run_onion_backline_rump_16x.png).

## Files

| File | Role |
| --- | --- |
| [`assets/source/characters/donut/donut_h40_locomotion.grid.json`](../../assets/source/characters/donut/donut_h40_locomotion.grid.json) | editable source (pixel-grid-v1), **source of truth** |
| `assets/source/characters/donut/donut_h40_locomotion.ase` | Aseprite/LibreSprite file rebuilt from the grid by the export (layers, tags, frame timings) |
| [`assets/manifests/donut_h40_locomotion.json`](../../assets/manifests/donut_h40_locomotion.json) | candidate manifest `donut_h40_locomotion_candidate_v0`, `prototype_fixture_unapproved` |
| [`scripts/author_donut_locomotion.py`](../../scripts/author_donut_locomotion.py) | authoring record and parts library; round-2 keys recipe (`--check-keys`) and in-between recipes (`--check-inbetweens`, `--apply-inbetweens` writes frames 5, 7, 9, 11 and the frame-13 tail only); never writes the keys |
| [`scripts/make_donut_locomotion_review.py`](../../scripts/make_donut_locomotion_review.py) | contact sheets, GIFs, Floor 1 lit compositions with Carl, measurements |
| [`scripts/make_donut_run_diagram.py`](../../scripts/make_donut_run_diagram.py), [`scripts/make_donut_run_onion.py`](../../scripts/make_donut_run_onion.py) | footfall diagram of run frames 4-9; back-line and rump onion image |
| [`tests/test_art_donut_locomotion.py`](../../tests/test_art_donut_locomotion.py) | idle preserved, face/head are master pixels, palette, clean clusters, footfalls, back-top profiles, head bob, tail sway, durations |
| [`docs/evidence/anim-donut-01/`](../evidence/anim-donut-01/README.md) | committed review renders with SHA-256 |

## Frames, tags and timings

Canvas **72×44**, pivot **(46, 43)**, one atlas row of 16 frames. Layers bottom to top: `tail, legs_far, body, legs_near, head, face, gear` (the master's five plus two leg layers, empty in the idle frames). Palette: the master's 22 keys, unchanged.

| Frame | Tag | ms | Pose | Lowest row |
| --- | --- | --- | --- | --- |
| 0–3 | `idle` | 320/260/180/260 | approved master idle, pixel-identical, re-padded (+24 px) | 43 |
| 4 | `run` | 80 | F0 key: near fore contact (x 51-56), spine neutral | 43 |
| 5 | `run` | 80 | F1 in-between: far fore contact (x 53-58), body lowering, near fore toe-off; rump x 23 | 43 |
| 6 | `run` | 80 | F2 key: gathered, spine arched 3 px, near hind contact (x 36-41); head 1 px back | 43 |
| 7 | `run` | 80 | F3 in-between: far hind contact (x 32-37), spine unarching, rump x 21, fores swing forward | 43 |
| 8 | `run` | 90 | F4 key: shorter extended suspension (rump x 19), flat back y 20, no paw on row 43 | 39 (airborne) |
| 9 | `run` | 80 | F5 in-between: descending, plain torso at rump x 20, fores reach down, no paw on row 43 | 42 (airborne) |
| 10 | `jump_start` | 90 | key: crouch, chest low, rear up, hinds bunched, tail lifts | 43 |
| 11 | `jump_start` | 70 | in-between: push-off, hind toes on row 43 (x 18-26), front rising, short near/far forelegs reaching past the chest | 43 |
| 12 | `jump_rise` | 120 | key: rising, long tilted body, fores forward, hinds trailing, tail streaming | 43 |
| 13 | `jump_fall` | 120 | falling: front slightly down, fores reach for the floor; tail moved down 5 px in round 3 to follow frame 12 | 43 |
| 14 | `land` | 80 | impact: squashed, eyes squeezed shut (master blink curve), tail flicks | 43 |
| 15 | `land` | 110 | recovery: chin up first while the body is still low | 43 |

Run cycle at 185 px/s: 490 ms (80, 80, 80, 80, 90, 80). Opaque heights: idle 40–41, run 33–39, jump 37–38, land 35–37; widths up to 62 px (full table in `measurements.json`).

## Notes for the integration agent

- **Frame size changes from 46×44 to 72×44.** A cat stretches horizontally; the extended run and rise frames are up to 62 px wide. All 16 frames, including idle, share one canvas, so `actor_sprite::DONUT` needs `width: 72, pivot_x: 46` (height, `pivot_y: 43` and the 40 px idle opaque height stay) when this atlas replaces `donut_scale_h40.png`. The approved 46×44 atlas is untouched until then.
- **Pivot convention kept:** the pivot is on the contact row (row 43) and every grounded frame's lowest opaque row is row 43 (0 px sink). Horizontally the pivot sits 6 px behind the nose in every frame, exactly as in the idle (pivot 22, nose 28), so the face stays within 1 px horizontally when switching idle ↔ run ↔ jump. The pivot is therefore under the chest in profile poses, not under the body centre; flipping mirrors around the face.
- **Airborne frames:** run F4/F5 (frames 8, 9) are a gallop suspension; their lowest row is 4/1 px above the pivot row on purpose. Keep the physics foot on the walking line; do not snap these frames down. `jump_rise` and `jump_fall` have their lowest pixel (trailing hind paws / reaching fore paws) on row 43, so the sprite sits on the physics foot position through the arc and `land` frame 14 meets the floor without a pop.
- **Playback rule (speed-linked, from the run spec section 4; a suggestion for the integration agent, not a decision):** Donut trails Carl at his ground speed `vx` (up to `RUN_SPEED` 185 px/s). The run is designed for a constant **D = 14.8 px of ground per frame**:

  ```
  frame_ms(i) = clamp( round( w[i] * 1000 * D / |vx| ), 50, 130 )     D = 14.8 px
  w = [1, 1, 1, 1, 1.125, 1]     (frame 8, the suspension, is held a little longer)
  ```

  | \|vx\| | Base frame | Frame 8 | Cycle | Stride per cycle |
  | --- | --- | --- | --- | --- |
  | 185 px/s | 80 ms | 90 ms | 490 ms | 90.7 px |
  | 145 px/s | 102 ms | 115 ms | 625 ms | 90.6 px |
  | < 114 px/s | 130 ms (clamped) | 130 ms | 780 ms | ground travel per frame < 14.8 px; a walk cycle is out of scope |

  The atlas stores the 185 px/s values.

  **Slip, corrected in round 3** (the round-2 claim "slip 0 at all speeds" was wrong). Single-frame contact only removes the *frame-to-frame* slide of a paw that stays on row 43. What remains:
  
  | Effect | 145 px/s | 185 px/s | < 114 px/s (cap 130 ms) |
  | --- | --- | --- | --- |
  | Ground covered while a paw shows its one contact frame (`vx * frame_ms`) | 14.8 px | 14.8 px | `vx * 0.13` < 14.8 px |
  | Near fore: lifted paw in the next frame, ahead of the ground point it touched (frame 4 to 5) | +3.8 px | +3.8 px | grows as speed drops |
  | Far fore: same, frame 5 to 6 | +6.8 px | +6.8 px | grows as speed drops |
  | Paws below the cap speed | n/a | n/a | the sprite's designed backward paw travel (about 11-15 px per frame) exceeds ground travel, so paws visibly drift backwards |
  
  Round 1 for comparison (frame-to-frame slide of paws on row 43 in two frames): near fore about 8 / 11 px, far fore about 5 / 8 px, near hind about 1 / 4 px at 145 / 185 px/s. The speed rule is only valid from about 114 to 185 px/s; below that the integration must switch to a walk cycle (not authored) or accept the backward drift.

  `jump_start` plays once on take-off (160 ms total, may be shortened to frame 11 only if input latency matters), `jump_rise` holds while rising, `jump_fall` holds while falling, `land` plays once on touchdown, then `idle` or `run`.
- **Trail:** Donut trails Carl by 32 px and draws behind him; her profile head reaches 22 px ahead of her pivot, so it overlaps Carl's back leg by a few pixels (see `floor01_trail_run_*`). This is the owner-accepted pass-through behaviour.
- No events or hitboxes are authored; `footstep` markers could be F0, F1, F2, F3 if audio needs them.

## How it was made (provenance)

- **Copied, not redrawn:** head, face, tiara, chest ruff, collar, butterfly charm and the tail plume are the master's pixels, translated only. The tests prove the face and head layers of every locomotion frame are the master's pixel sets (the landing impact frame uses the master's own blink face). Coat map, eyes and accessories are therefore identical in every frame.
- **New, hand-placed:** a side-view torso (24×19) with the master's near-side coat map (ginger saddle behind the shoulder, ginger haunch blotch with a dark brindle bite, H rim light, tufted dark skirt), a rounded tufted bottom for the chest ruff, seven 1 px shadow-side tufts on the tail edge that the seated idle hides, and 14 leg poses (cream fore socks; hind legs continue the ginger haunch blotch; far legs in shadow values).
- **Pose data:** whole-pixel offsets plus pixel-safe torso edits (repeat/drop one column for extension/compression, lift a column run for the arched back, per-column offsets for the tilted leap, per-row tail shear for sway). No rotation, scaling or resampling.
- **Cleanup:** 23 per-pixel fixes after the lint orphan report (mostly where the head outline or a tilt broke a two-pixel cluster). Final lint: 0 orphan and 0 isolated pixels in all 16 frames.
- Concept board use: the turnaround and the board's tiny "gameplay sprite concept" run/jump mocks were viewed as enlarged crops for silhouette only (profile body, 3/4 head, plume up). No pixel was copied from the board.

## Gait design

- **Feline, not human or generic quadruped:** a short-legged half-bound (transverse) gallop with contacts near fore (frame 4) → far fore (5) → near hind (6) → far hind (7), then a two-frame suspension (8, 9). **Measured, round 2:** each paw is on row 43 in exactly one frame (x 51-56, 53-58, 36-41, 32-37), no paw is on row 43 in two consecutive frames, and frames 8 and 9 have nothing on row 43 (lowest paw row 39 and 42). A paw therefore has no frame-to-frame slide on row 43. This replaces the round-1 claim "no skating" (forepaws slid about 8-11 px at trail speed); the round-2 claim "slip 0 at all speeds" was also wrong, see the corrected slip table in the playback rule.
- **Spine flex (measured, back-top y of the `body` layer at x = 32):** neutral 21 (frame 4), lowering 20 (5), arched 18 (6, 3 px above neutral, rump curling under, rump left edge x 24), unarching 19 (7), extended flat 20 (8, rump x 19), level 21 (9). Along the back no step is larger than 2 px past the rump curve, and the back meets the head without a 1 px notch in frames 5, 7 and 9 (tested).
- **Body length (round 3, the "inchworm" fix):**

```
frame          4     5     6     7     8     9    (4)
               KEY   in    KEY   in    KEY   in
face x         43    43    42    43    44    44   43
rump x         22    23    24    21    19    20   22     max step 3 px (round 2: 8)
body length    21    20    18    22    25    24   21     range 18-25 (round 2: 16-31)
```
Body length = face x minus the rump's left edge. Rump and head move together, so the stretch reads as the spine flexing between shoulders and hips, not as a rump sliding under a fixed head. Measured on the grid; image: `run_onion_backline_rump_16x.png`.
- **Weight:** the head bobs 1 px only (face top-left y 20, 21, 21, 20, 20, 20); the head lunges forward 1 px in the extension (frame 8). The tail follows the body; its tip lean changes by at most 2 px between neighbouring run frames (round 1 swung up to 7 px), measured on the grid.
- **Personality:** the proud, level head and raised plume over tiny scurrying cream paws is the joke; the landing squeezes her eyes shut ("oof") and the recovery lifts the chin before the body (dignity restored).

## Round 2 execution notes (Sonnet)

- Drew frames 5, 7, 9, 11 with `compose()` from the round-2 parts and the spec recipes (`INBETWEENS` in `scripts/author_donut_locomotion.py`), plus 8 single-pixel cleanups (`PATCHES3`). Keys 4, 6, 8, 10, 12, frames 0-3, 13-15, the head/face/gear pixels, palette, canvas, pivot and layers are unchanged (`--check-keys` passes).
- Deviations from the spec's starting recipes, to hit its own back-top targets within 1 px: frame 7 arch over columns 7-18 instead of 6-15; frame 9 got two extra plain back columns at column 17 on top of `stretch(..., 10, 2)`, torso at x 14 instead of 15; frame 11 torso lowered 1 px so the belly meets the hind legs.
- Optional items skipped: polish 6 (rotary gallop), frame 15 back 1 px lower.
- Durations of frames 5, 7, 9 set to 80 ms.

## Round 3 execution notes (Sonnet)

Opus redrew keys 6, 8, 12 (head 1 px back in frame 6, shorter extension in frame 8, short near/far jump forelegs `FN_REACH_NEAR`/`FN_REACH_FAR` in frame 12) and wrote the round-3 task list in the run spec. Sonnet executed it; provenance: round-3 keys and spec by Opus, execution by Sonnet, review pending independent Opus.

- Frame 7: rump at x 21, near-hind toe-off moved with it, tail (15, 6, 0.15). Frame 9: plain 24-column torso, rump x 20, head 1 px forward, hinds re-attached, tail (12, 6, 0.20). Frame 5: kept; one back-top pixel added at x 37 to remove the 1 px notch at the head. Frames 7 and 9 got the same one- or two-pixel patches (`PATCHES3`).
- Frame 11: `FN_REACH_NEAR` at (49, 42) and `FN_REACH_FAR` at (56, 39), tail (16, 10, 0.10). Frame 13: tail only, `TAIL_ONLY` (13, 9, 0.10); body and legs untouched.
- Tail shear k for run frames 4-9: 0.15, 0.10, 0.05, 0.15, 0.25, 0.20 (step at most 0.10 including 9 to 4); tip lean (top row minus bottom row) 5, 6, 7, 5, 3, 4 px.
- Deviation, reported to Opus: the spec asks for a jump-tail tip that moves at most 4 px between frames 10, 11, 12 and 13. Keys 10 and 12 are fixed and their measured tip pixels are 15 px apart (x 31 and 16), so one in-between cannot keep both steps within 4 px (they are 8 and 7). Frame 11 follows the spec's starting values; frame 13 steps 4 px from frame 12.
- Also not in the spec: frame 13 now steps 7 px to frame 14's tail (it was 4); frame 14 is a round-1 frame and was left alone.

## Key-pose review log (before the full sequence)

Contact, gathered and extended were built and inspected first at 6–12× and at 2–3× (`key_pose_review_round1_vs_final_5x.png`). Problems found and fixed before the in-betweens:

1. Round 1 torso was 38 px long and read as a dachshund; shortened to 24 px and rounded.
2. The haunch blotch with a dark centre read as a ring ("donut"); the brindle bite now enters from the edge.
3. A 2 px cream shoulder fleck read as a second eye; it is now cream-shadow and smaller.
4. Stick legs on a long body; legs became short stubs under the skirt, hind legs ginger so they read against the dark body.
5. The chest ruff was a square cream slab; it now has a rounded, tufted bottom and the master's chin line.
6. The trailing hind leg read as a detached "carrot"; it now attaches under the haunch at a diagonal.
7. The exposed tail base read as a flag on a pole; the plume sits 3 px further into the rump and its shadow edge got tufts.
8. Reaching forelegs drawn over the cream chest became dirty outline dots; airborne forelegs now reach out below the chest.

## Artistic self-review (round 1, Opus; not an acceptance)

What works: identity is locked (same face, eyes, tiara, collar, charm, coat map in every frame); the silhouette at 1× reads as a fluffy Persian with a plume, close to the board's run mock; the run has a readable gather/extend rhythm; landing has a clear personality beat; the idle is untouched and the face stays within 1 px horizontally across state changes.

Known weaknesses:

- **Static head.** The 3/4 head is the master's exact pixels in every frame, so it never turns, tilts or changes expression while running. This is the price of guaranteed face consistency; a later pass could add ear or eye acting variants.
- **3/4 head on a profile body.** Readable and matches the concept mocks, but anatomically it is a stylisation: her face looks at the camera, not where she runs.
- **Small legs.** Legs are 3–6 px; the gait reads mostly through paw positions and body bob. At 1× (640×360) the alternating contacts are hard to see individually; at 1080p (3×) they read.
- **Dark body on dark floors.** The tortoiseshell torso is low-contrast in cold shadow; the tail, face and ginger patches carry the silhouette.
- **Pixel-edit artefacts (round 2).** The keys use a flat extended torso and an even stair tilt, and the in-betweens use arches of 1 px; the back line has steps of at most 2 px past the rump. In frame 9 two plain back columns were repeated at column 17 (not in the saddle) so the flat back reaches the shoulders without stretching the saddle. Small stair steps remain by design.
- **Jump forelegs** in frames 11 and 12 now reach past the chest (`FN_LEAP`); frame 13 (fall) forelegs were not reworked (optional item skipped).
- **No in-engine check.** Floor 1 compositions use the Pillow A1 model; motion was judged frame by frame and in GIFs, not in Bevy, and no human has judged game feel.

## Regenerate

```bash
python3 scripts/author_donut_locomotion.py --check-keys         # round-2 key frames unchanged
python3 scripts/author_donut_locomotion.py --check-inbetweens   # frames 5, 7, 9, 11 and the frame-13 tail match their recipes
ART_CLI=/path/to/aseprite-or-libresprite bash scripts/export_assets.sh
python3 scripts/make_donut_locomotion_review.py --out docs/evidence/anim-donut-01
python3 scripts/make_donut_run_diagram.py --out docs/evidence/anim-donut-01/run_footfall_diagram.png
python3 scripts/make_donut_run_onion.py --out docs/evidence/anim-donut-01/run_onion_backline_rump_16x.png
python3 -m unittest discover -s tests -p 'test_art_donut*.py'
```

The committed grid is the source of truth; the authoring script never rewrites the keys (round 2 onwards).
