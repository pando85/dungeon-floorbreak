# GOB-01: common goblin design spec

**Status:** design complete for keys (phases 1 and 2), 2026-11-17. The in-betweens and full atlases are next (Sonnet), followed by an independent Opus review. Nothing here is wired into Bevy.

**Decision record:** height **36 px**, frame **64 × 56**, pivot **(28, 55)**. The height was proposed by the designing Opus agent and **decided by a separate, independent Opus agent**, under the owner's rule that enemy sizes are proposed by one Opus agent and decided by another. The tick windows in section 4 are a **proposal** for the combat work order, because WORK-021 has no goblin attack yet.

Model routing (AGENTS.md "Model routing for art work"): design and keys by Claude Opus (this document); in-betweens, atlases, validators and evidence by Claude Sonnet; review by a separate Claude Opus agent. Rights: fan-concept art under the owner-reported `ART-PUBLICATION-ALL-V1` scope, with no reuse licence and no game or asset-pack distribution right. The concept references under `assets/concept/floor-01/` were used for mood only; no pixel is traced or derived from them.

Evidence: [docs/evidence/gob-01/](../evidence/gob-01/README.md).

## 1. Canon and tone constraints

From the [opening encounter handoff](../07-agents/floor-01-opening-encounter-handoff.md) and the [Floor 1 opening direction](floor-01-opening-catacomb-goblins.md):

- Common goblins are green, about 4 ft tall, in leathery rag clothing, with sharp teeth and pointy ears.
- The early on-foot weapon is a **pineapple on a wooden stick**. It is not a studded or metal club.
- The two common goblins are a **visual** variation (A and B), not two combat classes. Same frames, same timing, same boxes.
- Tone ([antihero tone](../01-design/antihero-tone.md)): a scrappy, nasty, slightly ridiculous threat with a readable tell, never a heroic orc. Carl must stay believable when he knocks one away.

## 2. Size study and decision

Carl is 52 px, treated as about 6 ft, so a 4 ft goblin is 52 × 4 / 6 = **34.7 px**.

| Option | Idle opaque height | Ratio to Carl | Implied height if Carl is 6 ft | Opaque area (idle) |
| --- | --- | --- | --- | --- |
| **h36 (decided)** | 36 px | 0.69 | 4.2 ft (about 4.4 ft upright) | 666 px |
| h40 | 40 px | 0.77 | 4.6 ft | 830 px |
| h44 | 44 px | 0.85 | 5.1 ft | 988 px |

Reference: Carl idle 52 px with an area of 837 px; Donut idle 40 px with an area of 1,305 px. One parametric idle pose is rasterized natively at each height (`scripts/author_goblin_gob01.py size-study`). These are block-ins, not production pixels.

Why 36 px:

- **Readability:** a big head, about the size of Carl's, so the face reads at 1x, and the biggest idle-to-tell change (36 to 55 px).
- **Threat:** 80 % of Carl's area, and the tell towers above him.
- **Hit box:** the idle fits the WORK-021 enemy box (24 × 38).
- **Knockback flight:** a 4 ft goblin punted about 90 px stays a believable shove.

Accepted trade-off: the goblin is 4 px shorter than Donut (whose 40 px includes her tiara).

## 3. Frame, pivot, boxes and atlas

- **Frame 64 × 56, pivot (28, 55)**, with the same bottom-row convention as Carl (48 × 56 / (25, 55)) and Donut (72 × 44 / (46, 43)). Pivot x is the midpoint between the idle soles; pivot y is the contact row.
- **Grounded tags** have their lowest opaque row on 55 (0 px foot sink): `idle`, `walk`, `attack_tell`, `attack_swing`, `attack_recover`, `hurt`, `down`, `getup` and `corpse`.
- **Airborne tags** (`knockback` and `knockback_contact`) have **no pixel on row 55**. The knockback key's lowest row is 46 and the contact key's is 44. Never snap them to the floor.
- **Standing box:** 24 × 38 (the WORK-021 enemy AABB); the proposed hurtbox is 22 × 34 on the body, excluding the ear tip and the weapon.
- **Knockback box:** while airborne (frames 18, 19 and 26), the physics body is a **28 × 18 box centred on the torso**. That is frame-space x 14 to 41 and rows 31 to 48, centred at (28, 40) and drawn with the torso centre there. It is used both for being hit and for collision with other goblins, walls and props (CP-02). On touchdown the runtime switches back to the standing box with the pivot on the floor.
- Authored facing right. The runtime mirrors around the pivot column. The variant B bandana tails hang down and back, so they read either way.
- **One atlas per variant: 27 frames** (0 to 26), row-major, with the layers `body` and `weapon`, plus `accessory` for B.

