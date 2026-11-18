# ENG-01: Goblin Engineer readability set, design spec

**Status:** execution done. Spec written; hero key 0 (`idle`) frozen and the silhouette gate passed; keys 3, 10, 12, 13, 14 and 17 (frozen) and all in-betweens drawn (18 frames, 7 keys); validator PASS (15 pass, 1 skipped), evidence and review pack committed. Independent review: pass with execution items (19/25), items applied (notes 13 to 17).

Sections follow `docs/02-art/bard-01-spec.md` (a goblin variant on the GOB-01 base) and `docs/02-art/trog-02-spec.md` (executor detail). Coordinates are frame pixels (x right, y down, frame 64 x 56) unless a table says "from the pivot" (x forward, y up).

## 1. Canon and tone constraints

### Approved concept

- Only one committed concept image shows the Engineer: `assets/concept/floor-01/opening/02-goblin-trio-reference-crop.png` (1080 x 365), right panel, caption "ENGINEER / LEVEL 3". `assets/concept/floor-01/opening/README.md` (table, row `02-goblin-trio-reference-crop.png`) records it as human-approved broad visual direction. `docs/02-art/floor-01-enemy-concept-roster.md` (row "Goblin Engineer", line 12) records the pot-helmet engineer as owner-approved, with "dedicated editable engineer/driver art and animation not yet verified as completed".
- The trio image has no pixel heights printed on it. The "44 px" figure cited by other specs is second-hand (see section 3).

### Canon citations (file and line)

| Fact | Source |
| --- | --- |
| Level 3, a cooking-pot helmet, a more tool-focused silhouette, visibly operates the Dozer. A wrench is a non-canonical visual shorthand; ranged bombs, buffs, in-combat repairs and class perks need a later gameplay decision | `docs/02-art/floor-01-opening-catacomb-goblins.md:50` |
| Concept use: green goblin shapes, expressions and the engineer's pot helmet; generated weapons, classes and pixel sizes are not accepted | `docs/02-art/floor-01-opening-catacomb-goblins.md:101` |
| Engineer (1): kitchen-pot helmet, level 3, operates the Dozer | `docs/07-agents/floor-01-opening-encounter-handoff.md:46` |
| Rider identity: a level-3 engineer in a kitchen-pot helmet sits at the rear controls in an upside-down saucepan, its handle sticking out back | `docs/02-art/doz-01-dozer-spec.md:33` |
| One hand on the throttle lever, the other arm pointing at Carl; has a lever-pull key | `docs/02-art/doz-01-dozer-spec.md:34` |
| Rider sprite `dozer_riders_h36` / `engineer_*`, frame 64 x 56, seat pivot (28, 55), 7 frames (point, jab, idle 0 and 1, lever pull, lurch back, lurch forward) | `docs/02-art/doz-01-dozer-spec.md:91` |
| `engineer_point` construction: GOB-01 idle body seated (rows shifted down 9, no pineapple), saucepan 6 rows deep and 14 px wide over a 1 px lip, handle out back (fix round D7) | `docs/02-art/doz-01-dozer-spec.md:211`, `:271` |
| Other engineer frames: lever pull (line 220), lurch back (line 222, pot hops 3 px, white eyes), lurch forward (line 224, pot slides 3 px over the eyes); visible rider rows are GOB-01 pixels translated whole, apart from the pot and arms | `docs/02-art/doz-01-dozer-spec.md:239` |
| Common goblins: green, about 4 ft, leathery rag clothing, sharp teeth, pointy ears; early on-foot weapon is a pineapple on a wooden stick, not a club; tone is a scrappy, nasty, slightly ridiculous threat with a readable tell, never a heroic orc | `docs/02-art/gob-01-goblin-spec.md:11-19` |
| No bombs in the opening trio: the opening goblins use pineapples on wooden sticks; the Engineer is not automatically a mine-layer or turret | `docs/02-art/bard-01-spec.md:21`, `:224` |
| The opening goblins carry pineapples on sticks; the board's spiked club is not approved for them; the Engineer is not turned into a turret or mine-layer | `docs/02-art/chop-01-spec.md:47` |
| The engineer is distinct from the Bard's performance silhouette | `docs/02-art/bard-01-spec.md:14` |
| Chapter 15's larger goblin engineering and workshop facilities are a different, later area: do not move their clutter or machinery into the opening encounter | `docs/02-art/floor-01-opening-catacomb-goblins.md:80` |
| Production order: batch 1, "Goblin Engineer standalone (delta of GOB-01 / DOZ-01 rider)" | `docs/07-agents/pixel-art-pipeline.md:43` |

### Tone

`docs/01-design/antihero-tone.md` applies: a believable, grimy, slightly ridiculous threat with a readable tell, not a heroic mechanic and not a cute mascot. Everything on the board that reads as "class", "level" or "role" is a label, not a mechanic.

### Canon facts that must survive in pixels (reading of the concept and the rider)

