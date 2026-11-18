# TUSK-01: Tusklings readability set, design spec

**Status:** design stage. Spec written; hero key 0 (`idle`, tux costume) drawn and frozen; the gown variant is a rule on the same key. Silhouette gate on key 0 at 1x pending. Frames 1 to 15 are for the execution tier (sections 4, 7 and 8).

**Decision record:** one base Tuskling, 30 px idle height (rows 10 to 39), frame 56 x 40, pivot (22, 39), contact row 39; two costume variants (`tux`, `gown`) as a costume-layer key remap plus an accessory swap, not separate drawings; the SWINE-01 palette unchanged (19 keys, slot 20 left free).

The sections follow `docs/02-art/swine-01-spec.md` and `docs/02-art/gob-01-goblin-spec.md`. All coordinates are canvas pixels (x right, y down) unless marked pivot-relative.

## 1. Canon and tone constraints

- Approved concept: `assets/concept/floor-01/enemies/floor01-ball-of-swine-approved.png`, recorded in `docs/02-art/floor-01-late-enemy-boss-concepts.md` as an owner-approved visual concept for the Ball of Swine and the Tusklings. Crops for reference only: `docs/evidence/swine-01/packet/concept_crop_5.png` (three "knights" in torn tuxedos, bow ties, top hat, monocle, cane) and `concept_crop_6.png` (three "courtesans" in sequin gowns, feather boa, fan, pearls, a purple hair flower).
- Canon facts that must survive in pixels: tusked, pig-like, short-legged, aristocratic creatures; distinguishable inside the Ball and on their own; the Ball and the Tusklings share one visual family (SWINE-01 section 1).
- Tone (`docs/01-design/antihero-tone.md`): grotesque and grounded, threatening with an absurd contrast, never cute and never a faceless army. Design reading: a squat, hunched boar-man with the head carried low and forward, a pot belly and short bowed legs on split trotters; the same snarl as every head in the Ball (upper lip arched off red gums, fangs, lower teeth, a lolling tongue), two ivory tusks hooking up past the snout, a small sunk eye with a bag under a scowling brow, coarse bristle along the hump. The absurd note is the soiled high-society costume on swine (an open tux with a stained shirt and a red bow tie, or a red sequin gown with pearls and an ear flower). The danger is the tusks at Carl's thigh height and the numbers.
- Not approved (SWINE-01 "What is NOT approved"): the Tuskling count (the book says 30+; the sheet shows about a dozen faces), separate hitboxes, combat archetypes, post-breakout abilities, the "Knights" and "Courtesans" captions as production classes, damage and hit points. The props on the sheet (cane, monocle, top hat, fan, boa) are not in this set; the named Dominatrix of Chapter 27 is not this asset.
- Guardrails: no tracing or sampling of the concept; deterministic script output only; no image-model raster output.

### Why one body and two costumes

The sheet draws six costumes on what is one body plan (stocky biped, big tusked head). Six bodies would turn a route-C derived asset into six sets. One body carries every pose; the costume sits on its own layer, so a variant is a remap of the costume layer plus a few accessory pixels at fixed anchors. Two variants cover the sheet's two groups: `tux` (the knights) and `gown` (the courtesans). A mob of mixed Tusklings tells apart by the torso colour block (black against red) at 1x, while the silhouette, pivot, boxes and timing stay identical.

## Difficulty and route

**Score 1 of 4: Easy, route C** (the designer writes the spec and draws one hero key; the execution tier draws the rest). Matches the SWINE-01 estimate.

| Point | Met | Note |
| --- | --- | --- |
| 1. New design, not a variant | yes | No approved Tuskling asset; the head language comes from the Ball |
| 2. Native height above about 40 px | no | 30 px idle |
| 3. Complex anatomy | no | One stocky biped; the costume is one layer |
| 4. VFX, projectiles, special states | no | None; the Ball's break-apart belongs to the Ball (separate VFX work order) |

Reasoning: it stays at 1 only while the variants remain a layer rule. Drawing per-costume bodies, props (cane, fan) or a post-breakout ability would raise it to 2.

## 2. Size decision

