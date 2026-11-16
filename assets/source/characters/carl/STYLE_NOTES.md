# Carl, Book 1 "Survivor": native pixel masters (round 2)

**Status:** hand-authored candidate masters for the 48 / 52 / 56 px scale study, **round 2 redraw**. **Not approved art.** Human review of silhouette, personality and game feel is still required (`AGENTS.md` work cycle step 7). The rights basis for publishing them is the owner-reported `ART-PUBLICATION-ALL-V1` record in [`rights-and-release.md`](../../../../docs/04-production/rights-and-release.md); it is not independently verified and grants no reuse licence or game/asset-pack distribution right.

Round 2 replaces every pixel of round 1. The round-1 review found a mid-brown jacket, a spiky anime crest, a weak face, hearts that read as red "Y"/"T" shapes, a stiff, symmetric front-on pose with a fist clutching the waist and a tube-like far arm. Each of these points was checked against the concept board and confirmed before the redraw.

## Files

| File | Opaque height | Canvas | Pivot (feet centre, contact row) | Palette |
| --- | --- | --- | --- | --- |
| `carl_h48.grid.json` | 48 px | 36 x 52 | [18, 51] | 23 keys, all used |
| `carl_h52.grid.json` | 52 px | 36 x 56 | [19, 55] | 23 keys, all used |
| `carl_h56.grid.json` | 56 px | 40 x 60 | [22, 59] | 23 keys, all used |