- Green goblin skin with the GOB-01 head construction: pointy ears, yellow eyes (palette key `e`, as the whole goblin family), sharp teeth, snarl.
- An upside-down kitchen saucepan jammed on the head, handle sticking out back (to the left in the concept and in the rider).
- Tool-focused kit visible on the concept figure: a wrench, a grey-blue pauldron on the near shoulder, a wrist wrap, a belt with a buckle, a rag tunic and trousers. Which of these are canon and which are board flourish is resolved in section 12.
- Same family as GOB-01 A/B: no pineapple on a stick, no spiked club, no bombs.

## 2. NOT approved (do not derive from the boards)

- The captions "ENGINEER / LEVEL 3", "CONCEPT ONLY", "NO GAMEPLAY SPRITE" and any printed class, role or level text. The level-3 fact is canon from the handoff, not a stat.
- Any printed or implied size, including the second-hand "44 px".
- The wrench as a weapon or an attack (`floor-01-opening-catacomb-goblins.md:50`: "a non-canonical visual shorthand"). In this set it is a stowed prop only.
- Any mechanic: ranged attacks, turret or mine-layer roles, bombs, buffs, repairs in combat, class perks, drops. The ram in section 4 is a **placeholder** melee for the readability set, not an approved behaviour.
- The spiked clubs of Goblins A and B on the same board, and the pineapple stick (canon for the A/B goblins, not stated for the Engineer).
- The board's pose, lighting and rendering. Crops are reference for identity only, never traced or exported.

## Difficulty and route

**Score 0 of 4: Easy, route C** (variant: the spec and one hero key, idle key 0, are drawn by the design stage; a silhouette gate checks key 0 at 1x; the execution stage draws the other keys and the in-betweens from sections 7, 8 and 14).

| Point | Met | Note |
| --- | --- | --- |
| 1. New design, not a variant | no | GOB-01 body below the chin and the DOZ-01 rider's pot and head, pixel for pixel |
| 2. Native height above about 40 px | no | 40 px exactly (section 3) |
| 3. Complex construction | no | rigid part blocks (pot, head, torso, legs, wrench); no rotation, no rescale. The hurt and down bodies are GOB-01 frames 16 and 20 to 22 |
| 4. VFX, projectiles, special states | no | no projectile, no spark; the ram impact flash is left to the VFX order. The pot hop and slide are sprite poses taken from the rider |

## 3. Size decision, frame, pivot and boxes

**Decision: option B, 40 px idle opaque height** (rows 16 to 55), 0.77 of Carl, level with Donut and the Bomb Bard (40 px).

- The rider is the GOB-01 head on a 36 px body; its pot sits 4 rows above the GOB-01 crown (rider pot top row 25 = GOB-01 row 16 after the 9-row seat offset). Standing that same individual up gives exactly 36 + 4 = 40 px with **no** body change. That is the reason for B: the standalone Engineer is the rider pixel for pixel from the rim to the chin, and GOB-01 A pixel for pixel below the chin.
- A (36 px) would cut the pot or shrink the body: a different, smaller goblin from the one on the Dozer. C (44 px) would need a scaled-up body (a bigger goblin than the rider) and rests on an unverified printed number (question 1).
- Level 3 reads through the pot, the belt kit and the stowed wrench, not through extra body size. All opening goblins stay "about 4 ft"; the pot is what makes him the tallest of the trio on screen, as on the board.

Frame **64 x 56**, pivot **(28, 55)**, contact row **55** (same as GOB-01 A/B and the Bard, so all three swap in the same slot). Authored facing right; the runtime mirrors around the pivot column, so the pot handle and the wrench always trail. Two disjoint layers: `body` and `prop` (the wrench).

Key 0 extent: x 13 to 44 (handle tip to snout), rows 16 to 55.

Boxes are **placeholders** in px from the pivot (x forward, y up), re-measured by execution from the drawn frames. Hurtboxes include the pot and head, leave out the handle beyond x -9 and the wrench:

| Box | x | y | Frames |
| --- | --- | --- | --- |
| Standing (opaque extent) | -15 to 16 | 0 to 39 | 0 (measured) |
| Hurtbox idle/walk | -9 to 14 | 0 to 37 | 0 to 8 |
| Hurtbox tell | -11 to 12 | 0 to 38 | 9, 10 (the pointing arm is not a hurtbox) |
| Hurtbox ram | -6 to 22 | 0 to 33 | 11, 12 |
| Hurtbox recover | -7 to 16 | 0 to 35 | 13 |
| Hurtbox hurt | -16 to 10 | 0 to 33 | 14 (the hopping pot is not a hurtbox) |
| Hurtbox down | -14 to 26 | 0 to 12 | 15 to 17 |
| Ram hitbox (impact frame only) | 14 to 24 | 14 to 33 | 12: the pot front and the snout |

## 4. Readability-set animation list