## 4. WORK-021 domain (what exists) and the tick contract (proposal)

From `src/prototype.rs` at 60 Hz:

- goblin health 2;
- AABB 24 × 38;
- chase at 62 px/s while 22 px < |dx| < 220 px;
- damage to Carl is **contact only** (|dx| < 23 and |dy| < 36), followed by 55 ticks of hero invulnerability;
- Carl's melee is an instant hit with -9 to 54 px of reach and a 14-tick cooldown;
- `GRAVITY` is 1050 px/s²;
- there is **no** knockback, stagger or goblin attack window yet.

Proposed contract for the combat work order (COMBAT-PHYS-01 / CP-01 / CP-02). It replaces contact damage with a telegraphed bonk:

| Phase | Ticks (tell start = 0) | Gameplay meaning |
| --- | --- | --- |
| tell | 0 to 23 (24) | trigger when \|dx\| ≤ 30, grounded, cooldowns at 0 and an **attack token** held; **interruptible** (a hit cancels the attack and goes to `hurt`) |
| swing smear | 24 to 25 (2) | hitbox inactive |
| swing impact | 26 to 29 (4) | **hitbox active**: x +8 to +30 px forward of the pivot, y 0 to 30 px up; at most one hit per attack |
| recover | 30 to 49 (20) | vulnerable punish window |
| attack cooldown | 40 ticks in idle or walk | before the next tell |
| hurt | 12 | non-lethal hit; then a **20-tick hurt cooldown** before any new tell (walking is allowed) |
| knockback | launch 4, then airborne until touchdown | lethal or heavy hit. Reference launch: vx 200, vy 240 px/s gives about 27 ticks and about 90 px |
| knockback contact | 3 | starts on the tick the flying goblin's knockback box first overlaps another goblin's box; then airborne again, or down if grounded |
| down | 4 + 8 + hold | if alive, hold 24 ticks then get up (18); if defeated, hold 30 ticks then corpse |

**Attack token (group rule for the combat work order):**

- At most **2 goblins** may be in `attack_tell` or `attack_swing` at once.
- A goblin takes a token to start its tell, and two tell starts are at least **8 ticks** apart.
- The token is returned when the swing ends or the attack is cancelled.
- Token grants are deterministic: the lowest distance to Carl wins, ties go to the lower entity id.
- Goblins without a token keep walking or idle at 34 to 60 px from Carl.

**Alignment rule:**

- Every frame lasts a whole number of ticks.
- The atlas `duration_ms` is `round(ticks * 1000 / 60)` and is used for previews only.
- The runtime advances frames from simulation ticks and state, never from wall-clock time.
- Frame 13 is shown on exactly the hitbox-active ticks.
- Animation never feeds back into the simulation.

## 5. Animation list (27 frames per variant)

The keys are **done** (Opus). Every other frame is Sonnet's, following section 9.

