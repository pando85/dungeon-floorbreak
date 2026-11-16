# ANIM-DONUT-01 round 2: run and jump design spec (Opus design, for Sonnet execution)

**Status:** design spec, round 2. The keys are authored; the in-betweens, tests, evidence and docs are open work for the Sonnet execution agent. Nothing is accepted. The pipeline is the one in AGENTS.md "Model routing for art work": Opus designs, Sonnet executes, a separate Opus agent reviews, and the owner sees only the finished result.

This round fixes the independent Opus review of PR #36. BLOCKING 1: forepaws slide at trail speed. BLOCKING 2: the spine does not flex. Polish: tail sway, back-line steps, head bob, jump forelegs, and crouch versus landing.

## 1. Source of truth and what must not change

- The source of truth is the **committed grid** `assets/source/characters/donut/donut_h40_locomotion.grid.json`. `scripts/author_donut_locomotion.py` no longer writes it. `--round1-out PATH` refuses the committed path.
- **Finished keys (Opus): frames 4, 6, 8, 10, 12.** Do not edit their pixels. `python3 scripts/author_donut_locomotion.py --check-keys` must keep printing `round-2 keys match their recipe`. Their recipe is `ROUND2_KEYS` plus `PATCHES2` in that script. If a key looks wrong, report it back to Opus; do not re-decide it.
- **In-betweens (Sonnet): frames 5, 7, 9 (run) and 11 (jump push-off).** Frames 13, 14 and 15 stay as they are, apart from the optional notes in section 7.
- Never change these:
  - idle frames 0–3;
  - the master head, face and gear pixels (only whole-block translation is allowed; `tests/test_art_donut_locomotion.py` enforces this);
  - the palette, the canvas (72×44), the pivot (46, 43) and the layer names and order;
  - the master `donut_h40.grid.json`;
  - any Carl or Mongo file.
- **Tools:**
  - Build in-betweens with `author_donut_locomotion.compose(spec)`. It uses canvas coordinates, the round-1 parts (`TORSO`, `LEGS`, `stretch`, `arch`) and the round-2 parts (`TORSO_GATHER`, `TORSO_EXTEND`, `FN_LEAP`, `even_tilt`).
  - Write the result into the grid frame, then hand-fix single pixels until `pixel_grid_to_ase.py lint` reports 0 orphans and 0 isolated pixels.
  - Add the in-betweens' recipes to the script next to `ROUND2_KEYS` (for example an `INBETWEENS` dict plus their own patch list), so the frames stay reproducible. Do not route them through `--check-keys`.

Key poses for reference: `docs/evidence/anim-donut-01/round2_keys_6x.png`. It shows frames 4, 6, 8 / 10, 12, 14, with the walking line in brown.

## 2. Footfall sequence (transverse half-bound gallop)

Each paw touches row 43 in **exactly one frame**, and no paw is on row 43 in two consecutive frames. Planted paws therefore cannot slide between frames at any speed.

```
run frame        F0 (4)    F1 (5)    F2 (6)    F3 (7)    F4 (8)    F5 (9)
                 KEY       inbetw.   KEY       inbetw.   KEY       inbetw.
near fore        ####      .         .         .         .         _ (row 42)
far fore         .         ####      .         .         .         .
near hind        .         .         ####      .         .         .
far hind         .         .         .         ####      .         .
airborne?        no        no        no        no        YES       YES
spine            neutral   lowering  ARCH      unarching FLAT-LONG level
head (face y)    20        21        21        20        20        20
```

`####` = on row 43 (contact), `.` = in the air (lowest pixel row ≤ 41 for that paw), `_` = 1 px above the floor, landing next frame.

Spine, as a side-view sketch of the top of the back (x grows to the right; the head covers x ≥ ~36):

```
F2 gathered        ___________           rump curls under, back peak y18
                 _/                      3 px above neutral
F0 neutral   ____________________        y21 flat
F4 extended  ____________________________ y20 flat, body starts 9 px further back
```

## 3. Per-frame targets

Coordinates are canvas pixels (72×44, origin top-left, pivot (46, 43)). "Paws on row 43" lists the layer and the inclusive x range of opaque pixels in row 43. "Back top" is the topmost opaque pixel row of the `body` layer at the given x; `-` means no body there. "Face" is the top-left of the face layer's bounding box (run-neutral is (43, 20)). "Tail" is `compose` input: tail x, tail y, shear k.