18 frames, 7 keys. Ticks at 60 Hz. The Engineer is a scrappy foreman, not a brawler: the tell is the rider's own gesture, pointing at Carl (DOZ-01 `engineer_point`), and the attack is a short head-down ram that leads with the saucepan. **Attack choice:** the most conservative reading. The concept shows no attack, only a held wrench that canon calls a non-canonical shorthand, so a wrench strike is not justified. The pot is the one canon object he owns, and a ram uses it with no new weapon, no projectile and nothing from another goblin type (A/B swing pineapples, the Bard throws bombs). Placeholder for the owner (section 13).

| Role | Tag | Frames | Playback | Ticks | Notes |
| --- | --- | --- | --- | --- | --- |
| idle | `idle` | 0 to 2 | loop | 14 + 10 + 14 = 38 | breath and a pot rattle |
| move | `walk` | 3 to 8 | loop | 6 x 6 = 36 | contact plan in section 8: 4 px per frame, **40 px/s** (placeholder; GOB-01 chases at 62 px/s) |
| tell | `attack_tell` | 9, 10 | once | 8 + 24 = 32 | the 24-tick point-and-rear hold is the read window |
| attack | `attack_ram` | 11, 12 | once | 3 + 6 = 9 | hitbox on frame 12 only |
| (recover) | `attack_recover` | 13 | once | 18 | pot slid over the eyes: the punish window |
| hurt | `hurt` | 14 | once | 12 | pot hops off the head |
| defeat | `down` | 15 to 17 | hold | 5 + 10, then a state-driven hold | the pot lands over his face |

**Tell-to-attack read:** tick 0 starts the tell. The rear (9) runs ticks 0 to 7, the point-and-rear hold (10) ticks 8 to 31. The launch (11) runs 32 to 34, the impact with the hitbox active (12) 35 to 40, the recover (13) 41 to 58. The point pose (a long level arm at shoulder height, head and pot pulled back) must hold for at least 24 ticks (400 ms) and must not resemble any other frame; the ram (12) is the lowest standing frame and the only one with the pot ahead of the snout line. Reach, damage and knockback are placeholders.

| # | Tag | Pose | Ticks | Author |
| --- | --- | --- | --- | --- |
| 0 | `idle` | **KEY (drawn, frozen)** GOB-01 stance, rider pot and head, near fist closed at the belly, wrench stowed at the back hip | 14 | key |
| 1 | `idle` | breath: POT, HEAD and TORSO rows down 1 (legs fixed) | 10 | in-between |
| 2 | `idle` | pot rattle: key 0 with the POT block only up 1 (a 1-row gap of `K` rim stays closed, see 14) | 14 | in-between |
| 3 | `walk` | **KEY** contact A: near foot planted ahead, far foot at the end of its stance, body low | 6 | key |
| 4 to 8 | `walk` | section 8 contact plan | 6 each | in-between |
| 9 | `attack_tell` | rear: halfway from 0 to 10, arm raised to about 45 degrees forward | 8 | in-between |
| 10 | `attack_tell` | **KEY** point and rear: near arm straight at Carl at shoulder height, head and pot pulled back | 24 | key |
| 11 | `attack_ram` | launch: halfway from 10 to 12, arm swinging back | 3 | in-between |
| 12 | `attack_ram` | **KEY** ram impact: pitched forward, pot leading, low | 6 | key |
| 13 | `attack_recover` | **KEY** dazed: upright, pot slid down over the eyes, arms out for balance | 18 | key |
| 14 | `hurt` | **KEY** recoil: GOB-01 hurt body, the pot hopping off the head, white eyes | 12 | key |
| 15 | `down` | fall beat: GOB-01 frame 20 body, the pot in the air above the head | 5 | in-between |
| 16 | `down` | floor impact: GOB-01 frame 21 body, the pot 1 px above its landing place | 10 | in-between |
| 17 | `down` | **KEY** on his back (GOB-01 frame 22 body), the pot landed rim-down over his face, ear and feet sticking out | hold | key |

## 5. State diagram

```text
idle <-> walk -> attack_tell (9, 10) -> attack_ram (11, 12) -> attack_recover (13) -> idle
any grounded state -> hurt (14) -> idle | down (15 -> 16 -> 17 hold)
```

## 6. Palette `eng-floor01-v0` (17 of 20)

Every colour is reused with the same key and hex: no new colour. Pot, buckle and wrench share the rider's iron ramp, so the engineer's metal reads as one kit, cooler and lighter than the brick and torch light; the green and rags are the family's.