Carl 52 px (about 6 ft), Donut 40, goblin 36, Scatterer 15 (opaque idle). The Ball's rim heads are radius 11 (22 px across); the Ball's large centre heads are a foreground exaggeration of a squeezed mass.

| Option | Idle height | Ratio to Carl | Implied height | Why / why not |
| --- | --- | --- | --- | --- |
| A | 24 px | 0.46 | 2.8 ft | The head drops to about 11 px, the snarl needs 5 rows (lip, gums, fangs, lower teeth, jaw), which leaves no room for a brow over the eye; the tux V is 2 px; the result reads as a piglet, which is the cute risk |
| B | 30 px | 0.58 | 3.5 ft | Head 13 x 12 px plus a 4 x 7 px snout disc (19 px across with the snout, close to the Ball's 22 px rim heads), a 5-row snarl, a brow and a sunk eye, tusks 6 to 9 px; tusks at Carl's thigh; under the goblin, so a mob of 3 to 6 reads as minions |
| C | 36 px | 0.69 | 4.2 ft | Same height as the goblin: minion and goblin read as one tier; 1.44 times the pixels per frame; six on screen crowd the 640 px view |

Decision: **B, 30 px** (rows 10 to 39 of key 0, ear tip to trotter outline). Width 36 px (x 7 to 42): the body is hunched and long, a boar carried upright.

## 3. Frame, pivot, boxes and atlas

- Frame 56 x 40, pivot (22, 39), contact row 39 (the trotter outline row). The free space is for the gore: 14 px ahead of key 0's tusk tip (x 42 to 55) and 7 px behind (x 0 to 6) for the scraping rear trotter of the tell.
- Authored facing right; the runtime mirrors around the pivot column.
- Layers: `body` (flesh, face, tusks, bristle, hooves, outline) and `costume` (jacket, coat-tails, sleeve, shirt, bow tie). The layers are disjoint (validator V10): a costume pixel is never also a body pixel.
- Grounded tags: all. Airborne tags: none (the gore lunges along the floor).
- Boxes (placeholders, pivot-relative, x right, y up; damage, knockback and stagger not approved):
  - Collision: x -11..+12, y 0..+24.
  - Hurtbox: x -13..+16, y 0..+27 on every frame except the `down` hold.
  - Gore hitbox: x +22..+31, y +12..+24 (frame pixels x 44 to 53, rows 15 to 27) on frame 10 only.
- One atlas per variant (`tusk_01` tux, `tusk_01_gown`), frames 0 to 15, exported from the same grid by `variant()`.

## 4. Readability-set animation list

16 frames; keys marked K are frozen once drawn (key 0 now, the others by the executor after review).

| Frame | Tag (role) | Ticks | Pose |
| --- | --- | --- | --- |
| 0 K | `idle` (idle) | 18 | Key 0 (section 7): hunched, head low, snarling, near arm hanging over the shirt |
| 1 | `idle` | 12 | Breathe in: trunk, arm and head up 1 px, belly 1 px wider at rows 32 to 35; legs and trotters fixed |
| 2 | `idle` | 14 | Snort: head (skull, snout, tusks, ears) down 1 px and 1 px right, tongue 1 px longer; trunk as frame 0 |
| 3 K | `scramble` (move) | 5 | Contact A: whole upper body 2 px right and 1 px down (pitched forward); near trotter 4 px forward (x 27 to 33), far trotter 3 px back (x 11 to 15); coat-tails flick up 1 px |
| 4 | `scramble` | 5 | Passing: upper body 1 px up from frame 3; trotters under the body, near at x 21 to 27, far at x 16 to 20 lifted 1 px (outline still on row 39 under the near trotter) |
| 5 | `scramble` | 5 | Contact B: frame 3 with the legs swapped (far trotter forward to x 24 to 28, near trotter back to x 17 to 23) |
| 6 | `scramble` | 5 | Passing: frame 4 with the legs swapped |
| 7 | `tell` (tell) | 8 | Dip: head and shoulders down 2 px, snout turns down 1 px, near arm swings back 2 px |
| 8 K | `tell` | 20 | Hold: head 4 px lower than key 0 (skull top row 18), tusks levelled forward (far tusk tip at x 44 or more, rows 22 to 24), brow down 1 px, eye white 2 px, mouth gape 1 px wider; rump up 1 px; far trotter scraped back to x 9 to 13 |
| 9 | `gore` (attack) | 3 | Launch: whole body 5 px right, head still low, rear trotter dragging (x 13 to 17), front trotter planted |
| 10 K | `gore` | 6 | Impact: body 8 px right, head thrust up and forward 3 px (tusk tips at x 50 to 53, rows 15 to 22); mouth wide; hitbox active |
| 11 | `gore` | 14 | Recover: body 3 px right, head drooped 2 px, tongue out 2 px, both trotters planted |
| 12 K | `hurt` (hurt) | 12 | Snap back: body 2 px left, head up and back 2 px, eyes shut (no `w`, a `c` eye slit), squeal gape 2 px, ears flat |
| 13 | `down` (defeat) | 8 | Sag: knees buckle, body down 4 px, head lolls 2 px, eyes shut |
| 14 | `down` | 10 | Topple: body rotated backwards about 45 degrees (redrawn, not rotated at runtime), trotters leaving the floor |
| 15 K | `down` | hold | On its back: belly up (shirt or gown up), four trotters in the air, head on the floor at the right with the tongue out, eyes shut; opaque rows 24 to 39, width 44 px or less |

Tags: `idle` 0-2 loop, `scramble` 3-6 loop, `tell` 7-8 once, `gore` 9-11 once, `hurt` 12 once, `down` 13-15 hold.

### Tell-to-attack read (60 Hz)

- Tell start (frame 7) to the first hitbox tick (frame 10): 8 + 20 + 3 = **31 ticks (517 ms)**; from the hold start 23 ticks. A shade longer than the Scatterer (28) because Tusklings come in groups and the player must pick out which one is about to gore.
- The read, in order of strength: (1) the head drops 4 px, so the snout line falls below the shoulder hump and the silhouette turns from a hunched upright body into a low battering ram; (2) the tusks go level and stick out 2 px further; (3) the eye white appears (2 px); (4) the rear trotter scrapes back (a dust puff is a separate VFX).
- Gameplay note (not approved): stagger the tells of a group by at least 12 ticks so the reads do not stack.

## 5. State diagram

```text
idle <-> scramble -> tell -> gore -> idle
any (except down) -> hurt -> idle | down
down (hold)
```

`hurt` cancels `tell` and `gore` before frame 10. The Ball's break-apart spawns Tusklings straight into `scramble` or `idle` (the transition is a VFX work order).

## 6. Palette

The SWINE-01 family palette `tuskling-floor01-v1`, imported unchanged (`author_tusk01.PALETTE = author_swine01.PALETTE`): 19 keys, slot 20 free. No new colour: both variants are built from existing keys. A third costume must come from the same keys or claim slot 20 with a recorded reason.

Key 0 key use: flesh ramp `d s m l` with `h` only on the snout disc (7 px); `c` crevices, socket, mouth and the neck crease where the skull meets the hump; `t u v` tusks, teeth and shirt; `a b` bristle; `n o` jacket, sleeve and hooves; `r g` gums, tongue and bow tie; `w` the eye white (1 px). Counts on key 0: `K` 132, `n` 195, `s` 56, `m` 52, `d` 51, `l` 35, `o` 34, `u` 32, `c` 30, `v` 15, `r` 15, `a` 11, `h` 7, `t` 3, `b` 2, `g` 1, `w` 1.

### 6.1 Costume variants (rule)

`variant(cels, name)` in `scripts/author_tusk01.py`:

- `tux`: the drawn costume layer as is.
- `gown`: on the costume layer only, remap `n -> r`, `o -> g`, `u -> m`, `v -> l`, `t -> s` (black cloth to red sequin, sheen to glints, shirt to bare chest); every former `n` pixel with `(x + 2y) % 11 == 0` and `x < 20` (canvas) also turns `g` (scattered sequins on the lit side); the bow tie (`BOW`) turns `l` (bare throat); a 7 px pearl string (`PEARLS`, `v` beads and `t` links) drapes across the throat; a 6 px ear flower (`FLOWER`, `q` and `p`) sits on the far ear. Then the same orphan clean-up as key 0. The silhouette is the base body's.
- `BOW`, `PEARLS` and `FLOWER` are in design coordinates (canvas = design + (2, 4)) and anchored to key 0. For frames 1 to 15 the executor adds `ANCHORS[frame] = (throat_dx, throat_dy, ear_dx, ear_dy)`, the offset of the throat and the far ear from key 0, and `variant()` applies the accessories at that offset. On frame 15 (on its back) the pearls may be hidden; the flower stays on the head.

## 7. Key poses

### Frame 0, `idle` (hero key, drawn and frozen by the designer)

Built by `key0_design()` from shaded ellipses (light from the upper left, `author_swine01.LIGHT`), stamped features, a 4-neighbour `K` outline and the SWINE-01 orphan clean-up. Measured: opaque x 7 to 42, rows 10 to 39 (30 x 36 px), 0 orphans, 0 isolated.

| Part | Canvas pixels | Note |
| --- | --- | --- |
| Skull | ellipse centre (28, 18.5), radii 6.5 x 6 | domed; a `c` crease on its back edge, rows 15 to 22, splits it from the hump |
| Ears | near ear tip (29, 11), far ear (22-24, 12-14) | near ear pricked and folded forward with a `c` inner, far ear `d` |
| Snout | bridge centre (33, 18.5), disc centre (37.5, 18), radii 2 x 2.7 | disc `h` lit, `l` and `s` rim; nostril notches `K` at (39, 17) and (39, 19) |
| Eye | brow `l` (25-29, 15-16) over `c` (25-26, 16); socket `c` (27, 17), white `w` (28, 17), pupil `K` (29-30, 17); bag `d` (27-29, 18) | small and sunk; far eye `K` (32-33, 16) under an `l` brow |
| Snarl | lip `s` row 20 (x 31-36); gums `r` row 21 (x 32-35); fangs `v` row 22 (x 31-32, 34-35); lower teeth `u` row 23 (x 31-32); tongue `r` (35-36, 23-24) | the Ball's snarl at Tuskling size |
| Tusks | near (28-30, 19-23), tip (30, 19); far (37-40, 21-23), tip (41, 18) | `v` top, `u` body, `t` root; the far tusk hooks past the snout and is the forward-most point |
| Bristle | `a`/`b` along the hump and crown, rows 12 to 17 | coarse, uneven |
| Jacket | trunk below the collar line `y = 15 + 0.35 (x - 10)` (design), ragged hem rows 34 to 35, coat-tails x 8 to 13 rows 29 to 36 split at the vent | `n`, 1 px `o` sheen on the collar and back edge, `o` lapel line along the shirt |
| Shirt | V from the throat (x 28, row 24) widening to x 24 at row 31, ragged hem rows 31 to 32 | `u`, `v` where lit, a 3 px `t` stain; bare belly `s`/`d` below it |
| Bow tie | 5 x 3 `r` with a `g` knot, x 26 to 30, rows 25 to 27 | |
| Near arm | `n` sleeve from (21, 25) to (25, 30) with an `o` back edge, `v`/`u` cuff, flesh trotter-hand (26-29, 30-32) with an `n` hoof row 33 | |
| Legs | far shin (15-18, 33-36) and trotter (14-18, 37-38); near shin (24-28, 33-36) and trotter (23-29, 37-38) | `K` hoof clefts at x 16 and x 26; `o` 2 px toe sheen on the near trotter |

### Keys 3, 8, 10, 12, 15 (execution tier)

Draw them from the pose column of section 4 by moving the key-0 parts (head group, trunk group, arm, legs, costume), not by redrawing the head: the head, snarl and tusks are the family read and keep their pixel layout on every frame except where section 4 says the mouth or eye changes. Keep the outline and clean-up rules of key 0. The hurt and down eyes are a 2 px `c` slit in place of `w` + `K`.

## 8. In-between list

Frames 1, 2, 4, 6, 7, 9, 11, 13, 14: offsets and part moves from section 4. In-betweens must not introduce new costume shapes; the gown variant follows by `variant()` with the frame's `ANCHORS`.

## 9. Validators and readability rules

`python3 -m scripts.pixelart validate assets/source/characters/swine/tusklings/tusk_01.pixelart.json`: PASS (14 pass, 2 skipped) on the design-stage grid (key 0 only).

- V04 at most 20 colours (19); V05 isolated 0; V06 orphans 0; V07 contact row 39; V09 frozen key 0 (`tests/fixtures/tusk_01_keys.json`, layers `body` and `costume`); V10 layers disjoint.
- V13 `T1-size-idle-30px`: frame 0 top 10, bottom 39; `T1-idle-width` 32 to 38.
- V14 `T0-snout-lit` `h` at least 5; `T0-small-sunk-eye` `w` 1 to 2; `T0-flower-is-gown-only` `q` 0 on the tux grid.
- Rules for the executor to add: idle frames 0-2 top 9 to 11; tell hold (8) top at least 13 and right at least 44; impact (10) right at least 50; hurt (12) `w` 0; down hold (15) top at least 24, width at most 44, `w` 0; `w` at most 2 on every frame except 8 to 10 (at most 4); `unique_frames` on; V16 foot drift on `scramble`.
- `tests/test_art_tusk01.py`: key 0 equals the script and the frozen digest; canvas, pivot and disjoint layers; 30 x 36 px; at least 6 ivory pixels at x 38 or more (the far tusk); `h` at least 5, `r` at least 12, `w` at most 2, bristle at least 8, no `q` on the tux; palette equal to the SWINE-01 grid's (19 keys); the gown variant has 0 orphans, no `n`/`o` in the costume, at least 120 `r`, 1 to 6 `q`, at least 3 `v` pearls, and the same silhouette within 4 px.

### Acceptance targets

1. At 1x on lit Floor 1 next to Carl and the Ball, a Tuskling reads as one of the Ball's heads on legs: pink snarling snout, two ivory tusks, bristle, small sunk eye, at Carl's thigh-to-belt height.
2. Tux and gown tell apart at 1x by the torso colour block alone, with an identical silhouette.
3. Never cute: no round or big eyes (`w` at most 2), no smile, the head carried low; grotesque, not a mascot.
4. The tell reads in a group of four: the head drop and levelled tusks change the silhouette before the eye white is visible.
5. Every grounded frame touches row 39; the gore stays on the floor.
6. The palette is the SWINE-01 palette unchanged; the gown is derived by rule with 0 orphans.

## 10. Evidence

```text
python3 scripts/author_tusk01.py               # grid + .ase
python3 scripts/author_tusk01.py key0-scale    # docs/evidence/tusk-01/key0-scale.png (1x) and key0-scale-4x.png
```

`key0-scale.png`: Carl, the tux and the gown Tuskling (key 0, mirrored to face Carl) and the Ball of Swine idle key at 1x on lit Floor 1 (640 x 360); `key0-scale-4x.png`: the same scene at 4x. Needs the kit atlases (`scripts/export_assets.sh`).

## 11. Files

| File | Role |
| --- | --- |
| `scripts/author_tusk01.py` | deterministic authoring script (key 0, gown rule, scale evidence) |
| `assets/source/characters/swine/tusklings/tusk_01.grid.json` / `.ase` | pixel-grid source (layers `body`, `costume`) and Aseprite master |
| `assets/source/characters/swine/tusklings/tusk_01.pixelart.json` | asset config |
| `tests/fixtures/tusk_01_keys.json` | frozen key 0 |
| `tests/test_art_tusk01.py` | design-stage checks |
| `assets/manifests/tusk-01.json` | manifest with ticks and boxes (execution tier) |

## 12. Rights and provenance

Fan-concept art under the owner-reported `ART-PUBLICATION-ALL-V1` scope (`docs/04-production/rights-and-release.md`). No tracing of the concept, no image-model raster output, no third-party assets.

### Rights/Provenance

- Concept: owner-approved reference; crops for reference only.
- Spec and hero key 0 (with the gown rule): Claude Opus. Execution (frames 1 to 15, manifest, review pack): pending. Independent review: pending (silhouette gate on key 0 first).
