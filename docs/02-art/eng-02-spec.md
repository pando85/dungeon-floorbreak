# ENG-02: Goblin Engineer completion art, design spec

**Status:** executed. The design step is done and the independent design gate ruled FREEZE-with-notes (the two get-up keys are frozen as committed, `tests/fixtures/eng-02_keys.json`; its two design defects, the crush pot placement of section 8 and the flung launch timing of section 7, are fixed in this spec). The executor (Claude Sonnet) built the ram run, the slammed sheet, the death poses, the parts sheet, the wiring, the tests and the evidence; the deviations it recorded are under "Executor notes" at the end. **Owner art review pending, game-feel review pending, independent review of the finished frames pending.** Evidence: [`../evidence/eng-02/README.md`](../evidence/eng-02/README.md).

ENG-02 is the art order of [design sheet section 10](../01-design/enemy-gameplay-engineer.md#10-frames-per-state-and-the-missing-art-order-eng-02) (audit row E5). Today every Engineer death recipe falls back to `engineer_down`, which shows ENG-01 frames 15 to 17. This order gives each recipe its own art. It also replaces the get-up stand-in (frame 13, `attack_recover`), and it adds a ram run so the held frame 12 stops sliding.

Sections follow `docs/02-art/eng-01-spec.md` (the base set) and `docs/02-art/gob-02b-death-spec.md` (a death set derived from committed cels). Coordinates are frame pixels: x right, y down, frame 64 x 56, pivot (28, 55). The exception is a part cel, which is 32 x 32 with pivot (16, 16).

## 1. Canon and tone

- The Engineer's canon is unchanged from [ENG-01 section 1](eng-01-spec.md#1-canon-and-tone-constraints):
  - a level-3 goblin in a kitchen-pot helmet, with the GOB-01 body, the belt with its iron buckle, and a wrench stowed at the back hip
  - **the wrench is never in the hand and never swung** ([ENG-01 section 12](eng-01-spec.md#12-resolved-questions-from-the-stub), item 3)
- Tone follows [the antihero tone](../01-design/antihero-tone.md): a scrappy, slightly ridiculous threat. His deaths are the goblin's brutal deaths, with one running gag: **the pot outlives him**. It is squashed flat under the block, rides the severed head, or pops off and clatters on the floor.
- Death language follows [the monster death proposal](../03-tech/monster-death-system-proposal.md) and the merged GOB-02 presenter. Nothing here changes an authoritative outcome. Every frame, part and launch is presentation only.

## 2. Difficulty and route

**Route: Easy, difficulty score 1** (special states only), as in the [routing by difficulty](../07-agents/pixel-art-pipeline.md#routing-by-difficulty) table.

Only two poses are new: the get-up sit and the shove. They are the design keys, drawn and frozen here. Every other ENG-02 frame is a mechanical derivation of committed cels by rules stated in this spec, and the executor builds them:

| Derived from | Source cels | Derivation rule |
| --- | --- | --- |
| Ram run | ENG-01 frame 12 blocks | legs redrawn |
| Slammed | GOB-01 A slammed cels (CF02) | belt recoloured, weapon dropped |
| Death poses | GOB-02 A death cels | belt recoloured, weapon dropped, flat pot added |
| Parts | GOB-02 A parts, the ENG-01 `POT` and `WRENCH` blocks | see section 9 |

No design decision is left to the executor. Section 13 lists what may change during execution and what must come back to design.

## 3. Frame, pivot and palette

- **Frame:** 64 x 56, pivot (28, 55), contact row 55, facing right, tick rate 60. This matches ENG-01, GOB-02 A and the CF02 slammed sheets, so one pose-to-world transform serves every sheet.
- **Get-up and ram run:** palette `eng-floor01-v0`, the 17 ENG-01 keys unchanged. These frames join the ENG-01 atlas.
- **Slammed, death poses and parts:** palette `eng-death-floor01-v0`, 20 of 20 keys: the 17 ENG-01 keys plus the three GOB-02 gore keys.

  | Key | Name | Hex |
  | --- | --- | --- |
  | `R` | red_mid | `#b8322eff` |
  | `B` | blood_dark | `#6e1a22ff` |
  | `P` | blood_wet | `#e2434bff` |

  Every hex shared with `goblin-death-floor01-v0` is identical (checked: `K a s m l r t u w o R B P`).
- **Belt recolour.** The GOB cels' rope strap (wood keys `x` and `y` in the `body` layer) becomes the Engineer's belt:
  - every strap pixel becomes `u`
  - strap pixels with x divisible by 3 become `t`
  - a 2-px `5`, `6` buckle is placed by the buckle rule of section 7

  This is `belt_recolour()` in `scripts/author_eng02.py`. No other body pixel changes colour.
- **Eyes.** No yellow `e` appears in any slammed, death or part cel, as in ENG-01's rule `E1-no-eyes-after-ram`. GOB `R` pixels (dead-eye dots, tongue, cuts) stay `R`.

## 4. Frame list

| Sheet | Index | Tag | Kind | Ticks | Source | Pot | Wrench |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `eng_01` atlas | 18 | `getup` | **key, frozen** | 10 | GOB-01 A frame 23 body, belt | jammed over the eyes | none |
| `eng_01` atlas | 19 | `getup` | **key, frozen (hero)** | 12 | literal rows | shoved up, far hand on the handle | stowed at the hip |
| `eng_01` atlas | 20 | `attack_ram_run` | derived | 4 (state clock) | frame 12 blocks, legs passing | frame 12's | frame 12's |
| `eng_01` atlas | 21 | `attack_ram_run` | derived | 4 (state clock) | frame 12 blocks, legs swapped | frame 12's | frame 12's |
| `eng_slammed_h36` | 0, 1 | `slammed_stomped` | derived | 8, 6 (GOB A) | GOB A slammed 0, 1 | none (launched) | none (launched) |
| `eng_slammed_h36` | 2, 3 | `slammed_wall` | derived | 8, 6 | GOB A slammed 2, 3 | none (launched) | none (launched) |
| `eng_slammed_h36` | 4, 5 | `slammed_flung` | derived | 8, 6 | GOB A slammed 4, 5 | none (launched) | none (launched) |
| `eng_death_h36` | 0, 1 | `crush_contact` | derived | 3, 3 (GOB-02 A) | GOB-02 A 0, 1 | flat, squashed under the stone | none (launched) |
| `eng_death_h36` | 2 | `crushed_remains` | derived | 6 | GOB-02 A 2 | flat, on the head | none |
| `eng_death_h36` | 3 | `decap_contact` | derived | 4 | GOB-02 A 3 | on the head part | stowed at the hip |
| `eng_death_h36` | 4, 5 | `headless_collapse` | derived | 6, 6 | GOB-02 A 4, 5 | on the head part | stowed at the hip (4); see section 8 for 5 |
| `eng_death_parts` | 0 to 26 | parts | derived | n/a | section 9 | section 9 | section 9 |

Frame counts against the work order:

| Set | Ordered | Delivered |
| --- | --- | --- |
| Get-up | 2 | 2 |
| Slammed | 6 | 6 |
| Crush and decap | 4 | 6 frames on 4 tags (ENG2-AD5) |
| Pot death part | 1 | 1, plus `head_bare`, `head_bare_dead` and `wrench` |
| Ram run (optional) | 2 | 2, taken (ENG2-AD4) |

The slammed and death tick values are the GOB sheets' own `duration_ms` at 60 Hz. The executor copies those values; it does not retime them.

## 5. The two keys (frozen)

Sources:

- generator: `scripts/author_eng02.py`
- grid: `assets/source/characters/goblin/engineer/eng_02_keys.grid.json`
- Aseprite file: `assets/source/characters/goblin/engineer/eng_02_keys.ase`
- validator config: `eng_02_keys.pixelart.json`
- fixture: `tests/fixtures/eng-02_keys.json`

Layers are `body` and `prop`, the ENG-01 split: the wrench box is `prop`.

| Key | Grid frame, atlas frame | Pose | Bounds | Reading targets |
| --- | --- | --- | --- | --- |
| Sit | 0, 18 | GOB-01 A frame 23 (the goblin get-up sit) with the Engineer belt (buckle at (29, 50) and (30, 50)). The ENG-01 `POT` block is moved (-4, +11) so the rim covers the eye row. No wrench. | top row 27, bottom row 55 | No `e`. The teeth show (`w` 2 px or more). The pot is the largest light shape, and the face under it reads as "can't see". |
| Shove (hero) | 1, 19 | Body upright, the GOB-01 stance. The far arm is raised behind the head, and its 2-px forearm grips the handle and levers the pot up and back. The pot is lifted 3 rows off the crown, and the bald crown shows under it. Eyes back (`e` 3 px or more). The wrench is stowed at the back hip (box (14, 39) to (18, 46)). | top row 16, bottom row 55 | The silhouette is a raised elbow plus the pot tipped back off the head. At 1x it reads "pushes his helmet up", never "salutes" or "throws". |

The shove is literal rows in the generator (`KEY_SHOVE`). The sit is composed from committed blocks. Both are frozen; changing either one needs a design change and a re-freeze.

Gate: `eng02-gate.png`, which is not committed (it is a design review image). It shows both keys enlarged next to ENG-01 idle 0 and down 17, at 4x in colour and as black silhouettes, plus a 1x strip on the Floor 1 ground tone. It also previews the pot part and the flat pot rule.

**Enclosed pockets (V17, connectivity 4).** The independent design gate ruled all three intended negative space, and they are to be allowlisted.

| Key | Pocket | Size | What it is |
| --- | --- | --- | --- |
| Sit | (38 to 42, 50 to 53) | 11 px | The gap under the bent knee, from GOB-01 A frame 23. |
| Shove | (19 to 28, 25 to 28) | 26 px | The loop between the raised far arm and the head: it carries the "elbow up" read. |
| Shove | (21 to 23, 32 to 33) | 5 px | The gap between the upper arm and the back. |

The executor adds these three entries to `assets/source/pixel_hole_allowlist.json`, keyed by frame and pixels, citing the gate. It then turns V17 on in `eng_02_keys.pixelart.json` and sets that config's `max_colors` to 17.

Any other enclosed pocket in a derived frame is a defect, unless it is inherited verbatim from a committed GOB source frame. In that case, list it with its source frame.

## 6. Get-up and ram run (ENG-01 atlas frames 18 to 21)

**Get-up.** The `Downed` branch of `combat_anim::engineer_pose` currently shows the down frames while `down_ticks < lying_ticks`, then `attack_recover` 0 as the get-up. The new behaviour:

- `age = down_ticks - lying_ticks`
- `age < 10`: frame `getup` 0 (atlas 18)
- otherwise: frame `getup` 1 (atlas 19)

`q04_getup_ticks` is 22, which is exactly 10 + 12. The lying frames 15 and 16 are unchanged. Continuity:

- Frame 16 has the pot still on his head while he lies alive.
- Frame 18 has the pot fallen over his eyes as he sits up.
- Frame 19 has the pot shoved back.
- Then he returns to idle 0, where the pot rests on the crown.

**Ram run (taken, ENG2-AD4).** At 240 px/s, the held frame 12 slides 80 px over 20 ticks, which reads as a slide at 1x. The new frames, tag `attack_ram_run`:

- **20, passing:** all frame 12 blocks moved (0, -1), so the top row is 20 to 22. The near leg is lifted 2 rows and passes under the hips, foot right end about x 36. The far leg is planted, foot right end about x 32.
- **21, opposite contact:** the frame 12 blocks are unmoved (top row 21 to 23). The near leg trails, foot right end about x 30, heel lifted, toe pixels only on row 55. The far leg reaches forward and is planted flat, foot right end about x 40. This mirrors frame 12's braced-near, kicked-far contact.

The torso sack, the head, the pot, the gritted mouth, the near arm and the wrench are frame 12's pixels. Only the legs are redrawn, from the GOB-01 walk leg shapes; nothing is rotated or rescaled.

**Ram-run leg sources (binding).** The legs are not drawn freehand. They are taken from the committed ENG-01 walk frames:

- **Frame 20:** first move all frame 12 blocks by (0, -1), then paste the ENG-01 walk 7 legs (near leg lifted, far leg planted).
- **Frame 21:** paste the ENG-01 walk 6 legs (near leg back, far leg forward and planted) under the unmoved frame 12 blocks.
- **Both frames:** the walk legs are 7 rows high and frame 12's are 9. Extend both thighs 2 rows upward, continuing their own outline and fill columns, so that they meet frame 12's hip outline on row 46 (row 45 in frame 20, after the lift).
- The foot x values above are approximate, with a tolerance of +/- 1 px. The leg sources are binding; the x values are not.

The torso, the head, the pot and the wrench are frame 12's blocks verbatim.

The Ram phase plays the cycle `[12, 20, 21, 20]` at 4 ticks a frame, by ram age (ENG2-AD17). Launch (frame 11) and Recover (frame 13) are unchanged. The cycle reuses frame 20 for both passings, so one of them lifts the wrong leg. This is an accepted cheat at 4 ticks a frame (see Owner notes).

**Get-up clamp.** If `age` reaches 22 or more before the mode changes, hold frame 19. This is a defensive clamp, not a design change.

## 7. Slammed sheet `eng_slammed_h36`

**Source.** Each cel is the GOB A slammed cel (`goblin_a_slammed_h36.grid.json`) with these changes:

- The `weapon` layer is dropped. The pineapple is not his; nothing replaces it.
- The `body` layer is belt-recoloured.
- **Buckle rule (binding).** Find the longest horizontal run of strap pixels (wood keys `x` and `y`) of length n >= 4. The buckle goes on that run's row, and its left pixel is at index `floor((n - 2) / 2)` of the run. A cel with no such run gets no buckle. The rule gives exactly these buckles (left pixel), which a test pins:

  | Sheet | Frame | Buckle left pixel |
  | --- | --- | --- |
  | `eng_slammed_h36` | 2 | (35, 45) |
  | `eng_death_h36` | 0 and 1 | (20, 49) |
  | `eng_death_h36` | 2 | (32, 52) |

  Slammed frames 0, 1, 3, 4 and 5 and death frames 3 to 5 have no strap run, so they have no buckle. The recolour is a no-op there, and those slammed cels equal the goblin's minus the weapon. This is accepted under ENG2-AD6: the Engineer identity in the slam rests on the launched pot.
- The `gore` layer is kept as is.

Layers are `body` and `gore`. The tags and frame indices are the GOB A sheet's, so `cf02::slammed_pose`, `SlamArtInput` and `q21_slammed_impact_hold_ticks` work unchanged.

**The pot and the wrench are not in any slammed cel (ENG2-AD6).** The presenter launches two parts from launch points in the engineer poses manifest:

- the `pot` part (section 9), from `pot_launch` for the pose
- the `wrench` part, from `wrench_launch` for the pose

**Launch timing (binding, the gate's fix).**

- `stomped` and `wall`: at death age 0. `cf02::slammed_pose` is Some from age 0, so the slammed cel (no pot) replaces the base sprite on the same tick.
- `flung`: on the first tick `slammed_pose` is Some (the landing), from the `slammed_flung` frame 0 launch points. While the body is airborne, `slammed_pose` is None and the scene keeps the base sprite, ENG-01 frame 14 (hurt), which still wears the pot. An age-0 launch would show two pots in flight.

The rule in one line: at most one pot is visible on every tick, either on the base sprite or as the part, never both. A presenter test checks it for all three poses.

The pot clatters to the floor beside him and stays there (`ttl: keep`). The pot-shaped highlight on the floor is the cue that an Engineer died, not a goblin.

Launch points are in frame coordinates. The executor measures them from the cels:

- `pot_launch`: the head's top outline pixel nearest the head centre in the impact frame (`slammed_flung` frame 0 for the flung pose)
- `wrench_launch`: the rearmost belt pixel in the impact frame (`slammed_flung` frame 0 for the flung pose)

Velocities and spins (px/s and deg/s; `away` is the presenter's direction away from the attacker):

| Pose | Pot velocity (vx, vy up) | Pot spin | Wrench velocity | Wrench spin |
| --- | --- | --- | --- | --- |
| stomped | (away x 20 to 50, 150 to 210) | +/- 720 (coin) | (away x -30 to 60, 110 to 170) | +/- 900 |
| wall | (away x -80 to -40, 160 to 220): off the wall, back toward the attacker | +/- 720 | as stomped | +/- 900 |
| flung | (away x 120 to 170, 90 to 140): ahead of the body | +/- 720 | as stomped | +/- 900 |

The draws come from the death's own RNG, after `script_knockback`'s draws for that tick, and for an Engineer only. No goblin or llama sequence changes.

## 8. Death poses `eng_death_h36`

**Source.** Each cel is the GOB-02 A death cel (`goblin_a_death_h36.grid.json`):

- The `weapon` layer is dropped.
- The `body` layer is belt-recoloured.
- The `gore` layer is kept.
- Two layers are added: `pot` (the flat pot) and `prop` (the stowed wrench).

Layer order, back to front: `body`, `prop`, `pot`, `gore`. Tags, frame indices and durations are GOB-02 A's, so `Finish::pose_frame` is reused unchanged. Every anchor (`hand_near`, `mouth`, `squeeze_back`, `squeeze_front`, the decap anchors) is copied from `goblin_a_death_h36.json`, because the body pixels do not move. Add one anchor per frame, `hip_back`: the rearmost belt pixel. It is the wrench's launch point.

**Crush frames 0 to 2: the flat pot.** Use `flat_pot()` from `scripts/author_eng02.py`:

- 4 rows, 30 px wide (96 px)
- rows 16, 17, 23 and 24 of the pan stacked (iron does not stretch)
- the handle laid back on its third row
- no glint `7`

Paint it on the `pot` layer, with the pan over the skull and the handle trailing back. Remove the `body` pixels under the pot pixels, so the layers stay disjoint.

**Frames 0 and 1 (contact): the pot is under the stone (binding, amended by the gate).** GOB-02 A `crush_contact` frames 0 and 1 draw the stone's underside as a straight 42 px `K` line, on row 33 in frame 0 and row 36 in frame 1, and nothing above it. The pancake is squashed *inside* the head, between the stone and the face:

- The pancake's top row equals the underside row. It occupies rows underside to underside + 3, and no pot pixel is above the underside row.
- It covers the brow and the skull, not the popped eye and not the teeth. Its right end is at or left of the eye's left outline column (x 35 in both frames), with at most 2 px of overlap onto the outline only, never onto `w`.
- If the 30-px pancake does not fit, crop the handle end (left), never the pan.

The origins below meet these rules. They were checked against the GOB-02 A body cels: the pancake's rightmost pixel lands on the eye outline at x 35, and no `w` pixel is covered.

| Frame | Origin (top-left) | Pot rows | Note |
| --- | --- | --- | --- |
| 0 | (6, 33) | 33 to 36 | top row on the stone underside; handle over the flattened arm |
| 1 | (6, 36) | 36 to 39 | as frame 0, 3 rows lower |
| 2 | (28, 40) | 40 to 43 | remains; no stone on top; top row on the head's top outline row (moved up 1 px from (28, 41) by the independent review, section 13) |

The executor may not move the frame 0 and 1 origins up or right. It may move them left or down by up to 2 px only if a target below fails, and records the final values.

Targets:

- frames 0 and 1: no pot pixel above the underside row; the popped eye and the teeth stay visible
- frame 2: the pancake's top row is the head's top outline row
- the pancake is never clipped by the frame edge
- the pot layer has 90 or more pixels in each of frames 0 to 2

**Crush evidence.** The crush evidence is composited with the GOB-02 stone underside drawn, frames 0 to 2, at 1x and 4x. A sheet without the stone does not pass E2-4.

**Decap frames 3 to 5: no pot on the body.** The pot rides the severed head part (section 9). The wrench stays stowed at the back hip on the `prop` layer:

- It is the ENG-01 `WRENCH` block, whole and jaw-up.
- Its shaft bottom is 1 row below the hip's belt row, and it is 1 to 3 px behind the rearmost belt pixel, as in ENG-01 key 0.
- In frame 5, the collapsed lying body, place it the same way if its 5 x 8 box fits without crossing a gore pixel. Otherwise omit it in frame 5 and record that.

**Crush: the wrench.** The crush script launches the `wrench` part at age 0 in place of the goblin's club (ENG2-AD10):

- from `hip_back`, not `hand_near`
- with the club's velocity and spin draws, in the same RNG order

The crush frames therefore carry no wrench.

## 9. Parts sheet `eng_death_parts`

The cell is 32 x 32 with pivot (16, 16). Layers are `body`, `prop` and `gore`; the GOB `weapon` layer is renamed `prop` and holds iron only. Indices follow GOB-02 A order with `club` (2), `club_stick` (9) and `pineapple` (10) removed, then four parts appended, 27 parts in all. Every part keeps the GOB-02 A field values unless the table says otherwise. `frame0_origin` is in ENG-01 frame 0 coordinates.

| Index | Id | Source and placement | Changes from GOB-02 A |
| --- | --- | --- | --- |
| 0 | `head` | GOB-02 A `head` cel moved (+4, +2), with the ENG-01 `POT` block at cell = frame - (13, 9) | `frame0_origin` (29, 25); anchor `neck` (15, 22); `swap_on_first_contact: head_dead` |
| 1 | `torso_upper` | GOB-02 A, unchanged pixels | none |
| 2 | `head_dead` | GOB-02 A `head_dead` moved (+4, +2), plus the pot as for `head` | `frame0_origin` (29, 25) |
| 3 | `torso_lower` | GOB-02 A, belt-recoloured | none |
| 4 to 7 | `arm_near`, `arm_far`, `leg_near`, `leg_far` | GOB-02 A, unchanged | none |
| 8 | `ear` | GOB-02 A, unchanged | none |
| 9 to 22 | gibs, rag, blood drop, decals | GOB-02 A, unchanged, same order | none |
| 23 | `pot` | `pot_part()`: the ENG-01 `POT` block whole, cell = frame - (12, 4) (cell columns 1 to 30, rows 12 to 20), one glint `7` | class `clean`, material `iron`, mass `medium`, spin `fast`, settle `clatter`, bounce 0.45, presets [1, 2, 3, 4], `frame0_origin` (28, 20), anchor `grip` (2, 17), no stain, no trail, ttl `keep` |
| 24 | `head_bare` | GOB-02 A `head`, unchanged | `frame0_origin` (33, 27); `swap_on_first_contact: head_bare_dead`; presets [2, 3, 4] |
| 25 | `head_bare_dead` | GOB-02 A `head_dead`, unchanged | `frame0_origin` (33, 27); presets [2, 3, 4] |
| 26 | `wrench` | the ENG-01 `WRENCH` block whole, cell = frame - (0, 22) | class `clean`, material `iron`, mass `light`, spin `fast`, settle `clatter`, bounce 0.3, presets [1, 2, 3, 4], `frame0_origin` (16, 38), anchor `grip` (16, 20), no stain, no trail, ttl `keep` |

On `head` and `head_dead`, the moved goblin head plus the pot spans cell x 0 to 31 and fits. The pot is painted over the head where they overlap, as in ENG-01 key 0. Neither cel may be clipped.

**Layers of the head parts.** On `head` and `head_dead`, the pot pixels go on the `prop` layer, and the `body` pixels under them are cleared, so the layers stay disjoint. `head_bare` and `head_bare_dead` have no `prop` pixels.

**Engineer explosion tables** (`ENGINEER_EXPLOSION_TABLES`): the goblin tables with these part lists, and every other field equal to the goblin row.

| Preset | Parts |
| --- | --- |
| 1 | none: the sprite stays, with its pot |
| 2 | `head_bare`, `torso_upper`, `wrench`, `pot` |
| 3 | `head_bare`, `torso_upper`, `arm_near`, `leg_far`, `wrench`, `pot` |
| 4 | `head_bare`, `torso_upper`, `torso_lower`, `arm_near`, `arm_far`, `leg_near`, `leg_far`, `ear`, `wrench`, `pot` (10 parts, as the goblin's) |

## 10. Recipes and finishes

The `goblin_engineer` entry of `add_species` in `assets/data/death/species_death_profiles_cf02.json` changes as below. The causes and forces are unchanged. `engineer_down` is dropped everywhere (ENG2-AD11).

| Recipe kind | Cause, force | Finish id (new) | `Finish` | Art |
| --- | --- | --- | --- | --- |
| `slammed` | blunt, heavy | `engineer_slammed` | `GoblinSlammed(pose from events)` | `eng_slammed_h36`; pot and wrench launched |
| `exploded` | blast, catastrophic | `engineer_exploded` | `Exploded` | engineer explosion tables; parts sheet |
| `decapitated` | shear, heavy | `engineer_decapitated` | `Decapitated` | `eng_death_h36` 3 to 5; `head` (potted) flies |
| `crushed` | crush, routine | `engineer_crushed` | `Crushed` | `eng_death_h36` 0 to 2; flat pot; wrench launched |
| `knockback_down` | blunt, routine | `engineer_knockback_down` | `Knockback` | ENG-01 `down` 15 to 17 (the scene's sprite) |
| fallback | | `engineer_knockback_down` | `Knockback` | as above |

## 11. Wiring notes (for the executor)

**`src/gore.rs`**

- Add `ENGINEER_POSES_MANIFEST` and `ENGINEER_PARTS_MANIFEST`.
- Add `ENGINEER_VARIANT = DEATH_VARIANTS + 1`, after the llama.
- Add `DeathArt::load_engineer`, which calls `load_manifests` like `load_llama`.
- Set the manifests' `always_shown_layers` to `["body", "prop", "pot"]`, the llama convention. This field is documentation only: Rust does not read it, because `GridFile::compose` already draws every non-gore layer. Do not add code that depends on it. The `gore` layer follows the goblin rule.

**`src/death_present.rs`**

- Load an `engineer: Option<SpeciesDeath>`, the llama's `LlamaDeath` struct renamed or duplicated.
- Generalise `art_for`, `sheets_for` and `art_of` to pick by variant. Today `art_for` and `sheets_for` clamp with `variant.min(DEATH_VARIANTS - 1)`, which would map the Engineer back onto a goblin variant. Replace the clamp: if the Engineer art is loaded, use the Engineer sheets; only if it is None, fall back to `goblin_variant`. No silent clamp. A test checks that an Engineer death resolves the Engineer sheets. `decal_set(ENGINEER_VARIANT)` is 0: the goblin decals are identical.
- `begin_death` gives an Engineer target `ENGINEER_VARIANT`, not `goblin_variant(target)`.
- `Finish::from_id` maps the five new ids of section 10 and drops `engineer_down`.
- `explosion_table_of` returns `ENGINEER_EXPLOSION_TABLES` for the Engineer.
- `script_crush`: the held-weapon launch picks `(part, anchor)` by variant: `("club", "hand_near")` for the goblins, `("wrench", "hip_back")` for the Engineer. The draws are unchanged.
- `script_decap` needs no change: `head` is the potted head.
- The `GoblinSlammed` branch keeps `script_knockback`. For `ENGINEER_VARIANT` it adds the pot and wrench launches of section 7, with section 7's timing: age 0 for `stomped` and `wall`, the first tick `slammed_pose` is Some for `flung`. Their points come from `death_poses.spawner_hints.slam` in the engineer poses manifest, keyed by pose (`stomped`, `wall`, `flung`), each with `pot_launch` and `wrench_launch`, in frame coordinates of the 64 x 56 pivot (28, 55) frame.

**`src/combat_anim.rs` and `src/combat_anim/cf02.rs`**

- `engineer_ticks` accepts 22 frames: the `getup` and `attack_ram_run` frames are held by state, so they are `None`.
- `EngineerFrame` gains `ram_age: Option<u16>`, filled from the Engineer's phase clock.
- `engineer_pose`: the `Downed` branch is section 6, including the hold-frame-19 clamp at `age` 22 or more, and `Some(P::Ram)` cycles as in section 6.
- Load the engineer slammed clips next to `load_engineer`. Add `engineer_slammed_pose(index, slam)`, which mirrors `goblin_slammed_pose` with the engineer clips.

**`src/bin/floor01_scene.rs`**

- The slammed sprite system picks `engineer_slammed_pose` when `enemy.is_engineer()`, and the engineer sprite's texture and layout.
- Check that the Engineer's regular sprite is hidden while a death pose override or a slammed pose shows, as the goblin's is. If it is not, extend the same hide list.

## 12. Validators and acceptance targets

Every new grid has a `.pixelart.json` and passes `python3 -m scripts.pixelart validate`, including V17 holes (connectivity 4; intended holes go on the allowlist with a reason). Palette limits (`max_colors`): 17 for the keys config and the ENG-01 atlas; 20 for the slammed, death and parts sheets. Isolated pixels: 0. Orphan pixels: at most 9 per frame.

Sheet-specific targets:

| Id | Sheet | Target |
| --- | --- | --- |
| E2-1 | `eng_01` | Frames 0 to 17 are byte-identical to `main`. Frames 18 and 19 are byte-identical to the frozen key grid frames 0 and 1 (both layers). |
| E2-2 | `eng_01` | Ram run frames 20 and 21: bottom row 55; right edge 51 or more. Top rows: 20 to 22 for frame 20, 21 to 23 for frame 21. `e` 3 or more; glint `7` exactly 1; pot `5` 40 or more. Only leg pixels differ from frame 12, apart from frame 20's whole 1-row lift. |
| E2-3 | `eng_slammed_h36` | The `body` equals GOB A slammed `body` after `belt_recolour` (a test regenerates it). There is no weapon layer, no `e` and no pot key. The buckles are exactly those of the section 7 table, in this sheet and in `eng_death_h36` (a test pins the coordinates). |
| E2-4 | `eng_death_h36` | The `body` equals GOB-02 A `body` after `belt_recolour`, minus the pixels under the pot. The pot layer appears only in frames 0 to 2, with 90 or more pixels each and no `7`. In frames 0 and 1, no pot pixel is above the stone underside row (33, 36) and no `w` pixel is covered (a test checks both). The `prop` layer appears only in frames 3 to 5. Layers are disjoint. The crush evidence shows the stone underside drawn, frames 0 to 2, at 1x and 4x. |
| E2-5 | `eng_death_parts` | 27 parts, with ids and indices exactly as section 9. Each of `pot`, `head` and `head_dead` has exactly one `7`. No cell is empty. Unchanged parts are pixel-identical to GOB-02 A (a test diffs them). |
| E2-6 | death profile | No `engineer_down` anywhere: not in `Finish::from_id`, the overlay recipes or the fallback. A test greps `src/` and `assets/` and finds no `engineer_down` string. Each of the five recipes resolves to the finish of section 10 (a unit test). |
| E2-7 | presenter | An Engineer crush launches exactly one `wrench` and no `club`. An Engineer slam launches one `pot` and one `wrench`. The decap's `head` part is the potted one. The explosion at presets 2 to 4 launches `pot` and `head_bare`, never `head`, `club` or `pineapple`. Goblin and llama part lists and RNG draws are unchanged (the existing tests pass unchanged). During a slam in each pose, at most one pot is visible on every tick: base sprite or part, never both (a test). An Engineer death resolves the Engineer sheets, not a goblin variant (a test). |
| E2-8 | runtime | `--scene f01_late --f01-run` shows the get-up, the ram run and each finish on the Engineer in room 2 (Boiler) and room 6 (mastery). |

## 13. Execution boundary

The executor may:

- move a crush pot origin left or down (frames 0 and 1) or in any direction (frame 2), or a ram-run foot, by up to 2 px to meet a target (recorded)
- clean up orphan pixels in derived frames (recorded)
- choose the exact launch pixels by the rules in section 7

The executor must send these back to design:

- any change to keys 18 or 19
- any new pose
- any palette key
- a pot or wrench appearing where section 4 says none
- a target that cannot be met within 2 px

## 14. Adopted defaults

| Id | Default | Reason |
| --- | --- | --- |
| ENG2-AD1 | ENG-G-Q24 (the owner's game-feel verdict on the Engineer before any art) is **waived**. | Owner decision, 2026-11-25: "go with all" owner-gated items with adopted defaults recorded by id. |
| ENG2-AD2 | Route Easy, score 1. Only the get-up pair are design keys; everything else is a rule-based derivation. | Every other frame exists as a committed GOB-02 A, CF02 or ENG-01 cel. |
| ENG2-AD3 | The get-up is appended to the ENG-01 atlas (18, 19; tag `getup`; 10 + 12 ticks = `q04_getup_ticks`). It replaces the `attack_recover` stand-in. | No new sprite path; the window is the gameplay's own. |
| ENG2-AD4 | The ram run is taken: frames 20 and 21, tag `attack_ram_run`, cycle [12, 20, 21, 20] at 4 ticks. | The held frame 12 slides 80 px over 20 ticks; this is section 10's condition for taking it. |
| ENG2-AD5 | Crush and decap use 6 frames on the 4 GOB-02 tags: 4 keyed poses plus GOB-02 A's 2 in-betweens. | `Finish::pose_frame`, the timings and the anchors are reused unchanged. The work order's "4" counts the tags. |
| ENG2-AD6 | No pot is drawn in any slammed cel. The pot and the wrench are launched as parts at the slam, in all three poses. | Section 10 asked for a pot layer, knocked loose in wall and flung. The stomped head is tilted, and blocks are never rotated, so a layer could only perch the pot oddly. A launched pot reads at 1x and lands visibly. |
| ENG2-AD7 | Crush: a 4-row flat pot (rows 16, 17, 23 and 24 of the pan, full width) is squashed into the head in frames 0 to 2. In the contact frames 0 and 1 its top row is the stone's underside row and nothing is above it; it covers the brow and the skull and leaves the popped eye and the teeth visible (origins (6, 33) and (6, 36)). Frame 2 uses origin (28, 40). Decap: the `head` part carries the pot. *Amended by the independent design gate: the first origins put the pot above the stone.* | It is the gag of section 1; iron does not stretch. The stone must read as resting on the goblin, not on a hat. |
| ENG2-AD8 | The pot part is one cel rotated at runtime, not 3 rotation cels. | The presenter spins parts (`angle_deg`, `spin_dps`), so 3 drawn cels would double-rotate. This deviates from design sheet section 10. |
| ENG2-AD9 | Palette `eng-death-floor01-v0`: 20 keys (the 17 ENG-01 keys plus `R`, `B` and `P` at the GOB-02 hex). | The GOB-02 gore and wound pixels need them; no new hue. |
| ENG2-AD10 | The wrench stays a stowed prop. The crush and the slam launch it from the back hip (`hip_back`). The explosion throws it at presets 2 to 4. It is never drawn in a hand. | ENG-01 section 12, item 3. |
| ENG2-AD11 | Finish ids are `engineer_slammed`, `engineer_exploded`, `engineer_decapitated`, `engineer_crushed` and `engineer_knockback_down`. The last is also the fallback and keeps ENG-01 `down` 15 to 17. `engineer_down` is dropped. | One id per finish, as for the goblin and the llama. |
| ENG2-AD12 | The Engineer's death art is its own variant, `ENGINEER_VARIANT = DEATH_VARIANTS + 1`. Decals are shared with goblin set 0. | It follows the llama's pattern; the blood is identical. |
| ENG2-AD13 | The parts sheet drops `club`, `club_stick` and `pineapple`, and appends `pot`, `head_bare`, `head_bare_dead` and `wrench`. `head` and `head_dead` carry the pot. | No empty cells (V15); the presenter looks parts up by id. |
| ENG2-AD14 | No yellow `e` in any slammed, death or part cel; the GOB `R` pixels stay. | ENG-01's `E1-no-eyes-after-ram` rule. |
| ENG2-AD15 | ENG2-Q1: the death sheets use 20 keys (the 17 ENG-01 keys plus `R`, `B` and `P`). | Recommended answer, adopted after the gate. |
| ENG2-AD16 | ENG2-Q2: one runtime-rotated pot cel. | Recommended answer, adopted after the gate. |
| ENG2-AD17 | ENG2-Q3: the ram run plays at 4 ticks a frame. | Recommended answer, adopted after the gate. |
| ENG2-AD18 | ENG2-Q4: the wrench reappears on the shove; the sit stays as frozen. | Recommended answer, adopted after the gate. |
| ENG2-AD19 | ENG2-Q5: the stomped pot pops off. | Recommended answer, adopted after the gate. |
| ENG2-AD20 | Flung slam: the pot and the wrench launch on landing, not at age 0 (section 7). | Gate finding: the airborne base sprite still wears the pot. |

## 15. Questions (answered by adopted defaults)

The gate took the recommended answer for each question. The owner may change any of them in a follow-up.

| Id | Question | Adopted answer |
| --- | --- | --- |
| ENG2-Q1 | Should the death sheets keep strictly to the 17 ENG-01 keys, with blood mapped to `o`? | 20 keys (ENG2-AD9, ENG2-AD15). |
| ENG2-Q2 | Is one runtime-rotated pot cel enough, or should there be 3 drawn rotation cels with spin off? | One cel (ENG2-AD8, ENG2-AD16). |
| ENG2-Q3 | Ram run timing: 4 ticks a frame (5 frames per ram) or 3 (about 7)? At 240 px/s the feet still slide at either value. | 4 ticks (ENG2-AD17). |
| ENG2-Q4 | Get-up continuity: the wrench is absent in ENG-01 `down` 15 to 17 and in the sit, and reappears in the shove. Should the sit show it at the hip as well? This needs a sit re-freeze. | As frozen: it reappears on the shove (ENG2-AD18). |
| ENG2-Q5 | Stomped: should the pot stay on, flattened like the crush, instead of popping off? | It pops off (ENG2-AD6, ENG2-AD19). |

### Owner notes (game-feel review)

- **Ram cycle cheat.** The ram cycle `[12, 20, 21, 20]` reuses frame 20 for both passings, so the wrong leg is lifted on one of them. At 4 ticks a frame during a charge this is an accepted cheat. It is flagged for the owner's game-feel review and is not a blocker.
- **Slammed identity.** Slammed frames 0, 1, 3, 4 and 5 are the goblin's cels minus the weapon (no strap, so no belt). The Engineer reads in the slam only through the launched pot (ENG2-AD6).

## 16. Files

**Added in the design step:**

- `scripts/author_eng02.py`
- `assets/source/characters/goblin/engineer/eng_02_keys.grid.json`, `.ase` and `.pixelart.json`
- `tests/fixtures/eng-02_keys.json`
- `tests/test_art_eng02.py`
- this spec

**Added by execution:**

- `eng_slammed_h36` and `eng_death_h36`: grid, `.ase` and `.pixelart.json` under `assets/source/characters/goblin/engineer/`
- `eng_death_parts`: the same three source files, in the same folder
- manifests `assets/manifests/eng_slammed_h36.json`, `eng_death_h36.json` and `eng_death_parts.json`
- the generated atlases (`generated/art/`, not committed: `bash scripts/export_assets.sh`)
- `scripts/eng02_evidence.py` and `scripts/make_eng02_evidence.sh`, and `docs/evidence/eng-02/`

**Changed by execution:**

- `eng_01` grid, `.ase` and manifest (22 frames)
- `tests/test_art_eng01.py`
- the death profile overlay
- the Rust files of section 11
- evidence

## 17. Rights and provenance

### Rights/Provenance

Fan-concept art under the owner-reported `ART-PUBLICATION-ALL-V1` scope (see `docs/04-production/rights-and-release.md`). The pixels are derived from committed ENG-01, GOB-01 A, GOB-02 A and CF02 cels and the DOZ-01 pot; the shove key is new literal rows. No tracing, no image-model raster output and no colour sampled from a concept.

- Design, size and palette decisions, and the two keys: Claude Opus.
- Design gate: FREEZE-with-notes by a separate, independent Claude Opus agent. Its two design defects (crush pot placement, flung launch timing) are fixed in this spec.
- Execution: Claude Sonnet (frames 20 and 21, the slammed, death and parts sheets, the manifests, the validators, the wiring, the tests and the evidence).
- Independent review of the finished frames: pending (a separate top-tier agent).

## 18. Executor notes

Written by the executor (Claude Sonnet). Each item is a deviation from, or a reading of, the spec above; none changes a key, a palette or a design rule, and none sends anything back to design except where it says so.

**ENG2-X1: the belt strap of death frames 3 and 4 is on the `weapon` layer (carried to `body`).** In GOB-02 A the 18 px strap run of the two decapitation cels (and the outline pixel at its right end) is drawn on the `weapon` layer, with the club. Section 8 says to drop that layer; doing so cuts a transparent slit through the torso (rows 43 and 47, open to the right). The executor carries the strap run and its end outline to `body` and recolours it with `belt_recolour()`, with no buckle (section 7 pins none in frames 3 to 5, and the rule is applied to the `body` layer as authored). Slammed frames have no such strap (their weapon layer is the club only). The gate's note that frames 3 to 5 "have no strap run" is true of `body` only.

**ENG2-X2: orphan clean-ups (V06, at most 9 per frame, section 13).** The belt rhythm (`u u t`) leaves single `t` and `u` pixels, which count as orphans. Death frames 0, 3 and 4 were over the limit. The first body-layer orphan that is not inherited from the GOB source cel is recoloured to the commonest key among its neighbours (a belt pixel only among `u`, `t`, `r`), until the frame passes. Applied, in order: frame 0 `(6, 36) s -> K` (a skin pixel leaking past the pot's handle outline); frame 3 `(14, 43) u -> r`, `(24, 43) t -> r`; frame 4 `(12, 47) t -> r`, `(21, 47) t -> r`, `(24, 47) t -> r`. Pinned by a test (`cleaned_orphans`). The slammed sheet's `max_orphans_per_frame` is 9 (the GOB sheet's 0 predates the belt rhythm; frame 2 has 6).

**ENG2-X3: the stowed wrench of frames 3 and 4.** It is the ENG-01 `WRENCH` block with its bottom row on the belt row and its right edge 2 px behind the rearmost belt pixel, which is ENG-01 key 0's own relation (the block's bottom row, 42, is the belt row; the spec text "1 row below" reads as the same thing). In both cels the torso's rear outline is further back than the belt, so the wrench slides back 2 px more to clear the body: 4 px behind the belt's first pixel, 3 px behind the strap's end outline. Frame 5 (the collapsed body) has no belt, and its box would cross gore pixels, so the wrench is **omitted in frame 5** (section 8 allows it; recorded here and pinned by a test).

**ENG2-X4: ram-run legs.** The walk legs are the sources, as binding. Walk 7 has no 2 row thigh to add (its torso is unmoved, so its legs start on row 47): frame 20's thighs extend 1 row up (to row 46, under the lifted hem on row 45); frame 21's extend 2 rows (to row 47). The foot x values of section 6 are approximate: frame 20's lifted near foot ends at x 32 and its planted far foot at x 36 (the spec text has them the other way round); frame 21's trailing near foot ends at x 28 (the spec says about 30) and its forward far foot at x 40. No foot was moved.

**ENG2-X5: crush pot origins.** (6, 33) and (6, 36) met every target and were not moved. Frame 2 started at (28, 41); the independent review found the head's ear-tip nub poking above the pan's top row, so it moved up 1 px to (28, 40) (section 13 allows 2 px), which puts the pan's top row on the head's top outline row (row 40) and covers the nub; the move opens a one pixel gap between the pot's handle and pan at (37, 43), closed with an outline `K` on the body layer (V17). A test pins the rule. The pancake leaves a few skin pixels of the head showing in the gap between its handle and its pan in frames 0 and 1 (the handle is a separate strip); it reads as the brow under the iron.

**ENG2-X6: slam launch points.** `pot_launch` is the top outline pixel (a `K` with nothing above it, no lower than the head centre) nearest the centroid of the cel's mouth and teeth pixels (`w` and `o`); `wrench_launch` is the rearmost belt pixel (`u` or `t` with another belt pixel beside it), or, for a cel with no belt (the stomped and flung impact cels have no strap), the rearmost rag pixel (`r`, `t`, `u`). Measured: stomped (cel 0) pot (16, 40), wrench (18, 54); wall (cel 2) pot (39, 32), wrench (19, 39); flung (cel 4, the landing) pot (18, 45), wrench (23, 52). The velocities and spins of section 7 are in the same manifest block and are read by the presenter from it.

**ENG2-X7: the flung launch tick.** The presenter does not see the animators, so it launches the flung pot and wrench on the first tick the observed goblin is no longer airborne (the landing), which is the tick `cf02::slammed_pose` starts returning Some. A test steps all three poses and checks that at most one pot is visible on every tick (the part, or ENG-01 frame 14 while a flung body is airborne), and that the pot is launched once.

**ENG2-X8: the evidence lab flag.** No Floor 1 pilot kills the Engineer with anything but a jab (the knockback finish), and none lets his ram run for more than a tick (a scan of the 90 aligned boiler and mastery pilots, 9 seeds each, found no frame 12, 20 or 21). `floor01_scene --death-lab-engineer NAME` (`crushed`, `decapitated`, `exploded`, `slammed_stomped`, `slammed_wall`, `slammed_flung`) makes the presenter play that finish for every Engineer defeat, as `--death-lab-edge` does for the llama. Presentation only: a window run ends on the same `COMBAT_STATE` hash as the headless run. The ram run is captured in the CF02 arena fixture `eng_jump_over_ram`.

**ENG2-X9: the V17 allowlist.** The three get-up key pockets are listed with the gate's rulings (section 5). The death sheet's frames 1, 4 and 5 and the parts sheet's `torso_lower`, `decal_wall_spatter` and `decal_wall_spatter_b` carry enclosed pockets that are inherited verbatim from GOB-02 A (the same pixels are enclosed in the source composite); each is listed with its source frame, as the gate required. Nothing the executor added to those three sheets creates a pocket. **The ENG-01 atlas runs no V17 (as the brief says), and its new frames do carry pockets of the kind ENG-01 already has:** frames 18 and 19 mirror the key pockets P1 to P3 (allowlisted under the keys grid); frame 20 has the 1 row thigh slits (27, 45), (28, 45) and (32, 45) and the walk 7 crotch gap (29 to 30, 46 to 48); frame 21 has the hem gap (23, 46). The hem and crotch gaps are inherited from frame 12, walk 6 and walk 7 (compare ENG-01 frames 0 and 2, `(23, 46)` and `(32, 46)`). None was filled.

**ENG2-X10: other config changes.** The parts config has a canvas margin of 0 (the potted `head` and `head_dead` and the `pot` fill their 32 px cell width) and its 28 px part cap is 32 for those three. `tests/test_art_eng01.py` skips the pot-identity check on frame 19 (the far fist covers a few pot pixels by design) and expects the wrench on frames 19 to 21. The test that greps for the removed finish id renamed `a_dive_from_the_perch_knocks_the_waiting_engineer_down` (a Rust test name in `src/cf02_fixtures/engineer_tests.rs`) to `..._to_the_floor`, because the name contained the string.