| Key | Name | Hex | Source |
| --- | --- | --- | --- |
| `K` | outline_plum | `#1b1124` | goblin-floor01-v0 |
| `a` `s` `m` `l` | skin deep, shade, mid, light | `#263a26` `#46682e` `#7a9e36` `#b2cc56` | goblin-floor01-v0 |
| `r` `t` `u` | rag dark, mid, light | `#2c1e28` `#4c3436` `#73524a` | goblin-floor01-v0 |
| `e` | eye_yellow (accent) | `#ffd23a` | goblin-floor01-v0 |
| `w` | tooth_white (accent) | `#efe4c4` | goblin-floor01-v0 |
| `o` | mouth_dark | `#5c1426` | goblin-floor01-v0 |
| `2` `3` `4` `5` `6` | iron ramp | `#2c3046` `#3f445e` `#575e7a` `#78809a` `#a3abbf` | DOZ-01 rider |
| `7` | iron_glint (accent) | `#dde2ec` | DOZ-01 rider |

Dropped from goblin-floor01-v0: wood `x`/`y`, pineapple `F`/`G`/`H`, leaf `V`/`v` (no stick). Three slots stay free; execution adds none without a spec change.

## 7. Key poses

Part blocks of key 0 (`scripts/author_eng01.py`, inclusive x0, y0, x1, y1): `POT` (13, 16, 42, 24) saucepan, handle and rim; `HEAD` (16, 25, 44, 33) ear tip to chin; `TORSO` (19, 30, 41, 45) cape, belly, near arm and fist, belt and rag skirt; `LEGS` (19, 46, 38, 55); `WRENCH` (14, 35, 18, 42), layer `prop`. "Offset (dx, dy)" moves a block whole. Blocks are never rotated or rescaled. Each key must meet its section 9 bounding-box targets; joints may move by up to 2 px to meet them, and the target wins (record the change in section 14).