| Frame | Role | ms @185 | Paws on row 43 | Lowest row | Back top y at x=16/20/24/28/32/36 | Face | Tail (x, y, k) |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 4 F0 | KEY | 80 | near fore 51–56 | 43 | -/-/24/21/21/21 | (43, 20) | (14, 6, 0.15) |
| 5 F1 | inbetween | 80 | far fore 53–58 only | 43 | -/-/25/22/20/20 (±1) | (43, 21) | (16, 7, 0.10) |
| 6 F2 | KEY | 80 | near hind 36–41 | 43 | -/-/27*/21/18/18 | (43, 21) | (19, 5, 0.05) |
| 7 F3 | inbetween | 80 | far hind 32–37 only | 43 | -/23/21/20/19/19 (±1) | (43, 20) | (12, 6, 0.10) |
| 8 F4 | KEY | 90 | none | 39 | 21/20/20/20/20/20 | (44, 20) | (5, 6, 0.20) |
| 9 F5 | inbetween | 80 | none | 42 | 23/21/21/21/21/21 (±1) | (43, 20) | (8, 6, 0.175) |
| 10 J0 crouch | KEY | 90 | hinds 29–34, 33–38; fores 50–55, 54–59 | 43 | -/-/-/24/21/20 | (43, 23) | (20, 7, -0.10) |
| 11 J1 push-off | inbetween | 70 | hinds only (push toes), x 18–26 | 43 | -/24/22/21/20/19 (±1) | (44, 19) | (10, 11, 0.25) |
| 12 J2 rise | KEY | 120 | near hind 12–16 (trailing toe) | 43 | 26/23/22/21/20/19 | (44, 18) | (9, 14, 0.30) |
| 13 J3 fall | keep | 120 | near fore 51–56 | 43 | as committed | (43, 19) | as committed |
| 14 L0 impact | keep | 80 | as committed | 43 | flat, low (24) | (43, 26) | as committed |
| 15 L1 recover | keep | 110 | as committed | 43 | as committed | (43, 19) | as committed |

\* At x=24, frame 6 is the rump curve (rump left edge x 24). The arch peak (y 18) spans x 32–41; neutral is y 21, so the arch is 3 px. Frame 8's body starts at x 13 versus x 22 in frame 4: a 9 px longer, flat back.

Rump left edge (first body column), which the tail base must follow: F0 22, F1 ~23, F2 24, F3 ~18, F4 13, F5 ~15.

### In-between recipes (starting points; hit the targets above, then clean by hand)

Leg tuples are `(LEGS name, side, canvas x, bottom row)`, the same convention as `ROUND2_KEYS`.

- **Frame 5, F1 (far fore contact, body lowering, near fore toe-off):**
  - Torso: round-1 `TORSO` with `stretch(..., 10, -2)` and then `arch(..., 6, 15, 1)`. Pad 6 empty rows above first, as `pose()` does. Place it so the rump's left edge is at x 23 and the back top is about 20–21.
  - `head_dy=1`.
  - Legs:
    - `("FN_plant", "far", 53, 43)`: the **only** row-43 pixels;
    - `("FN_push", "near", 41, 41)`: toe-off, lifted 2 px, about 10 px behind its F0 contact;
    - `("HN_tuck", "near", 28, 41)`;
    - `("HN_trail", "far", 21, 41)`.
- **Frame 7, F3 (far hind contact, near hind push-off, spine unarching):**
  - Torso: round-1 `TORSO` with `arch(..., 6, 15, 1)` and no stretch. Rump left edge at about x 18; back top about 19 over x 30–36.
  - `head_dy=0`.
  - Legs:
    - `("HN_plant", "far", 31, 43)`: paw 32–37, the **only** row-43 pixels;
    - `("HN_push", "near", 19, 41)`: lifted;
    - `("FN_swing", "near", 52, 41)`;
    - `("FN_tuck", "far", 49, 40)`.
- **Frame 9, F5 (descending, still airborne):**
  - Torso: round-1 `TORSO` with `stretch(..., 10, 2)`. Rump left edge about x 15; back top 21 (1 px below F4).
  - `head_dy=0`, `head_dx=0`.
  - Legs:
    - `("FN_reach_low", "near", 54, 42)`: lowest pixel row 42;
    - `("FN_reach_low", "far", 57, 41)`;
    - `("HN_trail", "near", 12, 40)`;
    - `("HN_trail", "far", 15, 39)`.