- `pixel-grid-v1`. 48 and 56 are one frame (200 ms) tagged `idle`. **52 has a 4-frame `idle` (frames 0 to 3, `forward`)**; see [52 px idle](#52-px-idle-4-frames). All are fully opaque, with 4 rows of headroom, hair top is the highest pixel (row 4) and the bottom row is the contact row.
- `scripts/pixel_grid_to_ase.py lint GRID --strict --expect-height N` exits 0 for all three: 0 orphans, 0 isolated pixels, contact row equals pivot y. `build` also succeeds. No `.ase`, manifest or export is committed in this round.
- Pivot x is the midpoint between the two Croc sole runs on the contact row. At 52 it is exact (13.5 and 24.5 give 19). At 48 (12 and 23) and 56 (15 and 28) the true centre is x.5 and is rounded to 18 and 22.

## Provenance and method

- Each size was drawn independently on its own grid. No size is a resample of another. They share a design plan (pose, palette, layer map, costume decisions) but not pixels.
- No pixel of the concept board, of its Survivor figure or of its tiny mock sprites was copied, traced or quantised. The board was only viewed as enlarged crops: the Survivor panel for costume and pose, and the "A. Messy", "Neutral" and "WTF" heads for hair shape and expression. The colour values are hand-picked, informed by the board's Carl swatch strip (skin, hair, jacket/cloth plum-greys, boxers, accents) and then shifted toward plum for the gothic setting.
- Authoring used a scratch layered text format (one block of row segments per layer, composited bottom to top) plus nearest-neighbour review renders. Those helper scripts are scratch tooling and are not committed. The grids are the source of truth.

## Layers (same names and order at every size)

Bottom to top: `far_arm`, `legs`, `shoes`, `torso`, `boxers`, `jacket`, `head`, `hair`, `near_arm`, `watch`, `pin`.

- Most occluded areas are filled so layers can be moved for `run` and `hurt`: the full torso skin under the jacket, the jacket's back panel under the near arm, the forehead under the fringe, the far sleeve behind the far jacket panel, and the legs about 3 rows up under the boxer hem. The thighs above that, up to the waist, are **not** drawn.
- `watch` (band plus a 1 or 2 px face) and `pin` (2 x 2 yellow) are separate so either can be removed without touching the arm or jacket.
- `head` holds face skin, ear, brows, eyes, nose and mouth. `hair` holds the mop, fringe locks, sideburn and nape.

## Pose and read

Idle, facing right. The body is in roughly a 45 degree three-quarter view: the near arm hangs at the side of the torso and the far arm hangs just in front of the far jacket panel, both visible. The head turns a little more toward the camera than the body, as a wary sideways glance (bible: "idle: weight on one leg, small wary glance"), and sits forward of the shoulders for a mild slouch.

- **Weight:** the near (left) leg is straight and planted under the hip. The far leg is relaxed, with the knee bent forward 2 to 3 px and the foot planted ahead. Both Crocs sit flat on the contact row.
- **Arms:** both hang loosely. The near hand reaches mid-thigh, about 2 rows above the boxer hem, with the back of the hand to camera, the thumb forward and the wristwatch on the wrist. The far forearm and hand come forward past the boxers' front edge in shadow tones. No fist clutches the waist.
- **Body:** ordinary young-adult build. The shoulders are only a little wider than the loose boxers, with no V-torso. There is a soft belly bulge in the jacket opening, a slightly hairy sternum (2 to 3 shadow pixels), no abdominal modelling, and sturdy, plain legs.
- **Face:** plain and unglamorous: a soft, slightly heavy jaw, no chiselling and no stubble. The near brow sits low and flat over the eye (irritated). The far brow is raised one row (bewildered). Both eyes show white, with pupils toward the right. A 2 px dark open mouth with a shaded lower lip gives the "huh?" of the board's Neutral/WTF heads.

## Light, outline and cast consistency (aligned with Donut round 2)

- Light comes from the upper left front, the same direction as Donut's notes. Lit edges are on the top and left, and the shadow side is on the right and underneath.
- The global outline `K` is `#1b1124`, identical to Donut's `outline_plum`. It is used selectively: it traces the jacket, hand and boxer outer edges and the hair nape. Skin uses its own darker tones (`S` on lit edges, `D` on shadow edges), the Crocs use `X` and the hair uses `A`.
- Shadows hue-shift toward plum and violet: skin goes to `#c0624f`, then `#7c3846`, the boxers go to lavender, the hair shadow is plum-brown and the jacket shadow is violet-black. There is no pure black and no grey.

## Palette (identical in all three files)

| Key | Name | Hex | Role |
| --- | --- | --- | --- |
| `K` | outline_plum | `#1b1124` | selective outline (jacket, hands, boxers, nape); same as Donut |
| `D` | skin_deep | `#7c3846` | skin shadow-edge outline, jaw underside, ear lobe |
| `S` | skin_shade | `#c0624f` | skin shadow, lit-edge skin outline, chest/leg hair hint |
| `M` | skin_mid | `#eb9a6c` | skin base |
| `L` | skin_light | `#fcc596` | forehead, cheek, knees, knuckles, nose bridge (56) |
| `A` | hair_dark | `#28131c` | hair shadow, nape, brows |
| `H` | hair_mid | `#42231d` | dark-brown hair base |
| `h` | hair_light | `#70412d` | swept strand highlights, fringe locks |
| `J` | jacket_shadow | `#251a2e` | near-black leather shadow, arm cast shadow, hem |
| `j` | jacket_base | `#352540` | near-black plum leather base |
| `T` | jacket_sheen | `#5a4068` | lit leather edges, collar, lapel, creases, bunched cuffs |
| `Y` | jacket_glint | `#9478a8` | accent: 1 px leather glints (shoulder, cuff, pocket) |
| `W` | boxer_white | `#f7e4de` | off-white boxers |
| `V` | boxer_shade | `#c3a9c4` | boxer shadow side, waistband |
| `U` | boxer_deep | `#82689a` | hems, crotch fold, drawstring |
| `R` | heart_red | `#ec2f55` | 3 x 3 heart print |
| `P` | croc_light | `#ff86c8` | Croc collar rim, toe dome, strap rivet |
| `Q` | croc_pink | `#e23d9a` | magenta-pink Croc body |
| `X` | croc_shade | `#97207a` | Croc outline, sole, hole hints |
| `E` | eye_white | `#f6ecec` | accent: eye whites |
| `p` | pupil | `#140b1a` | accent: pupils and the open mouth (darkest value) |
| `G` | pin_yellow | `#ffcf3a` | accent: smiley pin |
| `w` | watch_face | `#b8c0d8` | accent: watch face |

23 colours (budget 24). Accent keys (`Y`, `E`, `p`, `G`, `w`) are the only deliberate single-pixel details. The palette remains provisional until the three-character, one-room colour test.

## Proportions (measured on the grids)

| Part | 48 px | 52 px | 56 px |
| --- | --- | --- | --- |
| Head, hair top to jaw underside | 12 rows (25%) | 13 rows (25%) | 14 rows (25%) |
| Hair above the forehead | 4 rows | 5 rows | 6 rows |
| Face, forehead to jaw | 8 rows | 8 rows | 8 rows |
| Head width incl. hair | 15 | 17 | 18 |
| Jacket rows (collar to hem) | 14 to 29 | 15 to 32 | 16 to 34 |
| Boxers rows (waistband to hem) | 28 to 36 | 31 to 40 | 33 to 43 |
| Bare leg below the hem | 10 rows | 10 rows | 11 rows |
| Croc (strap row + body) | 1 + 5 rows, 9 px long | 1 + 5, 10 px | 1 + 5, 11 px |
| Silhouette width (bbox) | 22 | 23 | 25 |
| Opaque pixels | 731 | 837 | 977 |

About 4 heads tall, against roughly 5.3 on the board: the head is deliberately enlarged so the face carries at gameplay scale (owner-allowed). Build is stocky and soft rather than lean.

## Per-size pixel budgets

| Element | 48 px | 52 px | 56 px |
| --- | --- | --- | --- |
| Near eye | 2 x 1 (white + pupil) | 2 x 1 | 3 x 1 (wide white + pupil) |
| Far eye | 2 x 1 | 2 x 1 | 2 x 1 |
| Brows | near 2 px low; far 2 px raised one row | near 2 px; far 2 px raised | near 3 px; far 2 px raised |
| Nose | 1 px tip past the face line + 1 px shadow | 2 px side/underside shadow beside the far eye, no protruding tip | 2 px lit ridge + 1 px tip past the face line + 2 px underside shadow |
| Mouth | 2 px open (pupil tone) + lip shade | same | same, set further forward |
| Near hand (skin below the watch) | 5 x 4 | 6 x 5 incl. thumb | 6 x 6 incl. thumb |
| Far hand (visible skin px) | 15 | 16 | 20 |
| Watch | 1 px face on a 5 px band | 1 px face, 6 px band | 2 px face, 6 px band |
| Hearts | 2 x (3 x 3) | 3 x (3 x 3) | 3 x (3 x 3) |
| Hair (opaque px) | 61 | 88 | 109 |
| Croc holes | 3 hints + rivet | 3 hints + rivet | 3 hints + rivet |
| Leg hair | none | 1 two-pixel stroke per shin | 1 to 2 strokes per shin |

### Heart decision

Every heart is `R.R / RRR / .R.` on off-white. The off-white notch between the lobes is what makes it a heart, so no heart is placed where its notch would touch the jacket hem or a hand. That rule also keeps the lint clean. Hearts are spaced in a sparse triangle (52, 56) or a diagonal pair (48), at least 1 to 2 px apart, so they never merge. An earlier 52 pass hid a heart lobe under the thumb, and it read as a red "Y" again.

Readability, checked on native renders: at 4x and above they are unmistakably hearts. At 2x they read as small red "v"/heart marks on the white. At 1x they read as red specks: the white boxers with a red print read, but the heart shape does not. No 3 px heart reads as a heart at 1x.

## Simplifications by size

- **48:** two hearts. No leg hair. 4-row hair cap with three swept strand runs. 1 px watch face. 2 x 2 pin. The chest opening is 2 to 3 px wide. The face is still complete (two eyes with white, both brows, nose, open mouth), but every feature is 1 to 2 px.
- **52:** the reference design. Three hearts, chest-hair hint, navel, leg-hair strokes, leather creases and pocket glint, both hands readable.
- **56:** the extra room goes into a wider near eye (the wary glance reads most clearly here), a 2 px watch face, a modelled nose and a lower jaw line. The pin stays 2 x 2: a smiley needs 5 px of width to show two eyes and a mouth, and no size has room for that on the near jacket panel.

## Canon and source labels

| Element | Label | Note |
| --- | --- | --- |
| Short, dense, tousled dark-brown mop; plain, mildly asymmetric, irritated or bewildered face; ordinary soft build | bible-specified (Stage I) | No stubble, cigarette or abs: the bible calls these illustration noise |
| Near-black battered leather jacket, worn open over a bare chest | bible-specified ("nearly black leather", "battered jacket") | Plum-black leather with sheen and glints |
| Full-length sleeves, bunched and pushed down to the lower forearm | concept-derived **interpretation** | The board shows sleeves pushed up past mid-forearm. The brief asked for full-length sleeves plus a visible watch, so they are drawn as long sleeves bunched at the lower forearm. Flagged for owner review |
| Heart-print off-white boxers with a drawstring hint | bible-specified (heart print), concept-derived (drawstring) | |
| Magenta-pink Crocs with hole hints and heel strap | bible-specified ("appropriate early-story pink Crocs") | Model and wear are not specified |
| Smiley pin (`pin` layer) | concept-board identity element, **kept per owner direction for this round**, not book-verified | Drop the layer if book evidence contradicts it |
| Wristwatch (`watch` layer) | concept-derived, kept per owner direction | |
| Weapons, shotgun, cigarette, lighter, extra gear | **omitted** | The shotgun is explicitly banned |

## Critique log (summary)

Every round compared 1x, 2x, 4x and 6x renders on `#191a2c` and `#2c2841` with an enlarged Survivor crop at matched height, plus 10 to 16x gridded close-ups. Each round asked: Is he instantly Carl at 1x? Is he ordinary and out of his depth? Does the face read? Do the hearts read? Is the jacket black leather? Is the hair a messy mop?

- **52 (about 9 rounds).** (1) Block-in: a narrow vest-like jacket slab, chest too wide, no far arm, sleeve merged with the body. (2) Layered rebuild: correct order, both arms, and boxers wider than the legs. (3) Proportions: limb lengths set (arm to mid-thigh), Croc toe dome, hair darkened. (4) Head redesign: a near-profile head read feminine with a bun-like back mass, so the head was turned toward camera (two eyes, a wary glance), the hair dome lifted and the back mass reduced. (5) Contrapposto: relaxed far knee forward, hearts re-spaced, collar and lapel added. (6) Leather creases, cast-shadow column separating the sleeve from the panel, chest hair and navel, new Croc profile with strap and holes. (7) Lint pass: pupil accent key added, strands connected, heart rows moved off the hem. (8) Far hand brought forward to read as a hand, rounder dome, highlights reorganised into swept strand runs because the scattered ones read as a curly perm. (9) Open-mouth "huh?" expression; hearts moved off the thumb.
- **56 (6 rounds).** (1) Native draft from the shared plan. (2) Far sleeve and hand rebuilt so only a clean strip shows past the far panel. The near eye was widened to 3 px. (3) The far-side hair column that darkened the face edge was removed, and the nose given a lit ridge. (4) A mustache-like 3 px mouth was cut to 2 px and the near calf thickened. (5) The chin was receded one pixel to soften a heavy jaw. (6) Swept strand hair and the open-mouth expression.
- **48 (5 rounds).** (1) Native draft. (2) Hearts reduced to two after three crowded into a red cluster. Leg-hair stripes removed because they read as bands. Pocket glint, collar and neck lint fixed. (3) Hearts placed on a diagonal, clear of the hand. (4) Swept strand hair. (5) Open-mouth expression.
- **Cross-size check** (three-up at 1x, 2x, 4x and 6x, and a 640 x 360 scene beside Donut 32/40/48 and Mongo 64/88 height bars): same costume map, layer set, light direction and expression at all sizes. At 1x all three read as dark mop, near-black jacket with a yellow dot, skin chest strip, white boxers with red, skin legs and pink Crocs.

## Ranking (my judgement, not approval)

1. **52 px, recommended.**
   - Pros: the best balance of face, costume and silhouette. The face reads as a face at 2x (brows, both eye whites, open mouth). Three hearts read as hearts at 4x. Both hands are readable, and the stocky, slightly slouched body reads as an ordinary guy rather than an athlete.
   - Relative scale: Donut 40 is about 77% of his height. Mongo 64 to 88 is 1.23 to 1.69 times his height, so the raptor stays clearly bigger.
   - Authoring cost: moderate, 837 opaque pixels per frame.
   - Cons: the hearts are specks at 1x, the pin is a plain square, and the watch is a band and a dot.
2. **56 px.**
   - Pros: the most expressive face (the wide near eye sells the wary glance) and the best hands, watch and nose. The likeliest to hold up at Steam Deck 1280 x 800 (2x).
   - Cons: the face is longer and the jaw heavier, drifting a little toward a leading-man head. About 17% more pixels to animate than 52.
   - Relative scale: against Mongo 64 the ratio is only 1.14, so Mongo would need about 72 px or more to keep his mass. It is the most comfortable against Donut 48.
3. **48 px.**
   - Pros: the most economical, at 731 pixels (13% fewer than 52). The stockiest and most readable silhouette at 1x, with the head the same 25% share.
   - Cons: only two hearts, a 1 px watch face, no leg hair, and every facial feature is 1 to 2 px, so the face is first to collapse in motion.
   - Relative scale: it would equal Donut's 48 px variant (the 48 Donut is not the recommended one; Donut 40 is), and it leaves Mongo 64 at 1.33 times.

## Remaining weaknesses (honest)

- **Hearts at 1x:** they are red specks. At 2x they are only "v"-ish marks. They read as hearts at 4x and above.
- **Pin and watch:** the pin is a 2 x 2 yellow square with no smiley face. The watch is a dark band with a light 1 or 2 px face.
- **Run conversion:** the body is three-quarter (about 45 degrees) and the head is turned further toward camera. A side-view `run` will need the torso rotated closer to profile and the head redrawn. The near hand also covers the near side of the boxers.
- **Jacket on `#191a2c`:** the outline nearly merges with the background. On 52 the near sleeve now has a lit sheen rim instead of a dark outline on its left edge (see the idle section), which separates it there. The far sleeve and the 48 and 56 masters still rely on sheen and glint pixels.
- **Hair at 1x:** the mop reads as tousled at 2x and above. At 1x it is a brown cap, though no longer a spiky crest.
- **Legs and Crocs:** the legs are simple ramps with little calf or knee modelling. The two Crocs share one design, with no individual wear.
- **Pivot rounding:** at 48 and 56 the pivot is a half-pixel rounding of the true feet centre.
- **Unreviewed:** no human has judged these in a game scene yet. Silhouette, personality and relative scale need owner review.

## 52 px idle (4 frames)

Every frame is derived from the frame-0 master by scripted, listed edits. No frame is redrawn independently and no limb is rotated. Tag `idle`, frames 0 to 3, direction `forward`, 1500 ms loop.

| Frame | Duration | Change from frame 0 | Top row / height |
| --- | --- | --- | --- |
| 0 | 420 ms | master: chest up, bewildered face (raised far brow, open mouth, eyes looking right) | 4 / 52 |
| 1 | 260 ms | exhale: row 26 is dropped and every layer row above it moves down 1 px, so the head, hair, shoulders, chest, collar, pin and upper sleeves settle by 1 px. The pocket glint is pinned in place so it does not blink. Rows 26 and 27 of the master differ only in that glint, so the seam is invisible | 5 / 51 |
| 2 | 560 ms | frame 1 plus an irritated side-glance: both pupils swap to the back of the eye (`Ep` to `pE`), the far brow drops from its raised position into a level scowl, and the open mouth closes to a flat 2 px line | 5 / 51 |
| 3 | 260 ms | frame 0 (chest back up) still holding the glance edits from frame 2 | 4 / 52 |

- **Beat:** startled, then "...seriously?" He settles, side-eyes back over his shoulder with a scowl, straightens, and the face resets to bewildered at the loop. The body bob (frames 0 to 1 and 2 to 3) and the face change (frames 1 to 2 and 3 to 0) never happen on the same frame.
- **Fixed parts:** nothing at or below row 27 changes in any frame: cuffs, hands, watch, boxers, hearts, legs and Crocs. The contact row 55 is identical in all four frames, the pivot stays [19, 55], and lint reports 0 px bottom-row drift. Pixels changed against frame 0: 0 / 199 / 198 / 11.
- **Lint:** `lint --strict --expect-height 52` passes: 0 orphans and 0 isolated pixels in every frame. Frame 0 height is 52; frames 1 and 2 measure 51 because the head bobs down.
- **Review:** GIFs at 1x and 4x on both backgrounds, a 6x strip with a pivot marker, and a per-frame diff map, all outside the repository. There is no jitter in the hands or feet and no shimmer in the jacket. The 1 px head and shoulder drop is visible at 4x and just perceptible at 1x.
- **Frame-0 polish in this pass (low risk):** the near sleeve's left silhouette edge (rows 18 to 30) is now the lit `T` sheen instead of the `K` outline. The column inside it went to base `j`, and the bunched cuff keeps its sheen. 21 px changed. This separates the sleeve from `#191a2c`, where `K` is almost the background colour. I did **not** change heart readability at 2x: `R.R / RRR / .R.` is the smallest heart with a notch, and a bigger heart would mean re-laying out the whole print, which is not low risk.

## What a 6-frame run would need from this master

- **Profile torso:** rotate the torso from about 45 degrees to near profile. The open jacket then shows as a front panel edge, a sliver of chest and a swinging hem. The pin moves to the visible front panel.
- **Profile head:** a one-eye head built from the 52 face parts (low brow, eye white plus pupil, nose tip past the face line), not the current camera-facing glance.
- **Arm and leg layers:** the near and far arms need complete shoulder-to-hand cels for the swing. The thighs must be drawn up to the waist, since they are currently filled only about 3 rows under the boxer hem.
- **Crocs:** a second, foreshortened push-off pose plus the planted version. The contact row and pivot rules stay as here, with an awkward, slightly heavy stride rather than an athletic one.
- **Bob:** reuse the idle's row-drop technique for the 1 px body bob instead of redrawing.

## Open canon questions

1. Sleeves: should they be pushed up past mid-forearm, as on the board, or long and bunched, as drawn here?
2. Is the smiley pin book canon, and on which side of the jacket?
3. Is the jacket's open state, with nothing underneath, canon for the opening chapters?
4. What are the exact early-story Crocs: model, colour and condition?

## Review renders

Nearest-neighbour review renders are kept outside the repository: per size at 1x, 4x and 6x on both backgrounds, a three-up at 1x, 2x, 4x and 6x, a 6x three-up beside the matched-height Survivor crop, per-layer sheets, and a 640 x 360 scale scene with Donut and Mongo height bars. Regenerate any of them with `scripts/pixel_grid_to_ase.py render`.