| Frame | Tag | Silhouette and pose | Readability reason |
| --- | --- | --- | --- |
| 0 | `idle` | **Drawn, frozen.** Rows 16 to 55, x 13 to 44. Rows 16 to 25: rider `engineer_hold` rows 25 to 34 (pot 14 px wide over a 1 px lip, glint `7` at (29, 19), handle to x 13). Rows 26 to 55: GOB-01 A frame 0 body. Near fist closed at (35 to 41, 37 to 40). Ear tip pushed out under the handle (16 to 22, 24 to 27). Belt on row 42 with a `56` buckle at x 31, 32. Wrench jaw-up at the back hip, shaft into a belt loop at (17, 42). | The pot is the largest light shape on a 40 px goblin; the handle trails, the wrench notches the back. Nothing else on Floor 1 has this head. |
| 3 | `walk` | Top row 18. POT, HEAD, TORSO, WRENCH offset (+1, +2). Legs redrawn as 2 to 3 px shins from the hip row (47) to the feet; near foot (light ramp, 9 px outline on row 55) planted with its row-55 right end at x 39; far foot (deep ramp, 6 px outline) at its stance end, right end x 27. Near fist offset (-2, 0) (arm swings opposite the near leg). | Longer stance, lower head: a stomping advance. |
| 10 | `attack_tell` | Top row 15, left edge x 10 or less, right edge x 57 or more. HEAD and POT offset (-3, -1); TORSO offset (-2, 0) with the near arm replaced by the rider's pointing arm: copy `dozer_riders_h36` frame 0 (`engineer_point`) rows 39 to 44, x 39 to 60, to rows 30 to 35 shifted -2 x (hand and finger end at x 58). Legs braced: near foot right end x 40, far foot right end x 25 (both on row 55). Mouth: the key-0 snarl. | One long level arm at Carl: "you!". Same gesture as on the Dozer, so the player learns it once. |
| 12 | `attack_ram` | Top row 21 to 23, right edge x 51 or more, bottom row 55. HEAD and POT offset (+8, +6): pot top row 22, rim front x 50, snout x 52. TORSO redrawn as a rag sack along a spine from the hips (27, 45) to the neck (37, 37), about 40 degrees forward; copy the cape's `u`/`t`/`r` banding along it. Near arm swung back along the side, fist at about (24, 41). Near leg braced forward, foot right end x 42; far leg kicked back, foot right end x 22, heel lifted 1 (only the toe pixels on row 55). Mouth closed to the `w` tooth row (teeth gritted). WRENCH offset (+2, +3). | The only frame with the pot ahead of the snout line, and the lowest standing head: charge. Contrasts with 10 at once (tall and back to low and forward). |
| 13 | `attack_recover` | Top row 19 to 21. HEAD offset (+2, +1); POT offset (+3, +4) (the rider's lurch-forward slide: the rim lip now covers the eye row, so no `e` shows). Both arms out for balance: the near fist at about (44, 37), a 2 px far hand showing behind the cape at about (17, 34). Legs: key 0. Mouth slack: the key-0 jaw with the `w` tooth row dropped 1. | Blind and wobbling: "hit him now", never "about to ram again". |
| 14 | `hurt` | Top row 13 to 15. GOB-01 A frame 16 body (layer `body` only; head offset from key 0 measured (-8, +1) by the eyes). POT offset (-8, -2): hopped 3 px off the crown, which shows under it. If the handle touches the raised hand (rows 21 and 22), hop 1 more (top row 13). Eyes white as in frame 16, no `e`. Belt row and WRENCH follow the GOB-01 frame 16 hips (about (-8, +2)). | Pot in the air over a recoiling body: unmistakably a hit, and the rider's lurch-back gag. |
| 17 | `down` | Top row 39 to 41 (the ear tip), width 54 or less. GOB-01 A frame 22 body offset (+10, 0). POT offset (-10, +31): rim lip on row 55, pot x 16 to 32 over the face, handle to x 3. The ear (x 25 to 29, rows 40 to 46) sticks up behind the pot; the feet stay up at x 43 to 54. No `e`, no `w` (face covered). WRENCH: none (dropped under him; prop layer empty in 15 to 17). | A flat goblin with a saucepan clanged down over his face: defeated, and still the Engineer. |

## 8. In-between list and walk contact plan

Blend the nearer key's blocks; no design decisions beyond these notes.

| # | Pose notes |
| --- | --- |
| 1 | Idle breath: POT, HEAD, TORSO and WRENCH offset (0, +1); legs fixed (the shin rows 46 and 47 compress by 1). |
| 2 | Pot rattle: key 0 with POT offset (0, -1). Close the 1-row gap at the crown by repeating the rim row 24 `K` run on row 24 (the pot lip sits 1 higher, the head is unchanged). |
| 4 to 8 | Walk, from the contact plan below. Body offsets from key 0: 4 (+1, 0), 5 (+1, +1), 6 (+1, +2), 7 (+1, 0), 8 (+1, +1). Near fist x offset from key 0: 4 -1, 5 0, 6 +2, 7 +1, 8 0. |
| 9 | Rear: HEAD and POT (-2, 0); the near arm halfway: upper arm forward, forearm at about 45 degrees up, fist at about (46, 32). |
| 11 | Launch: HEAD and POT (+3, +3); TORSO leaning about 20 degrees; the near arm at the side, fist at (30, 40); legs as 12. |
| 15 | Fall beat: GOB-01 A frame 20 body offset (+10, 0); POT 2 px clear above the highest head pixel, handle still pointing back. Top row at least 22. |
| 16 | Floor impact: GOB-01 A frame 21 body offset (+10, 0); POT at the key-17 place but 1 px higher (bounce). Top row at least 38. |

**Walk contact plan.** The runtime moves the sprite **4 px per 6-tick frame (0.667 px per tick, 40 px/s)**; a planted foot moves back exactly 4 px per frame, seam included, and never forward. Each foot is planted for 4 frames and lifted for 2. Positions are the right end of the foot's outline run on row 55 (planted) or of its lowest row (lifted):

| Frame | 3 | 4 | 5 | 6 | 7 | 8 | seam to 3 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Near foot | **39** planted | **35** planted | **31** planted | **27** planted | 31 lifted 3 | 35 lifted 2 | 35 to 39 (lands) |
| Far foot | **27** planted | 31 lifted 3 | 35 lifted 2 | **39** planted | **35** planted | **31** planted | 31 to 27 (-4) |
| Body (top row) | 18 low | 16 high | 17 | 18 low | 16 high | 17 | |

Contact frames 3 and 6 have both feet down and the body 2 px low; passing frames 4 and 7 carry the body at idle height (never taller than the idle). That is the 2 px weight bob. The near leg is always in front (light ramp), the far leg behind (deep ramp), so no limb swaps sides. Lifted feet keep their shape and rise whole.

## 9. Validators and acceptance targets

`python3 -m scripts.pixelart validate assets/source/characters/goblin/engineer/eng_01.pixelart.json`. Design stage: V01 to V15 pass on key 0 (V08, V16 not configured). Execution extends `tags`, `roles` (move `walk`, tell `attack_tell`, attack `attack_ram`, hurt `hurt`, defeat `down`), `frozen_keys.frames` (0, 3, 10, 12, 13, 14, 17) and these rules:

- V04 palette: at most 20 (17 used). V05 isolated: 0. V06 orphans: at most 9.
- V07 contact: every tag grounded; the lowest opaque row is 55 in every frame.
- V12 unique frames: on.
- V13 bounding boxes (every key's section 7 values meet these): idle 0 `top` 16 `bottom` 55; all frames `min_top` 13; idle 1 and 2 `min_top` 15 `max_top` 17; walk 3 and 6 `min_top` 18 `max_top` 18, 4 and 7 `min_top` 16 `max_top` 16, 5 and 8 `min_top` 17 `max_top` 17; tell 10 `max_top` 16 `max_left` 11 `min_right` 57; tell 9 `min_right` 46; ram 12 `min_top` 21 `max_top` 23 `min_right` 51; recover 13 `min_top` 19 `max_top` 21; hurt 14 `min_top` 13 `max_top` 15 `max_right` 40; down 15 `min_top` 22, 16 `min_top` 38, 17 `min_top` 39 `max_top` 41 `max_width` 54.
- V14 key counts: `e` at least 3 in 0 to 12, 0 in 13 to 17; `w` at least 2 in 14; `7` exactly 1 in 0 to 16 (the pot glint travels with the pot; in 17 it may be 0 or 1); `5` at least 40 in every frame (the pot is never hidden or cut).
- V16 foot drift: `{"max_px": 5, "tags": ["walk"]}`, loop seam included (the 4 px step plus 1 px of rasterization).
- Read targets (silhouette gate, by eye at 1x on lit brick): the pot reads as a saucepan with a handle, not a hat or a bucket; the tell (10) and the ram (12) are told apart by arm and head height alone; the recover (13) has no visible eye; the Engineer is told apart from goblins A/B and the Bard by the head alone.
- Tests (`tests/test_art_eng01.py`): generator parity, key 0 frozen, frame and pivot, 40 px, palette reused with the same hex, the pot rows equal to the rider's, the body below the chin equal to GOB-01, the wrench a stowed prop. Execution adds tick windows (`idle` 38, `walk` 36, `attack_tell` 32, `attack_ram` 9, `attack_recover` 18, `hurt` 12, `down` 15 + hold) and the pot-rows-identical check on every frame where the POT block is only offset.

## 10. Evidence

```text
python3 scripts/author_eng01.py keys            # grid (design stage: key 0)
python3 scripts/pixel_grid_to_ase.py build assets/source/characters/goblin/engineer/eng_01.grid.json assets/source/characters/goblin/engineer/eng_01.ase
python3 -m scripts.pixelart freeze assets/source/characters/goblin/engineer/eng_01.pixelart.json
python3 scripts/author_eng01.py key0-scale      # needs bash scripts/export_assets.sh first
```

`docs/evidence/eng-01/key0-scale.png` (896 x 360): key 0 at 1x on lit Floor 1 next to Carl, the GOB-01 goblin A and the Bomb Bard (key 0), 640 x 360 unscaled, plus a 4x copy on the right. Execution adds the manifest `assets/manifests/eng_01.json` (tick rows, boxes, attack ticks), the evidence set (GIFs, contact sheets, lit scenes), `walk_strip.png` and `tell_ram_strip.png` (`python3 scripts/author_eng01.py strips`), and `docs/evidence/eng-01/review-pack/` (3 images plus `validator_summary.txt`: `python3 -m scripts.pixelart evidence` and `review-pack` on the config).

## 11. Files

| File | Role |
| --- | --- |
| `scripts/author_eng01.py` | deterministic authoring script: palette, part blocks, literal key 0, key0-scale |
| `assets/source/characters/goblin/engineer/eng_01.grid.json` / `.ase` | pixel-grid source and Aseprite master |
| `assets/source/characters/goblin/engineer/eng_01.pixelart.json` | asset config |
| `tests/fixtures/eng-01_keys.json` | frozen key 0 |
| `tests/test_art_eng01.py` | spec tests |
| `docs/evidence/eng-01/key0-scale.png` | scale evidence |
| `assets/manifests/eng_01.json` | manifest: tag, tick rows, attack ticks, placeholder boxes |
| `docs/evidence/eng-01/` (`walk_strip.png`, `tell_ram_strip.png`, `review-pack/`) | execution evidence |

## 12. Resolved questions (from the stub)

1. **Printed 44 px:** not approved and not used.
2. **Standing height:** 40 px (section 3).
3. **Wrench:** kept as a stowed prop (jaw up in a belt loop at the back hip, layer `prop`), never in the hand and never swung. It carries the canon "more tool-focused silhouette" without making the shorthand a weapon.
4. **Kit:** the belt with a small iron buckle is kept (the board shows it; it fills the stick strap's row). The pauldron and the wrist wrap are board flourish: the rider has neither, and they would break the plain-rag family look.
5. **Behaviour set:** tell = the rider's point, attack = a head-down pot ram (section 4). No pineapple: it is canon for the A/B goblins, not for him.
6. **Where it appears:** assumed to be the Engineer on foot (for example after the Dozer is wrecked); the same set also serves a cutscene. Owner placeholder (section 13).
7. **Facing and handle:** authored facing right with the handle back (left); the runtime mirrors the whole sprite, so the handle always trails.
8. **Pot colour:** the rider's blue-grey iron ramp wins (same individual, already proven in A1 light); the concept's lilac cast is board rendering.
9. **Contact row and pivot:** 55 and (28, 55), standing, as GOB-01.

## 13. Owner placeholders and open questions

- **Attack:** the pot ram, its reach, damage and knockback are placeholders. Whether the Engineer fights on foot at all is an owner decision; the set is ready either way.
- **Move speed:** 40 px/s is a placeholder (slower than the common goblin's 62 px/s chase: a foreman, not a runner).
- **Wrench use:** any later use of the wrench (repairs, a tool strike) needs a gameplay decision and a spec change; the sprite never shows it in a hand.
- Gate: passed (two non-blocking notes, both applied in execution: the eye colour wording in section 1, and the pot-handle length guard in `tests/test_art_eng01.py`).

## 14. Executor notes (keys 3 to 17 and the in-betweens)

Add each frame to `scripts/author_eng01.py` as literal rows or as block offsets of key 0 plus redrawn parts (same `put`/segment style as `scripts/author_bard01.py`), extend `TAGS`, `TICKS`, `KEY_FRAMES`, the config and the fixture (`pixelart freeze`). Never edit `KEY0` or its frozen hash.

- **Blocks:** move POT, HEAD, TORSO and WRENCH whole by the section 7 and 8 offsets. Where two blocks overlap after an offset, the front block wins in this order: near arm and fist, HEAD, POT, TORSO, LEGS, WRENCH. Re-close any outline broken at a block seam with `K`; never leave a gap row between pot and head (the rim run on the row above the head is always continuous).
- **Pot:** the POT block stays pixel-identical in every frame where section 7 or 8 says "offset" (0 to 14, 16, 17; 15 too). The glint `7` travels with it. Never redraw, tilt or recolour the pot.
- **Legs (walk, tell, ram):** 2 px shins with an outline (near: `l m` with `s` shade; far: `m a`), from row 47 to the foot. Near foot: the key-0 near foot rows 52 to 55 shifted whole (9 px outline run on row 55). Far foot: the key-0 far foot rows 52 to 55 shifted whole (6 px run). A lifted foot keeps its rows and rises whole; draw the far leg behind the near leg.
- **Arms:** only the near arm is drawn (the far arm stays behind the cape, as in GOB-01), except the 2 px far hand in 13. The pointing arm in 9 and 10 is the rider's arm pixels; do not redraw it.
- **Faces:** snarl (key 0) in 0 to 11; gritted teeth (the `w` row with the `o` rows closed to `K`) in 12; slack jaw in 13; GOB-01 frame 16's face in 14; covered in 17.
- **Hurt and down:** take the GOB-01 A frame 16 and 20 to 22 `body` layers (never the `weapon` layer), then add the belt row on the hip row and the POT block. Keep the GOB-01 skin and rag pixels as they are.
- **Checks before reporting:** `python3 -m scripts.pixelart validate` with the section 9 rules, `python3 -m pytest tests -q -k "eng01 or pixelart"`, `key0-scale`, the review pack and the walk strip regenerated. If a section 7 joint and a section 9 target cannot both hold, the target wins; record it here.

### Executor notes (applied; the section 14 text above is the brief)

All frames are composed by `scripts/author_eng01.py` from key 0 pixel blocks; no pixel was edited by hand and key 0 is byte-identical (fixture hash unchanged). Conservative readings and deviations:

1. **Block partition.** HEAD and TORSO share rows 30 to 33 in their boxes; the split is the cape edge (x 27, 28, 29, 29). ARM (near arm and fist) is rows 34 to 41 right of the cape. The skirt and hem rows 43 to 46 belong to TORSO, so LEGS are rows 47 to 55 (the hem row 46 is not a leg row). With a torso offset of +2 the shin shows 3 rows below the hem (frames 3 and 6).
2. **Legs.** Feet are exact to the contact plan (right ends 39, 35, 31, 27 planted; lifted 3 and 2 px, whole). The shin (rows 47 to 51) moves rigidly with its foot, clamped to stay under the hem (near -10 to +2, far -4 to +13); the foot is wider than the shin, so the clamped remainder shows as a 1 to 2 px step at the shin and foot seam (frame 6 far leg, frames 10 to 12 near leg). A shear was tried and rejected: it left orphan pixels in the shins. Leg pixels at or above the hem row are hidden, and far-leg slivers left behind the near leg are dropped (orphan rule). Frames 4 and 5, 7 and 8 draw the lifted far or near leg behind the planted one.
3. **Orphan budget.** Key 0 already uses 8 to 9 of the 9 allowed orphan pixels (belt, buckle, wrench), so every other frame had to add none; the hurt belt is a plain rag-mid row with a 2 px `5` buckle for that reason (the key-0 belt pattern would add 10 orphans).
4. **Pointing arm (10).** Rider `engineer_point` rows 39 to 44, x 40 to 60 (not 39: the rider's chin outline is left out, and row 39 only from x 44), moved to rows 30 to 35 and 2 px left (hand end x 58). The wrench follows the torso (-2, 0) in 10, (+1, +2) in 11 and (+2, +3) in 12 (11 is halfway; the spec gives no offset).
5. **Rear (9) and arms.** The bent arm is drawn with a 2 px line (upper arm level, forearm at about 45 degrees up, fist at (46, 32)); the key-0 arm is dropped from 9 to 12. In 11 the key-0 arm hangs at the side, moved (-8, +1) (fist about (30, 40)); in 12 the arm swung back is a 2 px skin bar with a 4 x 3 fist at (22 to 25, 40 to 42).
6. **Ram (11, 12).** The leaning sack is a scanline generator (`lean_sack`): half-width 6, cape banding `u`, `t`, `r`, neck (31, 35) in 11 and (37, 37) in 12, hips (27, 45); skirt, belt and hem rows 42 to 46 are key 0's. Mouth in 12: the tooth row made continuous, the `o` rows closed to `K`.
7. **Recover (13).** The near arm is key 0's moved (+5, 0), with 4 px of skin stretched out to the cape (fist about (43 to 46, 37 to 40) instead of (44, 37)); the far hand is a 2 px `m` pair at (18 to 19, 34). The POT paints over the HEAD, so no `e` shows (spec priority order is reversed for this frame, as section 7 requires). Slack mouth: the tooth row's `w` moved down one row (`o` above).
8. **Hurt (14).** GOB-01 frame 16 `body` layer as is; POT offset (-8, -3) chosen directly (top row 13): the rim sits one row above the raised hand's top row, touching and not overlapping. WRENCH (-8, +2) as the spec; belt row on row 44 (see note 3).
9. **Down (15 to 17).** GOB-01 frames 20 to 22 `body` layers (+10, 0). POT offsets: 15 (-10, +14) (rim row 38, 2 px above the ear tip on row 41), 16 (-10, +30), 17 (-10, +31). No belt row and no wrench (the lying body shows the rag edge at the hip; the spec's "belt row on the hip row" has no hip row in a lying frame).
10. **Closure.** Frames 3 to 13 close outline seams by adding `K` at every empty 4-neighbour of a non-outline pixel; frames 0 to 2 and 14 to 17 need none.
11. **Boxes.** The section 3 boxes stay as unmeasured placeholders (see note 17). Frame 14's extent (x -23 to 11) is wider than the placeholder because the hopping pot and handle are not a hurtbox; frame 10's extent (x to 30) is the pointing arm, also not a hurtbox.
12. **Spec text corrections.** The glint `7` sits at (29, 19), not (28, 19) as section 7 says; the tests use the measured position. Idle 2's pot is the 16 to 23 rows moved up 1 with the key-0 rim row kept on row 24, so the opaque top is row 15 (one row above the idle).
13. **Review fix E1 (frame 12, re-frozen).** The near arm is bent at the elbow (upper arm (35, 38) to (31, 42), forearm to a 3 x 3 fist at (26, 40)); the fist and its outline stay 3 px clear of the wrench (test `test_ram_arm_is_clear_of_the_wrench`). The sack half-width is now 7 (was 6) so the cape banding `u`, `t`, `r` reads along the 40-degree spine; notes 5 and 6 above are superseded where they describe the 13 px bar and the 4 x 3 fist.
14. **Review fix E2 (frame 14, re-frozen).** The GOB-01 frame 16 body has a transparent row 43 across the waist; it is closed with `r` rag (x 14 to 31, between the torso above and the belt row 44), so the body is continuous. The flat belt stays. No orphan-rule conflict: frame 14 stays at 9 of 9 orphans. New test `test_no_see_through_row_inside_the_body`: every row from the top to the floor of frames 0 to 14 holds at least 3 body-layer pixels (the lying frames have 1 to 2 px ear and handle tips).
15. **Review fix E3 (frame 10, re-frozen).** The 1 to 2 px see-through gap at the shoulder (x 36 to 37, rows 31 to 32) between the chin and the copied pointing arm is closed with `K`.
16. **Review fix E4 (frame 11, in-between).** The key-0 near arm hangs at (-4, +1) instead of (-8, +1), so the cape banding shows left of the arm (the fist is at about (34, 40), not (30, 40)).
17. **Boxes and glint.** The section 3 boxes are labelled unmeasured placeholders (`boxes_status` in the manifest); the glint position in section 7 is corrected to (29, 19). Re-freeze record: `pixelart freeze --force` after items 13 to 15 (keys 0, 3, 13, 17 hashes unchanged; keys 10, 12, 14 changed).

## 15. Rights and provenance

Fan-concept art under the owner-reported `ART-PUBLICATION-ALL-V1` scope (see `docs/04-production/rights-and-release.md`). Crops are reference only, never traced or exported. No third-party assets, no copied designs or dialogue, no image-model raster output. Key 0 reuses committed project pixels (GOB-01 A frame 0, DOZ-01 rider frame 2) as recorded in `scripts/author_eng01.py`.

### Rights/Provenance

- Concept: the owner-approved trio board under `ART-PUBLICATION-ALL-V1`; packet crops are reference only.
- Packet: script plus Claude Sonnet.
- Spec + hero key (key 0): Claude Opus.
- Silhouette gate: passed (independent top-tier review).
- Remaining keys + in-betweens + execution: Claude Sonnet.
- Independent review of the finished set: pending.