| # | Tag | Frame | Ticks | Playback | Author |
| --- | --- | --- | --- | --- | --- |
| 0 | `idle` | hunched breath, stick forward | 12 | loop | **key** |
| 1 | `idle` | breath in | 8 | loop | Sonnet |
| 2 | `idle` | hold, ear twitch | 12 | loop | Sonnet |
| 3 | `idle` | breath out | 8 | loop | Sonnet |
| 4 | `walk` | contact, near foot forward | 5 | loop | **key** |
| 5 | `walk` | down | 5 | loop | Sonnet |
| 6 | `walk` | pass | 5 | loop | Sonnet |
| 7 | `walk` | contact, far foot forward | 5 | loop | **key** |
| 8 | `walk` | down | 5 | loop | Sonnet |
| 9 | `walk` | pass | 5 | loop | Sonnet |
| 10 | `attack_tell` | lead-in | 6 | once | Sonnet |
| 11 | `attack_tell` | wind-up hold | 18 | once | **key** |
| 12 | `attack_swing` | smear: stick vertical, pineapple overhead, arc trail | 2 | once | **key** |
| 13 | `attack_swing` | impact: overhead bonk | 4 | once | **key** |
| 14 | `attack_recover` | follow-through, pineapple on the floor | 12 | once | Sonnet |
| 15 | `attack_recover` | yank back | 8 | once | Sonnet |
| 16 | `hurt` | recoil | 4 | once | **key** |
| 17 | `hurt` | stagger | 8 | once | Sonnet |
| 18 | `knockback` | launch | 4 | once | Sonnet |
| 19 | `knockback` | airborne | until touchdown | hold | **key** |
| 20 | `down` | floor impact | 4 | once | Sonnet |
| 21 | `down` | settle | 8 | once | Sonnet |
| 22 | `down` | lying, dazed | hold | hold | **key** |
| 23 | `getup` | push up | 8 | once | Sonnet |
| 24 | `getup` | kneel, grab the stick | 10 | once | Sonnet |
| 25 | `corpse` | lying, X eyes, the pineapple rolled 6 px off the stick | hold | hold | **key** |
| 26 | `knockback_contact` | crumpled squash on hitting another goblin | 3 | once | **key** |

**Walk:** 6 × 5 ticks is a 30-tick cycle. At the 62 px/s chase speed that is 31 px per cycle and 15.5 px per step. The planted foot travels 5 px per frame (60 px/s). The keys put the stride at about ±8 px around the pivot.

## 6. State diagram

```mermaid
stateDiagram-v2
    [*] --> idle
    idle --> walk: Carl within 220 px
    walk --> idle: out of range or 22 px or closer
    idle --> attack_tell: |dx| <= 30, cooldowns 0, token
    walk --> attack_tell: |dx| <= 30, cooldowns 0, token
    attack_tell --> attack_swing: tick 24
    attack_swing --> attack_recover: tick 30
    attack_recover --> idle: tick 50 (attack cooldown 40, token returned)
    attack_tell --> hurt: hit (attack cancelled, token returned)
    idle --> hurt: non-lethal hit
    walk --> hurt: non-lethal hit
    attack_recover --> hurt: non-lethal hit
    hurt --> idle: 12 ticks (then 20-tick hurt cooldown)
    idle --> knockback: lethal or heavy hit
    walk --> knockback: lethal or heavy hit
    attack_tell --> knockback: lethal or heavy hit
    attack_recover --> knockback: lethal or heavy hit
    hurt --> knockback: lethal hit
    knockback --> knockback_contact: knockback box meets another goblin
    knockback_contact --> knockback: 3 ticks, still airborne
    knockback_contact --> down: 3 ticks, grounded
    knockback --> down: touchdown
    down --> getup: alive, after 24 ticks
    getup --> idle: 18 ticks
    down --> corpse: defeated, after 30 ticks
    corpse --> [*]
```

`attack_swing` cannot be interrupted. `knockback`, `knockback_contact`, `down` and `corpse` have no hurtbox for Carl's melee; the knockback box still collides with goblins and props. Corpse contact (CP-04) stays presentation-only, as in option A.

## 7. Palette `goblin-floor01-v0` (at most 20 colours per atlas)

A uses 19 colours and B 20. The colours are tuned for the A1 lighting (warm torch light falling into cold blue ambient).