- **Frame 11, J1 (push-off between crouch 10 and rise 12):**
  - Torso: `even_tilt(TORSO_EXTEND, 6, up=True)`, a milder tilt than frame 12. Rump left edge about x 18; front back-top about y 19.
  - `head_dy=-1`, `head_dx=1`.
  - Legs:
    - hinds `("HN_push", "near", 18, 43)` and `("HN_push", "far", 22, 43)`: pushing toes on row 43;
    - fores `(FN_LEAP, "near", 50, 41)` and `(FN_LEAP, "far", 52, 40)`. These are already lifting and reaching. They must be visible **past the chest's front edge** (chest right edge ≈ x 62–63) and below the chin, not as nubs under it.

## 4. Speed-linked playback (for the integration agent; record it in the write-up)

Donut trails Carl, so she moves at Carl's ground speed `vx` (up to `RUN_SPEED` 185 px/s). The run is designed for a constant **D = 14.8 px of ground per frame**:

```
frame_ms(i) = clamp( round( w[i] * 1000 * D / |vx| ), 50, 130 )      D = 14.8 px
w = [1, 1, 1, 1, 1.125, 1]   (frame 8, the suspension, is held a little longer)
```

| |vx| | Base frame | Frame 8 | Cycle | Stride per cycle |
| --- | --- | --- | --- | --- |
| 185 px/s | 80 ms | 90 ms | 490 ms | 90.7 px |
| 145 px/s | 102 ms | 115 ms | 625 ms | 90.6 px |
| < 114 px/s | 130 ms (clamped) | 130 ms | 780 ms | ground travel per frame < 14.8 px; a walk cycle is out of scope |

The atlas stores the 185 px/s values (80, 80, 80, 80, 90, 80 ms). Sonnet must update frames 5, 7 and 9 to 80 ms; the keys already carry their round-1 durations, and their final values are those in the table in section 3.

