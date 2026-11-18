# GOB-02-DEATH: goblin death poses and death-parts sheet, design spec

**Status:** executor pass complete (section 9a). Spec, hero keys (poses 0, 2, 3, 5; parts 0, 1, 2) frozen; the two in-betweens (poses 1 and 4), the 21 remaining parts (3 to 23), the manifests with anchors and spawn metadata, the export, the validators and the evidence are drawn and run. Independent review pending. Nothing here is wired into Bevy. The owner approved intensity preset 4 as the default (2026-11-18, recorded in the [death-system proposal](../03-tech/monster-death-system-proposal.md#b-suggested-data-contract-illustrative-rust-design-not-compile-ready-abi)). Gameplay acceptance of any finish is still a separate owner decision.

Scope: one work order that merges the plan's GOB-02-CRUSH (falling block D-04) with the PR C parts sheet of the [death-system proposal](../03-tech/monster-death-system-proposal.md#f-implementation-pr-sequence):

1. Four contact and aftermath poses on the GOB-01 frame and pivot: `crush_contact`, `crushed_remains`, `decap_contact`, `headless_collapse`.
2. The goblin A death-parts sheet: head, torso chunks, arms, legs, the pineapple club, gibs and blood decals, with anchors and hit-origin metadata so Bevy can launch, spin, settle, stain and fade them.

Sources: [`goblin_a_death_h36.grid.json`](../../assets/source/characters/goblin/death/goblin_a_death_h36.grid.json) and [`goblin_a_death_parts.grid.json`](../../assets/source/characters/goblin/death/goblin_a_death_parts.grid.json), written by [`scripts/author_gob02_death.py`](../../scripts/author_gob02_death.py). Evidence: [docs/evidence/gob-02-death/](../evidence/gob-02-death/README.md).

## 1. Canon and tone

- The goblin is the approved [GOB-01](gob-01-goblin-spec.md) common goblin: green, about 4 ft, leathery rags, long pointy ears, sharp teeth, a **pineapple on a stick**. Death art must not redesign it. Every pose and part keeps at least one 1x identity cue: the long ear, the hooked nose, the teeth or the pineapple.
- [Antihero tone](../01-design/antihero-tone.md): grimy, darkly comic, book-canon brutal. The deaths are nasty and a little ridiculous (bulging eyes, a sprung jaw, a lolling tongue, a head that lands with X eyes). Gore is pixel-honest and readable: a cut shows meat and a bone end, blood is a few clean shapes. Never cute, and never torture-porn lingering: no extra organs, no twitching hold, no close-up. The aftermath frames are holds; the motion is in the first 10 ticks.
- The [Death Lab](../../tools/art-references/death-lab/README.md) gave the attitude of intensity 4 only (a hard impact read, a decisive jet, a spreading pool, more pieces). None of its drawn cutouts, colours or timings were traced or sampled.
- Fan-concept layer: the goblin and its pineapple are the replaceable fan-concept layer. The data contract (section 7) is IP-neutral (`head`, `torso_upper`, `weapon` and so on).

**Not approved by this spec:** which causes may decapitate or explode a goblin (domain eligibility, PR D), the falling-block damage to players, any fragment or particle budget (set after profiling), sound, camera shake, and the B variant overlay (section 13).

**Living horizon:** this is art for already-planned finishes (D-04 crush, PR D blade and earned explosive). It adds no move, unlock or ability, so it does not change the [character progression horizon](../01-design/character-progression-horizon.md).

## 2. Frame, pivot, layers and anchors

**Poses atlas:** frames of **64 x 56, pivot (28, 55)**, the GOB-01 contract. All four tags are grounded: the lowest opaque row is 55. Authored facing right. The runtime mirrors around the pivot column (x' = 56 - x), anchors included.

**Parts atlas:** **32 x 32 cells, one part per frame**, cell pivot (16, 16). The cell pivot is the part's rotation centre and centre of mass. Each part's own anchors are in the manifest (section 7). A mirrored part mirrors around x 16 (x' = 32 - x).

**Layers (both atlases), bottom to top:** `body`, `weapon`, `gore`.

- `body` and `weapon` together are always a **closed, outlined, bloodless sprite**. Every coloured pixel touches only opaque pixels or the canvas edge (tested). A cut surface on `body` is a plain skin or rag cap.
- `gore` holds every wound, blood and bone-end pixel. It may sit on top of `body` pixels (a cut face) or outside them (a jet, a puddle).
- The blood keys `B` and `P` exist on `gore` only (tested). `R`, `o` and `w` appear on `body` only as the GOB-01 tongue, mouth, teeth and eye whites.

This split is what makes the presets data-only (section 5).

**Pose anchors (frame coordinates, facing right):**

| Pose (frame) | Anchor | xy | Use |
| --- | --- | --- | --- |
| `crush_contact` (0) | `hit_origin` | (28, 33) | the stone's underside on the goblin; radial origin for squeezed-out pieces |
| | `hand_near` | (55, 48) | club launch point (the club has left the hand on this tick) |
| | `mouth` | (49, 43) | blood emitter |
| | `squeeze_back`, `squeeze_front` | (6, 54), (46, 54) | floor squeeze-out points (splats, pool start) |
| `crushed_remains` (2) | `pool_origin` | (29, 55) | pool decal centre, on the walking line (after the facing fix) |
| `decap_contact` (3) | `neck` | (23, 31) | head part spawn: the head part's `neck` anchor goes here |
| | `hit_origin` | (24, 31) | the blade line crosses the neck on row 31 |
| | `jet_base` | (23, 30) | blood emitter for the jet after frame 3 |
| | `hand_near` | (12, 23) | the flung fist (the club stays in the pose) |
| `headless_collapse` (5) | `neck` | (17, 50) | stump; trickle emitter |
| | `pool_origin` | (12, 55) | pool decal centre |

Frames 1 and 4 (in-betweens) carry the anchors of their key moved by the same offsets as the pixels (section 9).

## 3. Palette `goblin-death-floor01-v0` (20 of 20 per atlas)

The 19 GOB-01 keys minus `e`, plus `B` and one new key `P`. Same hex values as goblin-floor01-v0 (tested). There is no `e`: a death atlas has no living glare. Shocked or dead eyes are `w` and `K`.

| Key | Name | Hex | Role here |
| --- | --- | --- | --- |
| K | outline_plum | `#1b1124` | outline; X eyes, pupils |
| a / s / m / l | skin deep / shade / mid / light | `#263a26` `#46682e` `#7a9e36` `#b2cc56` | skin; the far limbs one step darker |
| r / t / u | rag dark / mid / light | `#2c1e28` `#4c3436` `#73524a` | vest, skirt |
| w | tooth_white (accent) | `#efe4c4` | teeth, eye whites; **bone ends on `gore`** (the spine dot of a cut neck) |
| o | mouth_dark | `#5c1426` | mouth interior; on `gore` the deepest blood (old pool rim, a stain fading) |
| R | red_mid | `#b8322e` | tongue; on `gore` **fresh blood and meat** |
| x / y | wood dark / mid | `#4a2c1c` `#8c5c32` | stick, rope belt |
| F / G / H | pineapple dark / mid / light | `#7a3c12` `#d08826` `#f4c84a` | fruit |
| V / v | leaf dark / mid | `#1d4a3c` `#3e8a5c` | crown |
| B | blood_dark | `#6e1a22` | **gore only.** The GOB-01 B-variant bandana shadow, same hex, reused as clotted blood and the shadow edge of a jet |
| P | blood_wet (accent) | `#e2434b` | **gore only, new.** Wet highlight: 1 to 3 pixels per blood shape, the jet core |

Blood ramp, dark to light: `o` `B` `R` `P`. Goblin blood is dark crimson, not green (the #79 profile sketch's `goblin_green_dark_crimson`: green flesh, crimson blood). `P` is cooler and pinker than the orange brick, so a splat does not melt into the Floor 1 wall under the warm torch light. A small pool reads at 1x as a dark stain with one wet glint.

## 4. Pose list (poses atlas, 6 frames)

| # | Tag | Frame | Ticks (60 Hz) | Playback | Author |
| --- | --- | --- | --- | --- | --- |
| 0 | `crush_contact` | stone contact: rammed down to 23 px | 3 | once | **key** |
| 1 | `crush_contact` | flatten (placeholder = copy of 0) | 3 | once | executor |
| 2 | `crushed_remains` | broken heap at the stone's foot | hold | hold | **key** |
| 3 | `decap_contact` | blade tick: headless recoil, jet | 4 | once | **key** |
| 4 | `headless_collapse` | buckle (placeholder = copy of 3) | 6 | once | executor |
| 5 | `headless_collapse` | lying headless, puddle | hold | hold | **key** |

`crush_contact` → `crushed_remains` and `decap_contact` → `headless_collapse` are played in sequence by the renderer. Holds are state-driven (preview duration 100 ms).

## 5. Intensity presets 1 to 4: derivation by data only

Presets are presentation-only. They never touch the domain, eligibility, collision or the replay hash. Each preset is the same sources plus four data switches: **gore-layer visibility**, a **gore palette remap**, a **part-class filter** and a **count scale**. Nothing is redrawn per preset.

| Switch | 4 (default, full) | 3 | 2 (reduced) | 1 (off) |
| --- | --- | --- | --- | --- |
| `gore` layer on poses and parts | shown | shown | shown, remapped `gore_muted` | **hidden** |
| `gore_muted` remap (gore layer only) | none | none | `R`→`B`, `P`→`B` | n/a |
| part classes spawned | all (`gib_bone` at 4 only) | `wound`, `clean`, `gib` (no `gib_bone`), `blood` | `wound` (gore muted), `clean`, `blood` (drops only) | `clean` only, and `head` in a decapitation |
| count scale (gibs, drops) | 1.0 | 0.5 | 0.25 (drops only, no gibs) | 0 |
| decals | splat s/m, pool 1 to 3, wall spatter a and b, smear | splat s/m, pool 1 to 2 | splat s only (muted) | none |
| explosive finish | all body parts | head, torso_upper, club, 2 limbs | head, torso_upper, club | fallback: GOB-01 `knockback` → `corpse` + club + `gib_rag` + dust |

- **Preset 1** keeps the equivalent pose, silhouette and feedback: the crush and decapitation poses still play (hidden gore leaves clean caps, see the [preset 4 vs preset 1 sheet](../evidence/gob-02-death/keys_x4_preset4_vs_preset1.png)), the head still flies in a decapitation (the cause stays readable), and the club still clatters. It draws no blood sprite.
- **Preset 2** is the #79 `reduced`: less blood, darker blood, fewer pieces.
- **Preset 3 vs 4 must read clearly different** (owner ask). Only 4 has the wall spatters, the third (36 to 40 px) pool stage, the head-roll smear, bone gibs, the full jet emission after frame 3, the doubled crush drops and the full explosion breakup.

## 6. Key poses (as authored)

- **0 `crush_contact`.** The landing tick of the block. The goblin is rammed down to 23 px (rows 33 to 55; GOB-01 standing is 36). The top is a straight 42 px `K` line on row 33: the stone's underside. Nothing is drawn above it. The head is shoved forward and flattened. The long ear is pressed flat back over the hump, and its tip pokes out behind (the 1x cue). The near eye bulges (3 x 3 `w`, a pinpoint `K` pupil). The jaw is sprung wide (top teeth row, dark mouth) and the tongue lolls out. Both arms are flung out flat, the near hand open with three fingers. The club is **not** in the frame: it leaves as the `club` part on this tick. The knees splay. Gore (preset 4 impact, 55 px, 5 `P`): a solid 2 px gush out of the mouth (`R` body, `P` core, `B` underside) breaking into droplet pairs, and a low squeeze-out burst fanning outward at each squeeze anchor over rows 53 to 55 (`B` floor edge, `R` body, a `P` glint, one detached droplet pair). With the stone composited on row 33 it reads as pressed (see the scale sheet's top view); alone it can read as a lunge, so evidence must always show it under the stone. New literal rows (blocked in with polygon and capsule shapes, then hand-finished as rows).
- **2 `crushed_remains`.** The hold after the landing. **Faces right, like keys 0 and 1** (review fix D1: the heap is built head-left from the GOB-01 corpse, then mirrored with x' = 59 - x; a pure pivot mirror, x' = 56 - x, would clip the snapped shin, so the heap sits 3 px forward). The face stays right of the pivot through 0 → 1 → 2 (tested), so the hold never flips end to end. A broken, low heap (rows 40 to 55): **not** a paper cut-out. The head is GOB-01 frame 25's corpse head, mirrored, pixel for pixel down to row 50 (X eyes, the ear standing up, the tongue). The body is new: a pressed rag mass under a top outline on row 48 that is notched once and bumped once (torn rags, not a plank), a `u` top-light row broken by `t`, two `r` folds, the near hand on the floor, one shin snapped up with the sole showing at the back end. Gore: one continuous pressed-out seep along row 55 under the whole heap (40 px, `B` ends, `R` body, 2 `P`), plus the neck and hem seeping on row 54. It sits at the stone's foot, drawn in the actors layer as usual (in front of `loose_block_landed`). The low height lets the stone dominate.
- **3 `decap_contact`.** The blade's tick. This is GOB-01 frame 16 (the `hurt` recoil: torso thrown back, arm flung up, stick trailing low behind), with the head, ear and jaw removed above the collar. The body from row 38 down and the weapon layer are unchanged (tested). A new short neck stump stands on the collar. On `gore`: the cut face on rows 30 and 31 (meat `R`, wet `P`, the spine end `ww` at the back), and **the jet**: a 2 to 6 px column straight up out of the stump (rows 12 to 31). It has a dark `B` edge on the shadow side and a `P` core, leans slightly back the way the head flew, and breaks into gobbets near the top. The jet is the one frame of excess: 4 ticks, then the spawner takes over.
- **5 `headless_collapse` (hold).** GOB-01 frame 25 (`corpse`: on the back, knees up, the pineapple rolled off the stick) with the head removed (everything right of x 28 unchanged, tested). The new neck stump points to the camera-left. On `gore`: the cut face (a spine dot `w`), a run of blood down to the floor and a 14 px puddle on row 55. The spreading pool is a decal (section 7), not this frame.

**Part keys (parts atlas):**

- **0 `head`.** GOB-01 frame 0's head and ear unchanged, except the yellow glare becomes white eyes (`e` → `w`, the `K` pupils kept, as in the GOB-01 hurt keys): a shocked head. Neck cut at the back-bottom of the skull, on `gore` (meat, a wet glint, the spine end). Cell (1, 9) to (27, 23): 27 x 15.
- **1 `torso_upper`.** GOB-01 frame 0's leather vest hump and the chest strip, with the near arm and the head removed. On `gore`: the neck stump on top (a bone end) and the waist torn open at the bottom (spine ends `ww`, meat, two wet glints). 16 x 15.
- **2 `club`.** GOB-01 frame 0's pineapple (crown and fruit, unchanged, tested) on a **new straight 2 px shaft** at the idle angle: `y` on the upper edge, `x` below, the butt end capped. The idle shaft is half hidden by the fist and kinks under the fruit, which would read as broken on a loose club. No gore in any preset. 19 x 25.

## 7. Parts list and metadata schema

Keys 0 to 2 are frozen. The executor appends 3 to 23 in this order (indices are part of the contract). Classes: `wound` (a body piece with a cut face on `gore`), `clean` (no gore), `gib` (small flesh/bone), `blood` (gore-only sprites and decals).

| # | id | Class | Size (about) | Source and drawing rule | Anchors (cell) | Settle |
| --- | --- | --- | --- | --- | --- | --- |
| 0 | `head` | wound | 27 x 15 | key | `neck` (11, 20) | roll |
| 1 | `torso_upper` | wound | 16 x 15 | key | `neck` (17, 10), `waist` (13, 22) | flop |
| 2 | `club` | clean | 19 x 25 | key | `grip` (8, 27), `fruit` (21, 14) | flop, clatter |
| 3 | `head_dead` | wound | as 0 | key 0 with GOB-01 frame 25's 3 x 3 `K` X over each eye and frame 25's 2 x 3 `R` tongue out of the mouth corner, outlined | as 0 | (swap target) |
| 4 | `torso_lower` | wound | 18 x 7 | GOB-01 frame 0 rows 42 to 46, x 19 to 36 (rope belt and rag skirt; the stick's belt pixels kept); waist tear on top as `torso_upper` row 22 mirrored vertically; two hip stumps (2 x 1 `R` with `w`) under the skirt | `waist` top centre, `hip_near`, `hip_far` | flop |
| 5 | `arm_near` | wound | 9 x 9 | GOB-01 frame 0 near arm and fist, rows 34 to 41, x 33 to 41, the stick pixels removed and the fist closed with `K`; shoulder cut on top (gore) | `shoulder` | spin, flop |
| 6 | `arm_far` | wound | 9 x 9 | part 5 with the skin ramp one step darker (`l`→`m`, `m`→`s`, `s`→`a`) | as 5 | as 5 |
| 7 | `leg_near` | wound | 10 x 9 | GOB-01 frame 0 rows 47 to 55, x 28 to 38; hip cut on top | `hip`, `foot` | flop |
| 8 | `leg_far` | wound | 7 x 9 | GOB-01 frame 0 rows 47 to 55, x 19 to 26; hip cut on top | `hip`, `foot` | flop |
| 9 | `club_stick` | clean | 15 x 11 | part 2's shaft alone; the top end splintered (two 1 px `y` teeth with `K`) | `grip`, `break` | clatter |
| 10 | `pineapple` | clean | 13 x 9 | GOB-01 frame 25's rolled fruit (weapon layer rows 47 to 55, x 50 to 62) unchanged | centre | roll |
| 11 | `ear` | wound | 7 x 5 | GOB-01 frame 0 ear tip (rows 20 to 24, x 18 to 24), torn base on `gore` | `base` | flutter, flop |
| 12 | `gib_flesh_a` | gib | 5 x 4 | `l`/`m`/`s` skin chunk with `K` outline; 2 px meat side on `gore` | centre | bounce |
| 13 | `gib_flesh_b` | gib | 4 x 3 | `u`/`t` rag over `m` skin; meat side on `gore` | centre | bounce |
| 14 | `gib_bone` | gib | 4 x 2 | `w` bone with `K` outline (body layer); spawned at preset 4 only (section 5) | centre | clatter |
| 15 | `gib_rag` | clean | 5 x 3 | torn `t`/`u`/`r` scrap with `K` | centre | flutter |
| 16 | `drop_blood` | blood | 2 x 2 | gore only: `R P` / `B R` | centre | dies on contact → `decal_splat_s` (presets 3, 4) |
| 17 | `decal_splat_s` | blood | 6 x 2 | gore only, floor splat: `B` rim, `R` body, 1 `P` | centre-bottom | static |
| 18 | `decal_splat_m` | blood | 12 x 3 | gore only, splat with two 2 px satellites | centre-bottom | static |
| 19 to 21 | `decal_pool_1..3` | blood | 12 x 2, 20 x 3, **38 x 4** (review ruling: flat floor decals are exempt from the width cap; stage 3 widened from the executor's 28) | gore only, flat floor pools: `o` rim, `B` body, `R` centre, 1 or 2 `P`; each stage contains the previous one (growth by frame swap) | centre-bottom | static |
| 22 | `decal_wall_spatter` | blood | 22 x 18 | gore only, back-wall spatter: 5 to 7 radial streaks from the lower-left, **mostly `B` with `R` only near the origin** (`R`/`P` lose contrast on lit brick), 2 or 3 streaks ending in 1 to 2 px blobs, irregular lengths (not a comb), 3 drips (`B` 1 px wide, 3 to 6 px long), 2 `P` near the origin | origin (lower-left) | static |
| 24 | `decal_wall_spatter_b` | blood | 20 x 22 | second variant, so repeats do not stamp: same keys and rules as 22, but a steeper fan (rays between 50 and 110° from the origin, which sits bottom-centre), one long streak, 4 drips; appended after 23 so indices 0 to 23 stay stable | origin (bottom-centre) | static |
| 23 | `decal_smear` | blood | 18 x 2 | gore only, a skid smear, `B` fading to `o` at the tail | start (left) | static |

**Manifest metadata (per part).** The manifest `death_parts` section lists one object per frame:

```json
{
  "id": "head", "frame": 0, "cell": [32, 32], "pivot": [16, 16],
  "anchors": {"neck": [11, 20]},
  "frame0_origin": [33, 27],
  "class": "wound", "material": "goblin_flesh",
  "presets": [1, 2, 3, 4], "gore_layer": true,
  "mass": "medium", "spin": "fast", "settle": "roll", "bounce": 0.35,
  "swap_on_first_contact": "head_dead",
  "stain": "decal_splat_s", "trail": "decal_smear",
  "ttl": "keep"
}
```

- `frame0_origin` is where the part's cell pivot sits on GOB-01 A frame 0 (head (33, 27), torso_upper (26, 36), club (44, 34); the executor measures the others). It lets the spawner start an explosion breakup from the live pose. For another live frame, the spawner uses the pose's matching anchor or falls back to `frame0_origin` relative to the pivot.
- `presets` is the class filter of section 5. `gore_layer` says whether the part has `gore` pixels (hidden or remapped by preset).
- `mass`: `light | medium | heavy` (hint for launch speed scaling); `spin`: `none | slow | fast`; `settle`: `roll | flop | clatter | flutter | bounce | static`; `ttl`: `keep` (until room reset or budget eviction, oldest first) `| long` (about 6 s then a 30-tick fade) `| short` (about 2 s then a 12-tick fade). Times are hints until profiling.
- **Pose metadata:** the poses manifest carries `death_poses` with the section 2 anchors per frame (`hit_origin`, `neck`, `hand_near`, `mouth`, `jet_base`, `squeeze_*`, `pool_origin`).

**Never hide Carl (readability rules for the renderer):**

- Draw order: launched parts on a debris layer **behind player actors** (just under actor layer 7, above props 6), floor decals under every actor on the walking line, `decal_wall_spatter` on `backwall_decal` (3) only. Nothing is ever drawn over a player sprite.
- No launched part is larger than 28 x 28 (tested). **Flat floor decals** (`decal_splat_*`, `decal_pool_*`, `decal_smear`) are exempt from the width cap, up to 40 px wide, but keep a 4 px height cap and always draw under every actor, so they can never hide anyone. At rest a part lies on the walking line, never stacked higher than 12 px.
- Blood never lands on actors and never on foreground layers. No full-screen flash or red tint is part of this art.

## 8. Timing hints for the cosmetic spawner (60 Hz, preset 4, scale by section 5)

These are presentation hints for PR C/D, measured from the authoritative `EnemyDefeated` tick. Speeds are in px/s at 1x; gravity is the hero's 1050 px/s². "Away" means away from `hit_origin`, or from the block centre for a crush. Counts at presets 3/2/1 follow the section 5 scale.

**Crush (D-04, the landing tick L):**

| Tick | Event |
| --- | --- |
| L | pose 0 (3 ticks), ENV-02 block impact dust; `club` from `hand_near`: away 90 to 150, up 120 to 180, spin 540 to 900 °/s, 1 or 2 bounces (0.3), settle flat |
| L | **16 to 20** `drop_blood` from `mouth` (6 to 8) and the two squeeze points (5 to 6 each), low angles (10 to 35° above horizontal), 80 to 160; each drop that lands becomes `decal_splat_s` (at most 8 per death) |
| L | 2 `gib_flesh_a/b` + 1 `gib_bone` squeezed out sideways from the squeeze points, 60 to 120, low |
| L+3 | pose 1 (3 ticks) |
| L+4 | `decal_splat_m` at both squeeze points; `decal_pool_1` at `pool_origin` |
| L+8 | `decal_wall_spatter` (a or b, seeded, randomly mirrored) on the back wall at block-base height when a wall face is within 48 px: the squeeze blows sideways |
| L+6 | pose 2, hold |
| L+20 / L+60 | pool stage 2 / stage 3 (38 px), earlier than the decapitation pool: the crush empties the body at once |

**Decapitation (PR D, the blade tick D):**

| Tick | Event |
| --- | --- |
| D | pose 3 (4 ticks); `head` at the pose's `neck` (head anchor `neck` on it), start rotation -20° (chin up, matching frame 16's thrown-back head), velocity away 60 to 100 and up 220 to 280, spin 720 °/s backward, 2 bounces (0.35), then rolls 4 to 12 px; on first floor contact swap to `head_dead`; each bounce `decal_splat_s`; the roll leaves `decal_smear` |
| D+4 | pose 4 (6 ticks); `drop_blood` from `jet_base`, 2 every 3 ticks for 30 ticks, rising 150 to 220, spread ±12° |
| D+4 | `decal_wall_spatter` (a or b, seeded, randomly mirrored; never the same variant and facing twice in a row on screen) on the back wall behind the jet when a wall face is within 48 px |
| D+10 | pose 5, hold; trickle: 1 `drop_blood` every 8 ticks from `neck` for 40 ticks |
| D+14 / D+40 / D+100 | pool stages 1 / 2 / 3 at the pose 5 `pool_origin` |

**Explosive breakup (PR D, earned explosives only):** from `impact_position`, every part in the preset's list at its `frame0_origin`. Radial speed 160 to 300 (heavy parts −30 %), up-bias 60 to 120, spin by class. Then 8 gibs, 24 drops, both wall-spatter variants (one each side of `impact_position`, when walls are in range) and pools under the two heaviest pieces once they settle. Settled pieces lie irregularly (seeded rotations in 90° steps, piled and overlapping), never in an evenly spaced row. The club splits into `club_stick` + `pineapple` at preset 4 only.

Seeded by `hash(run_seed, event_id)` outside the domain, as the proposal requires. No visual element feeds back into combat.

## 9. Executor list (no design decisions needed)

General rules: copy from the keys and from GOB-01 A as each row says. Translate whole parts. Do not resample or rotate except by 90°. Keep the palette and the layer rule (blood keys on `gore` only, `body`+`weapon` closed). Add every new frame to `scripts/author_gob02_death.py` as rules or literal rows. Keys must stay byte-identical (the fixtures).

1. **Pose frame 1 (`crush_contact` flatten).** Copy frame 0. Move rows 33 to 45 down 3 rows (onto rows 36 to 48), replacing rows 46 to 48; rows 49 to 55 (skirt hem and legs) stay; close the seams with `K`. The near eye's `K` pupil becomes `w` (popped). Lengthen the mouth gush by 4 px with one more droplet pair. Extend frame 0's two squeeze bursts 2 to 3 px further outward, and add one more droplet to each. Anchors: `hit_origin` (28, 36), `mouth` (49, 46).
2. **Pose frame 4 (`headless_collapse` buckle).** Copy frame 3. Replace the legs (rows 47 to 55) with GOB-01 A frame 24's kneel legs (rows 47 to 55). Move the torso, neck and arm (rows 22 to 46) down 4 rows and 2 px left. Lower the fist to about (14, 30) by moving the forearm block. Gore: the jet becomes an 8 px column bent back (left) with two gobbets; the stump's cut face is unchanged. Anchors: `neck` (21, 35), `jet_base` (21, 34).
3. **Parts 3 to 23** per the section 7 table, in that order. The design spec gives every source and rule.
4. **Manifests** `assets/manifests/goblin_a_death_h36.json` and `goblin_a_death_parts.json` in the GOB-01/ENG-01 shape, plus `death_poses` and `death_parts` (section 7). Measure and record `frame0_origin` for parts 3 to 11.
5. **Configs and fixtures:** extend `PART_TAGS`/the parts config with the new tags. Set `unique_frames: true` in the poses config once frames 1 and 4 are drawn. Do not re-freeze the keys.
6. **Export** both atlases with the existing pipeline (`scripts/export_assets.sh` pattern used by GOB-01).
7. **Evidence** (`python3 -m scripts.pixelart evidence` on both configs, plus): GIFs of the four tags at x1 and x3; a preset strip (presets 4, 3, 2 and 1 of the same crush and decapitation moment, using the section 5 switches); a static spawner storyboard (L, L+3, L+6, L+30 and D, D+4, D+10, D+40 as stills on the lit Floor 1 with the section 8 placements; a still sheet, not a Bevy capture); a parts contact sheet at x4 with anchors marked; SHA256SUMS.
8. **Tests:** extend `tests/test_art_gob02_death.py` to the new frames (gore-only keys, closed body at gore-off, part sizes, `head_dead` = `head` outside the eyes and tongue, `arm_far` = ramp-shifted `arm_near`).

9. **Second executor pass (blind review, 2026-11-19), no design decisions needed:**
   1. Re-check frame 1 as the bridge into the mirrored key 2 (the face stays right of the pivot in 0, 1 and 2; tested). No redraw is expected; if a seam shows, fix it in frame 1 only.
   2. Wall spatter polish (part 22): streak bodies mostly `B` (with `o` darkening at the tips), `R` only within about 6 px of the origin, 2 or 3 streaks ending in 1 to 2 px blobs, irregular lengths. Add part 24 `decal_wall_spatter_b` per section 7.
   3. Widen `decal_pool_3` to 38 x 4 (still nested over stage 2, same centre and bottom row). Update the part-size test: the 28 px cap applies to launched parts only; flat floor decals are capped at 40 x 4.
   4. Lift `torso_lower`: a broken `u` top-light row under its top outline, broken by `t` in 2 places, as on the `crushed_remains` rag mass.
   5. Evidence: an irregular, seeded explosion settle scene (piled, overlapping, rotated in 90° steps) instead of the even row; one composite with Carl standing inside the debris field and the pools (proves "never hide Carl"); add the `crushed_remains` hold in front of `loose_block_landed` to `scenes_1x.png`; one GIF of the crush sequence (0 → 1 → 2 → pool stages 1 to 3 at the section 8 ticks) at x1 and x3.
   6. Manifests: `gib_bone` presets `[4]`; the decal width exemption noted per part (`floor_decal: true`); the new crush drop counts and pool ticks if the manifest carries spawner hints.

Design problems found while executing go back to the design stage. Do not re-decide them.

### 9a. Executor pass: result and decisions

Run `python3 scripts/author_gob02_death.py keys` (sources and `.ase`) and `manifests` (both manifests), then `bash scripts/export_assets.sh` (atlases under `generated/art`, git-ignored), then `python3 scripts/gob02_death_evidence.py` for the evidence stills. Validators: both configs PASS (poses 14 pass, parts 13 pass, the rest not configured); the pose config now sets `unique_frames: true`.

- **Pose 1 (flatten).** Frame 0 with rows 33 to 45 moved down 3 rows as specified. Rows 46 to 48 are replaced except for the open hand (x 50 and up), which stays so the three fingers survive. Popped eye, gush lengthened 4 px with a droplet pair, squeeze bursts pushed out (the back burst can only gain 1 px: it already sits at the canvas edge, so it gains droplets instead). Anchors `hit_origin` (28, 36), `mouth` (49, 46); `hand_near` and `squeeze_*` do not move.
- **Pose 4 (buckle).** Frame 3 with the torso, neck and arm moved down 4 and left 2, frame 24's kneeling legs (rows 50 and below, moved 5 px left so the thigh meets the lowered skirt; its skirt rows 47 to 49 are not used because the moved skirt covers them), the forearm block (and the stick and fruit it holds) lowered so the fist sits at (14, 30), and the jet bent back into an 8 px column with two gobbets. Anchors `neck` (21, 35), `jet_base` (21, 34), `hit_origin` (22, 35).
- **Parts 3 to 23.** Rule parts are GOB-01 A rectangles translated whole (`scripts/author_gob02_death.py`: `ARM_RECT`, `LEG_*_RECT`, `EAR_RECT`, `LOWER_RECT`, the frame 25 fruit); every cut surface is the deterministic `cut_face` rule on `gore` (meat `R` over the outline line and the skin line behind it, `B` ends, a bone dot `w`, a wet glint `P`, a short drip); `arm_far` is `arm_near` with the skin ramp one step darker; `head_dead` is part 0 with a 3 x 3 `K` X in an eye box over each eye (far box x 16 to 18, near box x 20 to 22, a gap column between them) and a 2 x 3 `R` tongue out of the mouth corner; gibs, drop, splats and the smear are literal bitmaps; the three pools are nested by construction (shared bottom row and centre, each stage a superset of the last); the wall spatter is seven rays from the lower-left origin plus three tails hanging from ray tips.
- **Anchors and metadata** are measured from the drawn pixels (`part_anchors`) and written with the spawn metadata into `assets/manifests/goblin_a_death_h36.json` (`death_poses`) and `goblin_a_death_parts.json` (`death_parts`); `frame0_origin` is recorded for parts 3 to 11 (the fruit's centre on frame 0 for `pineapple`, whose pixels come from frame 25). `presets` follows section 5, with the deviations below.
- **Deviations (no redesign).** `decal_pool_3` is 28 px wide (spec 30) because every part obeys the 28 px rule; `gib_bone` is preset 4 only (section 5 wins over the section 7 note, since 3 and 4 must read differently). *Both settled by the 2026-11-19 review rulings: `gib_bone` preset 4 only is now the section 5 wording; floor decals are exempt from the width cap and `decal_pool_3` goes to 38 px (item 9.3).*

## 10. Acceptance targets and validators

Automated (`python3 -m scripts.pixelart validate` on both configs, plus `tests/test_art_gob02_death.py`):

- **D1 Crush contact.** Top row 33, flat `K` line of at least 36 px, nothing above it; no weapon pixel (the club has left); at least 8 `w` (the bulging eye and teeth); `gore` at least 40 px with at least 3 `P` (preset 4 clearly bloodier than preset 1).
- **D2 Remains.** Top row 40 or lower and at least 14 px tall (a heap, not a sheet); faces right like keys 0 and 1 (eye whites and teeth right of the pivot in frames 0, 1 and 2); GOB-01 corpse head reused pixel for pixel, mirrored; `gore` at least 40 px with a continuous run of at least 16 px on row 55.
- **D3 Decap.** At least 40 `R` (the jet); GOB-01 frame 16's body from row 38 and its weapon layer unchanged; no `w`/`o` on `body` (no head).
- **D5 Headless.** Top row 38 or lower, width at most 60; GOB-01 frame 25 unchanged right of x 28 and its weapon layer unchanged.
- **Layer rule.** `B`/`P` on `gore` only. With `gore` hidden: 0 isolated pixels, fewer than 10 orphans per frame, every coloured pixel closed.
- **Parts.** Launched parts at most 28 x 28 and flat floor decals at most 40 x 4, inside the cell with a 1 px margin; the club keeps the GOB-01 fruit and crown; the club has no gore.
- **Palette.** At most 20 colours per atlas; only `P` is new; every other key has its goblin-floor01-v0 hex.
- **Frozen keys.** Poses 0, 2, 3, 5 and parts 0, 1, 2 (`tests/fixtures/gob-02-death_keys.json`, `gob-02-death-parts_keys.json`).

Review (human eye, at 1x in the [scale sheet](../evidence/gob-02-death/silhouette_scale_1x.png)):

- Every pose reads as *that goblin* at 1x next to Carl (52 px) and Donut (40 px) on Floor 1 brick: ear, nose, teeth or pineapple visible.
- The crush reads as **pressed by something heavy**, not as lying down. The beheading reads as **beheaded**, with the jet the brightest red on screen for 4 ticks.
- Preset 1 (gore hidden) reads as complete, clean sprites with the same silhouettes.
- No part or decal reads as a Carl or Donut colour shape. Blood never sits on a player.

## 11. Evidence (design stage)

- [`silhouette_scale_1x.png`](../evidence/gob-02-death/silhouette_scale_1x.png): two lit Floor 1 views at 640 x 360, stacked. Top: Donut, Carl, then `crush_contact` under the ENV-02 `loose_block_falling` (underside on frame row 33), `crushed_remains` in front of the foot of `loose_block_landed`, the two decapitation keys and the three part keys settled on the walking line. Bottom: the uncomposited lineup with GOB-01 A idle for scale. [`silhouette_scale_4x.png`](../evidence/gob-02-death/silhouette_scale_4x.png) holds the walking-line bands of both views at 4x.
- [`keys_x4_preset4_vs_preset1.png`](../evidence/gob-02-death/keys_x4_preset4_vs_preset1.png): every key at 4x, preset 4 (as authored) above preset 1 (the `gore` layer hidden), and a bottom strip of every key at 1x (preset 4, then preset 1).

### Executor evidence

All in [docs/evidence/gob-02-death/](../evidence/gob-02-death/README.md): `preset_poses_x4.png` and `preset_parts_x3.png` (every pose and part at presets 4, 3, 2, 1), `parts_anchors_x4.png`, `frame_anchor_timing.png`, `scenes_1x.png` (lit Floor 1 at 1x with Carl and Donut: crush under the ENV-02 stone, decapitation tick, headless collapse, a scatter of parts), `storyboard_crush.png` and `storyboard_decap.png` (section 8 timelines as stills), `preset_strip.png`, and per-atlas `poses/` and `parts/` toolkit output (contact sheets at x1 and x4, pose GIFs at x1 and x3, SHA256SUMS).

## 12. Files

| File | What |
| --- | --- |
| `scripts/author_gob02_death.py` | authoring record: palette, rules and literal rows, `keys [--check]`, `scale` |
| `assets/source/characters/goblin/death/goblin_a_death_h36.grid.json` / `.ase` | poses source |
| `assets/source/characters/goblin/death/goblin_a_death_parts.grid.json` / `.ase` | parts source |
| `assets/source/characters/goblin/death/*.pixelart.json` | toolkit configs (validators, frozen keys) |
| `tests/fixtures/gob-02-death_keys.json`, `gob-02-death-parts_keys.json` | frozen keys |
| `assets/manifests/goblin_a_death_h36.json`, `goblin_a_death_parts.json` | manifests with `death_poses` (anchors per frame) and `death_parts` (anchors, `frame0_origin`, spawn metadata) |
| `scripts/gob02_death_evidence.py` | evidence stills: preset sheets, anchor and timing diagram, storyboards, 1x scenes (seeded placement, not a Bevy capture) |
| `tests/test_art_gob02_death.py` | spec rules beyond the toolkit, anchor and metadata schema, gore separation on every part |
| `docs/evidence/gob-02-death/` | design-stage evidence |

## 13. Rulings (silhouette gate 2026-11-18, blind review 2026-11-19) and open notes

1. **Variant B is a separate follow-up work order**, not this executor pass. Approach: an `accessory` layer on the head-bearing frames (poses 0, 1, 2; parts `head`, `head_dead`) with GOB-01 section 9's B overlay rule; headless frames and parts are shared. Gate it separately after this set is final.
2. **Remains stay in front of `loose_block_landed`** (actors layer, no exception). A goblin hidden under the 96 px block would lose the payoff; the low heap does not hide the stone (see the composited scale sheet). Revisit only if the Bevy capture reads as "lying next to" rather than "squeezed out from under".
3. **Preset 1 decapitation: the head still flies with its cut hidden** (default). Optional later data flag, an owner decision and not part of this pass: decapitation falls back to GOB-01 `knockback` → `corpse` + dust at preset 1, mirroring the explosive fallback. No art change either way.
4. **Remains contrast** is solved by the gate fix (the broken `u` top light, the notched outline and the folds on the rag mass). No palette change.
5. Non-blocking note: `torso_upper` reads only as a generic chunk at 1x (acceptable for spinning debris). An optional later polish is a 1 px `u` rim on the vest hump's top edge; the key stays frozen for now.
6. **Blind review rulings (2026-11-19).** (a) D1 design fix: `crushed_remains` re-keyed to face right like keys 0 and 1 (mirrored, x' = 59 - x; `pool_origin` (29, 55)); only key 2 re-frozen. (b) `gib_bone` is preset 4 only. (c) Flat floor decals are exempt from the 28 px width cap (40 x 4 maximum, drawn under every actor); `decal_pool_3` becomes 38 px. (d) Preset 4 is pushed through the spawner and decals, not denser sprites: crush drops 16 to 20, earlier and bigger crush pool, a crush wall spatter, a second wall-spatter variant, a bigger explosion count (section 8). (e) `arm_*` reading as a bent hook is accepted for tumbling debris; a 3 x 3 `K`-outlined fist knob is optional polish.

6. Open notes from the executor pass (no art change made): the section 5 table and the section 7 gib note disagreed on bone gibs (kept at preset 4 only); `arm_near` and `arm_far` read as a bent hook at 1x, which is fine for spinning debris but is the weakest part silhouette; the wall spatter in the 1x scenes lands on the dark alcove wall, so its contrast should be re-checked on brick in the Bevy capture; the 90 degree rotation steps in the storyboards are an evidence simplification, the runtime rotates freely.

## 14. Rights and provenance

Fan-concept art under the owner-reported `ART-PUBLICATION-ALL-V1` scope; no public reuse licence; no game or asset-pack distribution right. Every pixel comes from GOB-01 A sources (frames 0, 16 and 25) or from the shape rules and literal rows in `scripts/author_gob02_death.py`. Nothing was traced or sampled from concept boards, the Death Lab or any image-model output.

### Rights/Provenance

- Design, spec, hero keys and silhouette sheet: Claude Opus (top tier); gate fixes to keys 0 and 2 and the composited sheets: same designer.
- Silhouette gate: independent Claude Opus (FIX-KEYS, rulings recorded in section 13).
- Execution (section 9): in-betweens (poses 1 and 4), parts 3 to 23, anchors and manifests, export, validators, tests and evidence: Claude Sonnet (medium tier).
- Independent review: pending, separate top-tier agent.