| Key | Name | Hex | Role |
| --- | --- | --- | --- |
| K | outline_plum | `#1b1124` | shared with Carl |
| a / s / m / l | skin deep / shade / mid / light | `#263a26` `#46682e` `#7a9e36` `#b2cc56` | yellow-green skin; cool shadows, warm lights |
| r / t / u | rag dark / mid / light | `#2c1e28` `#4c3436` `#73524a` | cool, dark leather that separates from the orange brick |
| e | eye_yellow (accent) | `#ffd23a` | glaring eyes |
| w | tooth_white (accent) | `#efe4c4` | teeth; eye whites in hurt, knockback and down |
| o | mouth_dark | `#5c1426` | |
| R | red_mid | `#b8322e` | tongue; variant B bandana |
| x / y | wood dark / mid | `#4a2c1c` `#8c5c32` | stick, rope belt |
| F / G / H | pineapple dark / mid / light | `#7a3c12` `#d08826` `#f4c84a` | checker-pattern fruit; H is an accent (single facet highlights, and the smear arc) |
| V / v | leaf dark / mid | `#1d4a3c` `#3e8a5c` | teal crown, kept off the skin hue |
| B | rag_red_dark (variant B only) | `#6e1a22` | bandana shadow |

## 8. Key poses (as authored)

- **0 idle (accepted, polished).** Hunched, with a big bald head ahead of the leather-vest hump. A long ear sweeps back. Glaring yellow eyes, a hooked nose and an upper row of teeth. The stick is held forward with the pineapple upright at chest height. Polish: a `u` rim on the hump top and 2 notches in the vest hem.
- **4 / 7 walk contacts.** The idle upper body sits 1 px lower. The leading foot strikes with the heel on row 55 and the trailing foot pushes off its toes; frame 7 mirrors the legs. The stride is about ±8 px around the pivot.
- **11 tell (accepted, polished).** The torso leans back with a braced stance. The arm rises from a thickened shoulder at about (24, 30); a 2 × 2 far hand sits higher on the stick. The pineapple is the highest point (row 1, 21 px above the head), and the mouth is open in a yell.
- **12 smear.** The stick is vertical with the pineapple overhead at x 26 to 31 and rows 2 to 10, held by both arms. A 1 px `H` arc runs from behind the head, over the top, to the front. Squeezed eyes and clenched teeth.
- **13 swing impact (overhead bonk).**
  - The hump is high and the head is down under the arms.
  - Both arms reach forward-up, with the far arm visible in `s`. The fists are at x 38 to 41, rows 25 to 28.
  - The stick angles 37 degrees down and forward. The pineapple's centre is at about (51, 33), with the crown leading at x 53 to 57, rows 35 to 39 (Carl's hip and belly).
  - The front foot is on row 55 at x 36 to 41, and the back leg pushes off its toes.
  - Squeezed eyes and clenched teeth.
- **16 hurt.** The torso leans 3 px further back, and the head is thrown back with the chin up (crown 1 px back, jaw 1 px forward). White eyes and an O mouth. The front heel is up, so only the toe touches row 55. The near arm flings up and back with the hand above the shoulder, and the stick trails down-back to the pineapple.
- **19 knockback.**
  - The torso is tilted about 20 degrees, with the head low on the side away from Carl (left) and the feet high toward him.
  - A **new thrown-back profile head**: chin up, nose spike up, a 2 × 2 eye white with a `K` pupil, an open mouth with a 2 × 3 `R` tongue, and the ear swept back.
  - The thighs are 3 px wide, the knees bent, the feet 3 × 2.
  - The near arm is flung toward Carl, and the pineapple trails back over the body.
  - Compact: 34 × 31 px, lowest row 46, with the head as the clean leading edge.
- **22 down.** Lying on the back, head left and face up, eyes dazed, mouth open. The knees are up, the arm has flopped onto the stick, and the pineapple is still attached.
- **25 corpse.** As 22, but with X eyes and the tongue out. The pineapple has come off and rolled 6 px away; the stick lies bare.
- **26 knockback contact.** The knockback pose crumpled into a ball (27 × 34 px, airborne, lowest row 44). The head is squashed against the side it hit, the eyes are shut, and the knees and arm are bunched with the pineapple up.

