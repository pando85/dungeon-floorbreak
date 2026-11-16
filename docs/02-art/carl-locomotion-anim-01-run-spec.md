# ANIM-CARL-01 run, round 2: design spec for the execution agent

**Status:** design by Claude Opus (round 2, after the independent Opus review of PR #34). Execution by Claude Sonnet, followed by a separate Opus review. Draft, awaiting owner art review. Execution done by Claude Sonnet (see section 9); review pending an independent Opus pass. This spec supersedes the run section ("Run speed and foot slip") of [carl-locomotion-anim-01.md](carl-locomotion-anim-01.md). Jump, land and idle are unchanged and passed review.

Source: `assets/source/characters/carl/carl_h52_locomotion.grid.json` (frames 4-9 = `run`). Canvas 48 x 56, pivot (25, 55), contact row 55. All coordinates below are canvas pixels, x right, y down.

## 1. What was wrong and what changed

| Review finding | Round-2 design |
| --- | --- |
| BLOCKING: short, lopsided stride (near heel lands at x24, behind the pivot; far heel at x30; 21 / 9 px steps, which read as a limp; 5 px ground travel per frame) | Both hip joints sit at one x (27) instead of 6 px apart, so both legs share one geometry and the two steps are identical. Contacts land with the heel at x30 and the ankle and Croc centre at x34 (+9 ahead of the pivot). Push-off ends with the toe at x18-21 (back pixel 7 px behind the pivot). The planted foot moves **8 px per frame**, and each step is **24 px** on both sides |
| Far arm: a straight, level, mechanical reach that vanished on 6-8 | Bent-elbow pump on both arms, with the elbow at about 80-90 degrees throughout. The far fist is visible in every frame |
| No passing pose; the swing foot jumped 14 px | The swing foot moves forward in even steps: push 18, trail 17, tuck 20, pass 29, contact 34 |

Why the hips moved: Carl's leg (hip to ankle) is 14.5 px and almost straight at idle, so the ankle can only reach about 7 px in front of the hip. A heel strike in front of the pivot needs the hips forward of the pivot. The whole run body therefore sits 2 px ahead of the idle position (body offset +2, plus the 1 px torso lean). This reads as a lean-in when he starts running. A strike with the **heel pixel** 7-9 px ahead would need the body 4-5 px ahead of idle, which looks like a pop at the idle-to-run transition, so I stopped at +2. The contact is +9 measured at the Croc centre and +5 measured at the heel pixel.

## 2. Frame plan: keys vs. in-betweens

Structure: contact, down, passing, for each leg. "Passing" is one frame in which the stance leg toes off behind while the swing leg passes forward with the knee high.

| Frame | Role | Status |
| --- | --- | --- |
| 4 | contact, near foot leading | **KEY: authored by Opus, finished. Do not change the legs, feet or arms** |
| 5 | down, near foot planted | in-between: **Sonnet redraws** (the current pixels are a placeholder block-in) |
| 6 | passing: near toe-off, far knee forward | **KEY: finished** |
| 7 | contact, far foot leading | **KEY: finished** (except polish item 2, see section 5) |
| 8 | down, far foot planted | in-between: **Sonnet redraws** |
| 9 | passing: far toe-off, near knee forward | **KEY: finished** |

"Finished" means pose, stride, contact pixels and arm pump are final. Sonnet may still do the optional polish in section 5 (hearts, jacket hem) on the keys, and must keep every number in the tables.

## 3. Per-frame numbers

Body offset is relative to the master's pixels: head, hair, torso, jacket, boxer seat and pin are the master frame-0 cels translated by `body` (dx, dy). The torso, jacket, pin and head get +1 more in x (`lean`). The head gets no further offset (head x = master + 3 in every run frame). Hip = hip joint used for both legs.

| F | body (dx, dy) | head total (dx, dy) | hip | row-55 ground pixels | bottom of opaque | top row |
| --- | --- | --- | --- | --- | --- | --- |
| 4 | (+2, +2) | (+3, +2) | (27, 38) | 30-34 (near heel) | 55 | 6 |
| 5 | (+2, +3) | (+3, +3) | (27, 40) | 22-29 (near, flat) | 55 | 7 |
| 6 | (+2, +2) | (+3, +2) | (27, 38) | 18-21 (near toe) | 55 | 6 |
| 7 | (+2, +2) | (+3, +2) | (27, 38) | 30-34 (far heel) | 55 | 6 |
| 8 | (+2, +3) | (+3, +3) | (27, 40) | 22-29 (far, flat) | 55 | 7 |
| 9 | (+2, +2) | (+3, +2) | (27, 38) | 18-21 (far toe) | 55 | 6 |

Bob: 1 px (down frames are 1 px lower). The whole run is 2-3 px lower than idle: a hunched scramble, not an upright sprint.

### Legs (ankle = bottom of the shin capsule; Croc stamp anchored there)

| F | near ankle | near knee | near Croc | far ankle | far knee | far Croc |
| --- | --- | --- | --- | --- | --- | --- |
| 4 | (34, 50) | (33, 44) | heel strike, toe up | (17, 46) | (24, 45) | trailing, toe down |
| 5 | (26, 50) | (31, 46) | flat | **(20, 42)** | (25, 47) | trailing, tucked high |
| 6 | (18, 50) push | (23, 45) | toe push, heel up | (29, 46) | (34, 41) | hanging under the knee |
| 7 | (17, 46) | (24, 45) | trailing | (34, 50) | (33, 44) | heel strike |
| 8 | **(20, 42)** | (25, 47) | trailing, tucked high | (26, 50) | (31, 46) | flat |
| 9 | (29, 46) | (34, 41) | hanging | (18, 50) push | (23, 45) | toe push |

Knees always bend forward (to the right). For the in-betweens (5, 8): the tucked swing Croc must sit **entirely above row 48** and to the left of the planted shin (planted shin columns about 25-30). It must not stack on the planted Croc, which was the round-1 "one pink blob" defect. The current placeholder overlaps the boxer hem; fix it by dropping the tucked ankle to (19-20, 43) with the heel kicked up and the toe pointing down and back.

### Arms (bent-elbow pump; shoulder, elbow, wrist, fist centre)

| F | near arm: shoulder, elbow, fist (angles) | far arm: shoulder, elbow, fist (angles) |
| --- | --- | --- |
| 4 | (22,24), (18,28), (23,33): back, fist at front of hip (-45, 45) | (32,22), (36,28), (42,24): forward, fist at chest height (35, 125) |
| 5 | (22,24), (20,30), (27,34) (-25, 65) | (32,24), (34,30), (41,28) (20, 105) |
| 6 | (22,24), (24,30), (32,28): coming forward (15, 100) | (32,22), (30,29), (36,31): going back (-20, 70) |
| 7 | (22,24), (29,26), (35,22): forward, fist outside the chest (70, 120) | (32,22), (27,27), (32,32): back (-45, 45) |
| 8 | (22,24), (25,31), **(34,28)** (target, see item 2) | (32,24), (29,29), (36,32) (-25, 65) |
| 9 | (22,24), (20,30), (27,32) (-20, 70) | (32,22), (34,29), (41,28) (15, 100) |

Angles are degrees from straight down, positive = forward: (upper arm, forearm). Arms swing opposite to the legs. The forearm stays 80-95 degrees ahead of the upper arm, so the elbow never straightens. No fist may reach column 45 or beyond (canvas edge 47).

## 4. Foot diagram (row 55 of the current grid, x 14-37; `#` = Croc on the floor, `|` = pivot x25)

```
x   111111222222222233333333
    456789012345678901234567
F4  ...........|....#####...   near heel strike (heel pixel 30)
F5  ........########........   near flat (22-29), 8 px further back
F6  ....####...|............   near toe-off (18-21), 8 px further back
F7  ...........|....#####...   far heel strike
F8  ........########........   far flat
F9  ....####...|............   far toe-off

swing foot (ankle x, row): 18@50 -> 17@46 -> 20@42 -> 29@46 -> 34@50 (contact)
                      F6/F9  F7/F4    F8/F5    F9/F6     F4/F7
```

Ground travel per frame: heel 30 to 22 = 8 px, toe 29 to 21 = 8 px. Step length: 3 frames x 8 px = **24 px, the same for both feet**.

## 5. Polish items

1. **Far arm pump (blocking-adjacent): done in the keys.** Sonnet keeps it in the in-betweens using the table.
2. **Near arm forward swing hides in the torso (frames 7-8).** In frame 7 the near fist centre is (35, 22), 3 px outside the torso front outline (about x 32 at rows 20-24): done. **Frame 8 (Sonnet):** the fist centre goes to (34, 28) so at least 2 fist pixels sit outside the jacket front edge. Where the forearm crosses the jacket, its upper edge uses `T` (sheen) instead of `K` for the 3-4 pixels next to the fist, so the forearm separates from the jacket body. Do not enlarge the sleeve.
3. **Head jump between rise (12) and fall (13): done.** The fall frame now uses torso lean 1 and head +1, the same as rise, so the head x is identical in 12 and 13 (only y changes, by the body offset). Sonnet: verify, and do not undo it.
4. **Passing pose / swing jump: done** (sections 3 and 4).
5. **Optional:**
   - **Jacket hem flap:** in contact frames (4, 7) only, the jacket's back-lower corner may flare 1 px left and up, and settle on the down frames. It must not exceed 2 px or touch the near arm.
   - **Hair lag:** do NOT do it in this round. The identity test requires the hair cel to be an exact translation of the master hair. Hair lag needs an owner decision and a test change.
6. **Three hearts in motion.** On all 6 run frames, paste the master's third heart (master boxers, `R.R / RRR / .R.` at master x 21-23, rows 37-39), translated by the frame's body offset (canvas x 29-31 + 0, rows 39-41 + dy). Only paste it if all 9 cells land on boxer pixels whose current key is `W` or `V`, and the off-white notch pixel (the centre of the top row) stays `W`. If not, move it up by 1 row, at most twice. If it still does not fit, leave it out on that frame and list the frame in the PR. Never cut a heart, and never let it touch a `K` outline or a fist.

## 6. Timing and the playback rule (for the integration agent)

- Authored: 6 x **60 ms** (360 ms cycle). That is slip-free at 8 px / 60 ms = **133 px/s**.
- Speed-linked rule: `frame_ms = clamp(round(8000 / |vx_px_per_s|), 50, 90)`.
  - 145 px/s (Playground): 55 ms, slip-free.
  - 185 px/s (greybox `RUN_SPEED`): 43 ms ideal, so the 50 ms floor applies and leaves about 1.3 px of slip per frame (about 16 %, hard to see).
  - Below 89 px/s the 90 ms cap applies (walk-speed starts look slower, not frantic).
- Advance frames from accumulated simulation time (fixed tick), never wall clock, and keep the phase when speed changes. Animation must not feed back into physics.

## 7. Do not change

- Frames 0-3 (idle), 10-15 (jump and land): unchanged, already passed review. The only exception is frame 13's head/torso offset, which is already fixed.
- Keys 4, 6, 7, 9: leg and foot pixels, contact pixels, arm angles and body offsets. Only the polish in section 5 is allowed on them.
- The master-derived pixels (head, hair, torso, jacket, seat, pin): translation only. Face edits are limited to the existing `effort` brow.
- Canvas 48 x 56, pivot (25, 55), palette, layer names and order, tag ranges, and the manifest contract.
- Do not edit `export_assets.sh`, `validate_assets.py`, workflows, physics, camera, lighting, environment, Donut or Mongo files.

## 8. Execution checklist (Sonnet)

1. Redraw frames 5 and 8 to the tables. Apply item 2 (frame 8) and item 6. Item 5 hem is optional.
2. Rebuild the `.ase` from the grid: `python3 scripts/pixel_grid_to_ase.py build`, or `bash scripts/export_assets.sh`.
3. Checks:
   - row 55 is exactly as in the table for frames 4-9;
   - 0 isolated pixels and at most 2 orphans per run frame;
   - no transparent see-through pixel inside the body at the waist;
   - no fist at x >= 45;
   - `tests/test_art_carl_locomotion.py` passes, and the identity test is not weakened.
4. Update `carl-locomotion-anim-01.md` (run timing, foot-slip numbers, self-review entries that this spec resolves). Regenerate `docs/evidence/carl-locomotion-anim-01/` and its `SHA256SUMS` with `scripts/make_carl_locomotion_previews.py`, then make CI green.

## 9. Execution notes (Sonnet)

- **Frames 5 and 8:** the leg and Croc layers were redrawn (identical in both frames). Near/planted leg: hip (27,40), knee (31,46), ankle (26,50), flat Croc on row 55 x22-29. Tucked leg: knee (25,47), ankle (20,42), Croc stamp at rows 41-47, x17-22 (entirely above row 48, left of the planted shin, clear of the boxer hem). The tucked leg is drawn in a darker `S`/`D` ramp so it does not merge with the planted leg. Head, hair, torso, jacket, pin and boxers were already the correct master translations (body +2, +3) and are unchanged.
- **Frame 5 arms:** kept (they are the key-frame arm sprites placed 1 px from the table: near fist about (27,33) vs (27,34), far fist about (41,29) vs (41,28)). One outline pixel was added at (20,34) to remove an orphan.
- **Frame 8 near arm (item 2):** redrawn. Shoulder (22,24), elbow about (25,31), fist 4 px wide at x33-36 (centre about (34.5,28)), so 5 fist skin pixels (x35-36, rows 27-29) sit outside the jacket front edge (x34). The forearm's top outline next to the fist (x28-31, row 27) uses `T`. Sleeve width was not enlarged. The elbow-to-fist length is about 9 px, longer than the 7 px forearm of frame 7, because the target fist position is 9.5 px from the table elbow; I kept the table values. The frame-8 far arm already matched (fist about (36,32)) and is unchanged.
- **Item 6 (third heart): skipped on all six run frames.** At up 0 and 1 rows the 3 x 3 cell block hits the boxer hem or a `K` outline on every frame. At up 2 (frames 4, 5, 7, 8) all nine cells are `W`/`V`, but the heart touches a `K` pixel diagonally and its top right pixel is orthogonally adjacent to the second heart, which renders as one red smear (checked on frames 4 and 7). On frames 6 and 9 no placement fits. Needs an Opus decision (for example, moving hearts A and B).
- **Item 5 (optional jacket hem flap):** skipped. Hair lag: skipped as instructed.
- **Orphans (checklist 3):** frames 5 and 8 have 2 each (the translation-only master boxer edge pixels at x33). Keys 4, 7 and 9 have 3, 5 and 3, and frame 6 has 1; I did not touch the keys. All frames have 0 isolated pixels.
- **Checks:** row 55 is exactly x30-34 / 22-29 / 18-21 / 30-34 / 22-29 / 18-21 on frames 4-9; no fist at x >= 45 (widest fist column 43); no body see-through at the waist (frames 5 and 8 changed only below row 40 and in the arm); the identity tests pass unchanged.