**Slip (corrected in round 3, using the round-2 reviewer's measurements).** The round-2 claim "slip 0 at all speeds" was wrong. Single-frame contact only removes the *frame-to-frame* slide of a paw that stays on row 43. What remains:

| Effect | 145 px/s | 185 px/s | < 114 px/s (cap 130 ms) |
| --- | --- | --- | --- |
| Ground covered while a paw shows its one contact frame (`vx * frame_ms`) | 14.8 px | 14.8 px | `vx * 0.13` < 14.8 px |
| Near fore: lifted paw in the next frame, ahead of the ground point it touched (frame 4 → 5) | +3.8 px | +3.8 px | grows as speed drops |
| Far fore: same, frame 5 → 6 | +6.8 px | +6.8 px | grows as speed drops |
| Paws below the cap speed | n/a | n/a | the sprite's designed backward paw travel (≈ 11–15 px per frame) exceeds ground travel, so paws visibly drift backwards |

Round 1 for comparison (frame-to-frame slide of paws on row 43 in two frames): near fore ≈ 8 / 11 px, far fore ≈ 5 / 8 px, near hind ≈ 1 / 4 px at 145 / 185 px/s. The speed rule is only valid from about 114 to 185 px/s. Below that, the integration must switch to a walk cycle (not authored) or accept the backward drift.

## 5. Polish fixes Sonnet must do

- **Polish 2, tail sway halved:**
  - Use the shear k values in section 3. The tip lean changes by at most 2 px between neighbouring run frames (Δk ≤ 0.10 over the 20-row plume). Round 1 swung k between 0 and 0.35.
  - The tail base follows the rump (rump left edge minus about 8 px). That translation is body motion, not sway.
  - Do not add tail tufts or redraw the plume.
- **Polish 4, back-line steps and the saddle stripe:**
  - Keys 8 and 12 already fix this. Frame 8 uses the new flat `TORSO_EXTEND`, whose saddle is a compact patch behind the shoulder and not a stretched stripe. Frame 12 uses `even_tilt`, which gives steps of 1 px every 4 columns.
  - In the in-betweens, never use round-1 `tilt()`: its Python `round()` gives uneven steps. Never stretch the torso by more than 2 columns, because the saddle then turns into a stripe.
  - Acceptance: along the top of the back, no step is larger than 2 px except at the rump curve.
- **Polish 5, head bob 1 px:**
  - In the run, the face top-left y is 20 or 21 only (section 3), and the head never goes above 20.
  - `head_dx` is 0 except frame 8 (+1, a forward lunge in the stretch).
  - The jump and land frames may move the head more by design (crouch 23, rise 18, impact 26).

## 6. Acceptance checks for the Sonnet round

1. `python3 scripts/author_donut_locomotion.py --check-keys` passes; frames 4, 6, 8, 10 and 12 are unchanged.
2. Lint: 0 orphans and 0 isolated pixels in all 16 frames.
3. In run frames 4–7, exactly one paw is on row 43, at the x ranges in section 3 (±1 px). Frames 8 and 9 have nothing on row 43. Extend `tests/test_art_donut_locomotion.py` to check this, plus the 1 px head bob and that no paw is on row 43 in two consecutive run frames.
4. Back-top profiles within ±1 px of section 3, and the arch in frame 6 ≥ 3 px above frame 4 at x = 32. Frame 10 must not equal frame 14: crouch back-top y 21 at x = 32 versus impact y 24.
5. Re-export and re-run `scripts/make_donut_locomotion_review.py` and the Floor 1 compositions, then update the evidence README and SHA256SUMS. Add a footfall diagram and a speed-rule table to the evidence or write-up.
6. Update `docs/02-art/donut-locomotion-anim-01.md`: the frame table, the durations, the playback rule from section 4 (replacing round 1's "460 ms cycle"), and the round-2 provenance. Opus designed the keys and this spec; Sonnet executed the in-betweens and evidence.

## 7. Optional (only if time allows; does not block review)

- Polish 6: a rotary gallop (diagonal contact order) instead of transverse. Defer it; it changes the key contact x ranges.
- Frame 13 (fall): give the forelegs the same visible-past-the-chest treatment as frame 11, if the reviewer flags it.
- Frame 15: make its back top about 1 px lower than frame 4 so the recovery reads as rising.

## Round 3 (after the round-2 independent Opus review of head a9c002b)

Opus redesign, pushed with this section. Sonnet then executes the task list below. `--check-keys` now pins the round-3 versions of keys 6, 8 and 12. Keys 4 and 10 are unchanged.

### What Opus redrew (finished keys; do not edit)

- **Frame 12, B1 jump forelegs.** New parts `FN_REACH_NEAR` and `FN_REACH_FAR` replace the 20 px `FN_LEAP` sticks.
  - Near leg: about 12 px long, from under the chest ruff (x 50, row 36), angled forward and slightly down, ending in a 3×3 paw at x 58–62, rows 38–40.
  - Far leg: drawn behind the body, angled up; its paw shows past the chest front edge at x 63–67, rows 33–35.
  - The two paws are 3–6 rows apart and do not merge.
  - The tail was calmed to `(11, 12, 0.20)`; its tip is now at about (9, 12).
- **Frame 6:** `head_dx -1` (face top-left (42, 21)), so the gathered spine reads shorter. The far forepaw toe-off is raised to row 41 (bottom row 41, x 46–49).
- **Frame 8:** the extension is shorter and lands where the cycle can reach it. Torso is `stretch(TORSO_EXTEND, 12, -6)`, rump left edge x 19 (was 13), flat back y 20 from x 24 to x 40. Face (44, 20). The tail streams back a little more at `(11, 7, 0.25)` (tip ≈ (8, 7)). Hind legs re-attached under the rump: `HN_trail` near x 15, bottom 38; far x 18, bottom 37.

### Inchworm fix: body-length targets for the whole run

Body length = face x − rump left edge (first body column). Rump and head move together, so the stretch reads as the spine flexing between shoulders and hips, not as a rump sliding under a fixed head.

```
frame          4     5     6     7     8     9    (4)
               KEY   in    KEY   in    KEY   in
face x         43    43    42    43    44    44   43     head_dx  0  0 -1  0 +1 +1
face y         20    21    21    20    20    20   20     head_dy  0 +1 +1  0  0  0
rump x         22    23    24    21    19    20   22     max step 3 px (was 8)
body length    21    20    18    22    25    24   21     range 18-25 (was 16-31)
tail tip x     12    15    18   ~13    8   ~9    12      max step 5 px
```

### Round-3 task list for Sonnet (in this order)

1. **Frame 7 (F3) in-between:** keep the contact (far hind, x 32–37, the only row-43 pixels).
   - Move the torso 3 px right so the rump's left edge is at **x 21**: `torso_xy` (18, 14) → (21, 14).
   - Move the near-hind toe-off with it: `HN_push` near x 19 → 22, bottom stays 41.
   - Tail `(15, 6, 0.15)`.
   - Back top: y 19–20 over x 26–36, with **no 1 px dip** at x 34–37. The current dip comes from the arch ramp; extend `arch(..., 7, 18, 1)` to cover the columns under the head edge, or patch the top row there. The back must meet the head with no notch.
2. **Frame 9 (F5) in-between:**
   - Rump left edge at **x 20**: use the plain round-1 `TORSO` (24 columns; drop both stretches), `torso_xy` (20, 15).
   - `head_dx +1`, `head_dy 0` (face (44, 20)).
   - Tail `(12, 6, 0.20)`.
   - Hinds re-attached under the new rump: `HN_trail` near x 17, bottom 40; far x 20, bottom 39.
   - Fores unchanged: lowest row 42 at x 54–61.
   - Back top: y 21 flat from x 22 to the head, **no dip** at x 34–36.
3. **Frame 5 (F1) in-between:** keep everything (rump x 23, face (43, 21), far-fore contact x 53–58). Remove the 1 px back-line dip at **x 37** (raise that column's top pixel to match x 36 and x 38, with the correct `j`/`H`/`B` stack).
4. **Frame 11 (J1, push-off) in-between:**
   - Replace both `FN_LEAP` legs with `FN_REACH_NEAR` / `FN_REACH_FAR`, keeping the frame 12 arrangement 1–2 px lower, since the body is lower in frame 11.
   - Starting points: `(FN_REACH_FAR, "far", 56, 39)` and `(FN_REACH_NEAR, "near", 49, 42)`.
   - Acceptance: near paw a 3×3 blob with its tip at x 60–63, below the chest bottom; far paw visible past the chest front edge (x ≥ 63); the two paws at least 3 rows apart; neither leg longer than 13 px.
   - Smooth the jump tail: the tip moves at most 4 px between frames 10 → 11 → 12 → 13. Tips: frame 10 (20, 7), frame 11 target ≈ (15, 10), frame 12 (9, 12). Starting value for frame 11: tail `(16, 10, 0.10)`.
5. **Frame 13 (J3, fall), tail only:** the tip is at (14, 4); move it to about (12, 9) so frame 12 → 13 moves ≤ 4 px. Starting point: tail `y` +4 and `k` +0.05 in its recipe. Leave the body and legs alone.
   - Frame 13 is round-1 output in the grid. Either add a recipe for it to `INBETWEENS` (preferred), or patch its tail cel by shifting rows, with no redraw.
6. **Tail sway check (run):** shear k for frames 4–9 is now 0.15, 0.10, 0.05, 0.15, 0.25, 0.20 (Δk ≤ 0.10 between neighbours, including 9 → 4); tips as in the diagram above.
7. **Tests:** update `tests/test_art_donut_locomotion.py` to the round-3 numbers:
   - face x 43/43/42/43/44/44 and y 20/21/21/20/20/20 in frames 4–9;
   - rump left edge 22/23/24/21/19/20 (±1);
   - back-top profiles from the measured grid (frame 8 now flat y 20 from x 24; no single-column dip of 1 px or more within 4 px of the head edge in frames 5, 7 and 9);
   - the tail k list above;
   - frame 12 forelegs: two separate near/far paw clusters, each at most 13 px long.

   Three current tests fail on the round-3 keys until this is done: `test_back_top_profiles`, `test_run_head_bob_is_one_pixel` and `test_tail_sway_is_halved`.
8. **Docs:** in `docs/02-art/donut-locomotion-anim-01.md` and the evidence README, replace every "slip 0" claim with the corrected table in section 4 (14.8 px per contact frame; +3.8 / +6.8 px fore offsets; backward drift below 114 px/s). Add the body-length diagram above. Record provenance: round-3 keys and spec by Opus, execution by Sonnet.
9. **Evidence and CI:**
   - Re-export, re-run `scripts/make_donut_locomotion_review.py`, refresh SHA256SUMS, and update the `round2_keys_6x.png` reference image (already regenerated by Opus for frames 4, 6, 8, 10, 12, 14).
   - Add a rump/back-line onion image, frames 4–9 overlaid, like the reviewer's `run_onion_backline_rump_8x.png`.
   - Make CI green.

Do not change keys 4, 6, 8, 10, 12, the idle, the head, face or gear pixels, the palette, the canvas or the pivot. If a target above conflicts with a key, report it back to Opus.