## 9. In-between list for Sonnet (no design decisions needed)

General rules:

- Copy from the neighbouring keys; translate whole parts, and do not resample or rotate pixels except by 90 degrees.
- Keep the palette, the pivot (28, 55) and the layer split (weapon keys on `weapon`).
- Grounded frames end on row 55; airborne frames stay off it.
- Run the lint and the validators in section 10 on every frame.

| # | Pose notes |
| --- | --- |
| 1 | Frame 0 with everything above row 47 (head, vest, arm, stick, pineapple) up 1 px; legs and feet unchanged; shorts hem stretched 1 px |
| 2 | Frame 0 unchanged except the ear tip: move the outermost 3 ear pixels up 1 px (twitch); add `K` behind them |
| 3 | Frame 0 with the head only (rows 20 to 33) down 1 px; jaw outline merges into the vest collar |
| 5 | Frame 4 upper body 1 px lower (2 px below idle); near (front) foot flat on row 55, 5 px behind its frame-4 heel position; trailing far foot lifted 2 px, toe still down |
| 6 | Frame 4 upper body at idle height (1 px above frame 4); near foot planted under the pivot (x 28 to 36); far foot passing at rows 51 to 53, x 24 to 30, behind the near shin; pineapple 1 px higher |
| 8 | Frame 7 with the same changes as 5, legs swapped (far foot flat, near foot trailing) |
| 9 | Frame 7 with the same changes as 6, legs swapped |
| 10 | Halfway from 0 to 11: torso and head 1 px back; arm raised to shoulder height; the stick points up-back at 45 degrees; the pineapple at x 10 to 18, rows 14 to 22; mouth half open (one `o` row) |
| 14 | Follow-through: frame 13's body 2 px lower and 1 px further forward; arms straight down-forward; **pineapple on the floor**, crown leading, lowest row 55, centre x 46 to 50; stick from the fists at about (38, 40) down to it; face unchanged |
| 15 | Halfway from 14 to 0: torso upright, stick pulled back across the body, pineapple at x 40 to 46, rows 36 to 44; eyes open (yellow) again |
| 17 | Halfway from 16 to 0: head back to 2 px behind idle with no shear; eyes still white; arm lowered to shoulder height; front heel down on row 55; pineapple hanging low behind at x 2 to 10 |
| 18 | Launch, halfway from 16 to 19: body tilted 45 degrees back, feet leaving the floor (lowest row 53); head is the frame-19 profile head turned to face up-right; tongue not yet out (`o` mouth); arm flung forward |
| 20 | Floor impact: frame 22 squashed 2 px (body rows 1 px lower, knees 1 px lower); a 1 px squash wider on both ends; eyes shut |
| 21 | Settle: frame 22 with the head lifted 1 px and the far knee 1 px lower; eyes dazed |
| 23 | Push up: torso rotated to about 45 degrees (sitting up), head an upright idle head (dazed eyes, a `K` line over `w`); arms behind as props; stick on the floor |
| 24 | Kneel: frame 0's upper body 4 px lower, the near knee on the floor at (34, 55), far foot planted; the fist closing on the stick, pineapple on the floor in front |
| All 27, variant B | Copy the A frame and draw the `accessory` layer: bandana (2-row `R`/`B` band over the forehead, knot at the back of the skull, two tails: one `R` down and back, one `B` hanging lower), ear notch (2 `K` pixels across the ear 2 px from the tip), missing front upper tooth (`o`), and a 3 × 3 vest patch (`uuu` / `uKu` / `uKu`). Translate the overlay with the head and vest of each frame (offsets used by the keys: 0/1/2/3 as frame 0; walk +1 row; 10 and 11 +2,+2; 12 +3,+3; 13 +1,+9; 14 and 15 interpolated; 16 and 17 follow the head; profile-head frames as frames 19, 22, 25 and 26). Body-coloured tidy pixels may go on `accessory`; the B `body` and `weapon` cels stay identical to A |

## 10. Validators and readability rules

All of these are automated in `tests/test_art_goblin_gob01.py` for the keys. Extend them to all 27 frames per variant.

- **R1 Tell.** In `attack_tell` the pineapple is the highest opaque point, at least 12 px above the idle head top (row 20); key 11 reaches row 1. **Amended:** the pineapple may also be above the head in frame 12 (the smear), and nowhere else among the grounded frames.
- **R2 Tell body.** The mouth is open (at least 4 × 2 dark pixels) and the torso leans back at least 2 px.
- **R3 Idle and walk.** The pineapple is in front at chest height, below row 20, with at least a 3 px gap to the nose.
- **R4 Impact.** The pineapple's centre is at x 50 to 53, rows 31 to 37, at least 16 px ahead of the pivot; the front foot is on row 55 at x 36 to 41; the frame shows on the active ticks only.
- **R5 Hurt.** Eye whites (`w`) and no yellow eyes; the front heel is lifted with only the toe on row 55. A hurt goblin never reads as a tell.
- **R6 Airborne.** `knockback` and `knockback_contact` have no pixel on row 55. The knockback air frame is at most 36 px wide and at least 22 px tall, its lowest row is 50 or above, and the head is the leading edge (no weapon pixel in the leftmost column).
- **R7 Cast separation.** Goblin green appears only on goblins. Check at 1x in front of both the warm brick and the cold arch.
- **R8 Contact and cleanliness.** Grounded frames end on row 55. Lint has **0 isolated pixels** and **fewer than 10 orphan pixels per frame**. `e`, `w` and `H` are accents; `K` pupils and brows count, so keep them few.
- **R9 Variant B** reads at 1x from the red bandana alone. The B `body` and `weapon` cels equal A's.
- **R10 Tone.** Nasty and slightly ridiculous (grins, yells, a lolling tongue, a pineapple), never a heroic-orc physique. Carl's knockback of a goblin must read as a believable shove.
- **Palette:** at most 20 colours per atlas (A 19, B 20). Layers are disjoint, and weapon keys live on `weapon`.

## 11. Evidence list (for the Sonnet execution PR)

- Per-tag GIFs at x3 and x6 (A and B), from tick durations.
- A contact sheet of all 27 frames per variant, at 1x and x6, with the contact row and pivot marked.
- A lit Floor 1 composition with **3 A and 2 B goblins** around Carl at 640 × 360 and 1920 × 1080.
- Tell and hurt side by side at 1x (A and B).
- A **CP-02 strip**: kick, launch, flight, contact, the second goblin hurt, down, with tick numbers.
- A layout, pivot and timing diagram: the frame grid with the pivot, standing box, knockback box, the hitbox on frame 13, and a tick bar for the attack (24 + 2 + 4 + 20) and the cooldowns.
- `measurements.json` and `SHA256SUMS`.

Phase 2 updates the key-pose sheet and the lit key scene only.

## 12. Files

| File | Role |
| --- | --- |
| `assets/source/characters/goblin/goblin_a_h36_keys.grid.json` / `.ase` | variant A keys (11 of the 27 frames: atlas frames 0, 4, 7, 11, 12, 13, 16, 19, 22, 25 and 26), layers `body` and `weapon` |
| `assets/source/characters/goblin/goblin_b_h36_keys.grid.json` / `.ase` | variant B keys: A's cels plus `accessory` |
| `assets/source/characters/goblin/goblin_size_study.grid.json` / `.ase` | block-ins at 36, 40 and 44 px |
| `assets/manifests/goblin_a_h36_keys.json`, `goblin_b_h36_keys.json`, `goblin_size_study.json` | candidate manifests (`prototype_fixture_unapproved`); `keys.atlas_frames` maps grid frames to atlas frames |
| `scripts/author_goblin_gob01.py` | palette, layer split, size-study generator |
| `scripts/make_goblin_gob01_review.py` | size-study sheet, key-pose sheet (A and B), lit Floor 1 compositions |
| `tests/test_art_goblin_gob01.py` | contract tests for the sources and the R1 to R9 rules on the keys |
