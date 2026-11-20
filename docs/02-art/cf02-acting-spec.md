# CF02 ACT-DESIGN: combat-acting key poses and spec

**Status:** design in review (ACT-DESIGN of [COMBAT-FUN-02](../01-design/combat-fun-02-post-demo01-proposals.md), work order row ACT-DESIGN in [combat-fun-02-workorder.md](../07-agents/combat-fun-02-workorder.md)). Key poses and this spec; the Carl production sheet (`carl_h52_cf02`, ART-CARL-CF02) is drawn and awaits review, the Donut and goblin sheets and all Bevy wiring do not exist yet. Nothing here is owner-approved. Rights basis for publishing: owner-reported `ART-PUBLICATION-ALL-V1` ([rights register](../04-production/rights-and-release.md)); no reuse licence, no game distribution right.

Serves CF02-Q21 (slammed poses), CF02-Q70 to Q74 (acting overlays, pose budget, tone), with the frame data of Q02 (stomp), Q05 (follow-through), Q50 (air kick), Q52 and Q53 (dive and bounce), Q54 (bail-out), Q56 (landing recoveries), Q61, Q65 and Q72 (goblin reactions).

Read with: [antihero tone](../01-design/antihero-tone.md), [Carl bible](carl-character-bible.md), [Donut bible](donut-character-bible.md), [ANIM-CARL-02 melee spec](carl-melee-anim-02-spec.md) (frame, pivot and stamp vocabulary this spec inherits), [GOB-01](gob-01-goblin-spec.md), [GOB-02 death spec](gob-02-death-spec.md), [death system proposal](../03-tech/monster-death-system-proposal.md), [animation system](animation.md).

| File | Role |
| --- | --- |
| `assets/source/characters/carl/cf02_keys/carl_h52_cf02_keys_ground.grid.json` | Carl grounded keys: `stomp_knee_up`, `stomp_stamp`, `dive_heavy_landing`, `flinch`, `frustration_shake` |
| `assets/source/characters/carl/cf02_keys/carl_h52_cf02_keys_air.grid.json` | Carl airborne keys: `air_kick_boot`, `dive_tuck`, `dive_plunge`, `bailout_scramble` (lowest-row anchored, section 2) |
| `assets/source/characters/donut/cf02_keys/donut_h40_cf02_keys.grid.json` | Donut keys: `startle_puff`, `smug` |
| `assets/source/characters/goblin/cf02_keys/goblin_a_h36_cf02_keys_acting.grid.json` | Goblin A acting keys: `hesitate`, `panic_flee`, `taunt_thump` |
| `assets/source/characters/goblin/cf02_keys/goblin_a_h36_cf02_keys_slammed.grid.json` | Goblin A slammed keys: `stomped_impact`, `stomped_rest`, `wall_impact`, `wall_rest`, `flung_impact`, `flung_rest` |
| `assets/source/characters/carl/carl_h52_cf02.{grid.json,ase,pixelart.json}`, `assets/manifests/carl_book1_h52_cf02.json` | ART-CARL-CF02 production sheet: 13 Carl frames (the 9 keys above, the airborne lift applied, plus the 4 derived frames; reviewer pinhole fix, PR #146: three 1 px pixels closed in the `stomp_knee_up` jacket, `dive_plunge` far arm and `bailout_scramble` jacket keys, in both the key files and the sheet), authored by `scripts/author_carl_cf02.py`; evidence in [`docs/evidence/carl-cf02/`](../evidence/carl-cf02/README.md) |
| [`docs/evidence/cf02-act-design/`](../evidence/cf02-act-design/README.md) | key sheets at 1x and 3x, two lit Floor 1 compositions at gameplay scale, the unlabelled hesitate check |

Every key file is `pixel-grid-v1`, uses the approved palette of its character unchanged (goblin files add only the two approved death colours `B` blood_dark and `P` blood_wet from the GOB-02 death sheet), the approved canvas and pivot, and passes `scripts/pixel_grid_to_ase.py lint --strict` (0 isolated, 0 orphan pixels, every frame's lowest row on the pivot row). They are built from approved frames (layer cels moved, mirrored or re-posed) plus new limb pixels; no new face, head or costume was drawn.

## 1. Acting brief

Carl is a tired, ordinary man doing hard physical things badly but stubbornly. Every new Carl pose shows **effort or annoyance**, never control: the stomp is a knee-up and a stamp that drags his whole weight down, the air kick is a flailing boot, the dive is a fall he committed to, the bail-out is a scramble with his eyes on where he is going, the flinch is a cringe, frustration is a muttered flap of the hand. Donut is a proud show cat: she puffs up when something crashes and looks down her nose when her own hit lands. Goblins are cowardly bullies: they double-take, run with their arms up, and thump their chest only when they feel safe. Each slammed finish shows the cause the player chose.

## 2. Frames, pivots and anchoring

| Character | Canvas | Pivot | Grounded rule | Airborne rule |
| --- | --- | --- | --- | --- |
| Carl | 48 x 56 | (25, 55) | lowest opaque row 55 | keys are drawn **lowest-row anchored** (lowest row 55 in the key file, the ART-INT-02 runtime convention). The production sheet lifts each airborne frame **2 px** (lowest row 53, nothing on rows 54-55) so the ANIM-CARL-01 airborne pivot rule holds; this lift is the only change allowed to an airborne key. Exception (review ruling, PR #146): `bailout_scramble` (55 px tall) is lifted 1 px (lowest row 54, like `jump_fall`); every other airborne key is lifted 2 px |
| Donut | 72 x 44 | (46, 43) | lowest opaque row 43 | no airborne keys |
| Goblin | 64 x 56 | (28, 55) | lowest opaque row 55 | `wall_impact` is lowest-row anchored like Carl's airborne keys; its **wall contact column is x16** (12 px behind the pivot), which the presenter puts on the wall face |

All frames face right. Mirror around the pivot column for facing left. Columns 0 and the last column stay empty in every key (clipping rule). Hurtboxes and hit boxes stay gameplay data (Q02, Q50, Q52); the sprites never feed back into collision.

Key bounding boxes (pivot-relative x, canvas rows), for the reviewer and the executor:

| Key | x from pivot | Rows | Height |
| --- | --- | --- | --- |
| `stomp_knee_up` | -21 .. +15 | 10-55 | 46 |
| `stomp_stamp` | -14 .. +17 | 12-55 | 44 |
| `dive_heavy_landing` | -10 .. +18 | 13-55 | 43 |
| `flinch` | -12 .. +10 | 7-55 | 49 |
| `frustration_shake` | -11 .. +12 | 6-55 | 50 |
| `air_kick_boot` | -23 .. +18 | 7-55 | 49 |
| `dive_tuck` | -20 .. +14 | 12-55 | 44 |
| `dive_plunge` | -20 .. +18 | 5-55 | 51 |
| `bailout_scramble` | -24 .. +15 | 1-55 | 55 |
| `startle_puff` (Donut) | -21 .. +22 | 1-43 | 43 |
| `smug` (Donut) | -21 .. +22 | 3-43 | 41 |
| `hesitate` | -12 .. +18 | 19-55 | 37 |
| `panic_flee` | -24 .. +23 | 18-55 | 38 |
| `taunt_thump` | -27 .. +15 | 20-55 | 36 |
| `stomped_impact` | -22 .. +34 | 36-55 | 20 |
| `stomped_rest` | -24 .. +34 | 37-55 | 19 |
| `wall_impact` | -12 .. +25 | 21-55 | 35 |
| `wall_rest` | -14 .. +32 | 33-55 | 23 |
| `flung_impact` | -16 .. +17 | 35-55 | 21 |
| `flung_rest` | -16 .. +15 | 40-55 | 16 |

## 3. Carl (budget 13 new frames)

**K** = key in this PR (frozen: the execution copies it pixel for pixel, except the 2 px airborne lift). **D** = derived by the execution PR from the named key and approved frames, as described.

| # | Frame | K/D | Serves | Emotion beat | Silhouette | Ticks | Pivot and contact | Reuse |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | `stomp_knee_up` | K | Q02 startup (the 9-tick tell) | strain, "hold still" | From kick wind-up k1: knee up to the jacket hem, shin vertical, the **Croc held flat, sole down, 7 px above the floor** at x +6..+15 (inside the stomp box's reach, off the ground). Upper block 2 px forward and 1 px down of k1 (he lurches over the target), head 1 px further forward and down (looking at the goblin). Near arm thrown back level at shoulder height for balance, far fist by the chin | t0-8 | support Croc flat on row 55 under the hips; the root step (6 px, ticks 1-6) moves the whole sprite with the sim | k1 lower body, k2 near arm |
| 2 | `stomp_stamp` | K | Q02 active, hit-stop, Q21 stomped finish | full weight, ugly | Haymaker b1 lower body (front knee deep over a flat Croc at x +8..+17, back foot up on its toe), the jab upper block dropped 4 px and pushed 2 px forward: hunched over the foot, chin down. Near arm flung low behind, far arm hanging forward-down. Reads as the whole body landing on one foot | t9-15 (active t9-11, hit t9; hit-stop 6, +3 lethal, holds this frame) | front Croc and back toe on row 55 | b1 legs, a1 upper, k3 far arm |
| 3 | `stomp_recover` | D | Q02 recovery | exhale, a shake of the stamping leg | From `stomp_stamp`: upper block back up 3 px (U +1 forward of idle), head v3, the stamping foot pulls back to x +3..+12 (at most 2 px from idle), back foot comes down flat at x -10..-1; near arm hangs. Hands over to idle with at most a 2 px foot snap (like k6) | t16-25 | grounded | idle arms, k6 feet |
| 4 | `air_kick_chamber` | D | Q50 startup | gathering, clumsy | From `dive_tuck`: the near knee pulled up, the far leg trailing straight down; upper block and arms of `air_kick_boot`. A 5-tick wind-up must not look like the tuck: the far leg hangs, it is not tucked | t0-4 | airborne | `dive_tuck` legs, `air_kick_boot` upper |
| 5 | `air_kick_boot` | K | Q50 active (box reaches 16 px below the feet), Q51 relaunch | flailing yell | Kick contact k3 upper (leaning back, yelling), **the near leg driven down-forward at 45 degrees** from the boxer flap to a toe-down Croc at about +12..+18 forward, the lowest point forward of the pivot; far leg hanging. Near arm thrown back level, far arm hanging forward. Not a flying kick: the body leans away from the kick | t5-14 (active t5-10, hit-stop 5 holds it); t15-20 shows the existing `jump_fall` (or `jump_rise` while rising) | lowest-row anchored, Croc toe on the lowest row | k3 upper and far arm, k2 near arm, `jump_rise` bent leg |
| 6 | `dive_tuck` | K | Q52 tuck, the readable tell | visibly gathering himself | Both knees pulled up (a ball), both Crocs hanging toe-down, the kick-chamber upper block (leaning back, fist by the chin, near arm cocked). The compact ball against the tall `dive_plunge` is the tell | t0-5 (6 ticks, vy 0) | lowest-row anchored | k1 upper and near leg, new far leg |
| 7 | `dive_plunge` | K | Q52 active plunge, the dive hit and lethal stomped finish | falling, regretting it | Upright idle body, **legs straight down with both Crocs toe-down**, both arms flung out sideways with open hands (d0 arms, the far arm 3 px higher than the near one, so it is no mannequin T), the pain face with eyes shut. A man dropping, not a hero diving: no fist leads, no pointed toes forward | t6 until landing or 30 active ticks; held through the hit-stop; after 30 ticks the existing `jump_fall` | lowest-row anchored | d0 arms, h0 pain head, idle legs |
| 8 | `dive_heavy_landing` | K | Q52 / Q56 whiff landing (10 ticks) | "my knees" | Land frame squat pushed 3 px deeper, near forearm braced on the front knee with a fist, pain face. **No hand on the floor** (no three-point landing) | 10 ticks after a whiff landing; a dive hit has no landing frame (Q56: bounce, recovery 0) | Crocs on row 55 | `land` squat, h0 pain head |
| 9 | `bailout_pushoff` | D | Q54 take-off | alarm | From `jump_start`: weight thrown back, front foot pushing, the k3 yelling head of `bailout_scramble` (eyes on the threat), the `dive_plunge` d0 arms (out sideways at shoulder height, open hands, the far arm 3 px higher) instead of the scramble arms (review ruling Q5: the scramble arms read as cheering at take-off). Airborne from the press tick, so it is drawn lowest-row anchored with the pushing toe on the lowest row | t0-2 | lowest-row anchored | `jump_start`, `bailout_scramble` head, `dive_plunge` arms |
| 10 | `bailout_scramble` | K | Q54 hop (escape about 5 ticks after the press) | alarm, backpedalling with eyes on the threat | `jump_rise` body moved 2 px back, the **k3 yelling head** (mouth open, eyes on the goblin), arms **asymmetric**: the near arm flailing up and back with an open hand above the head line, the far hand raised palm-out at face height toward the goblin (warding). Legs running in the air. Facing unchanged (Q54). Not a jaunty hop: no calm face, no level arms | t3 until landing (about 30 air ticks); landing recovery 4 ticks shows the existing `land` frame | lowest-row anchored | `jump_rise` legs, k3 head |
| 11 | `flinch` | K | Q70 flinch (near miss within 12 px) | cringe | Idle stance, upper block 1 px back and 2 px down (shoulders up, neck in), pain face eyes shut, far fist pulled up by the chin. Distinct from `hurt` (head snapped back, arms flung): the flinch pulls **in**, the hurt flies **out** | 8 ticks; idle only | idle feet on row 55 | idle legs, h0 head, k1 far arm |
| 12 | `frustration_shake` | K | Q70 frustration (3 whiffs) | annoyed muttering | Idle stance slumped 1 px, the k1 effort head 1 px lower and tilted (its front half 1 px further down), closed mouth, near forearm raised in front of the chest with the **hand open in a fan** (three spread fingers and a thumb), palm toward the goblin ("forget it"). Must not read as a guard: no closed hand, no forearm across the face | 6 ticks per frame, alternating with 13 for 24 ticks; idle only | idle feet | idle legs, k1 head |
| 13 | `frustration_flap` | D | Q70 frustration, second frame | | `frustration_shake` with the hand 3 px lower and the wrist turned down (fingers point at the floor), head 1 px lower | alternates with 12 | idle feet | |

**Follow-through decision (Q05, Q73): yes, the follow-through skips the kick chamber k1.** `FollowKick` (startup 9, active 4, recovery 19, 32 ticks) plays the approved kick frames as: k0 load t0-6, **k2 t7-8**, k3 contact t9-14 (active t9-12, hit t9), k4 t15-19, k5 t20-26, k6 t27-31. The 9-tick startup is all k0 except the last 2 ticks, so the nearly extended k2 keeps the 2-tick hold it has in the legacy kick and no long reach shows before the first active tick. Reasons: the confirm only fires on a goblin already staggered by the jab, so the chamber's fairness tell is not needed; going from the load straight to the extension, without the chamber, makes the 4-tick-faster kick visibly different from the full push kick, whose k1 chamber stays its signature tell. No new frame is spent.

**Planted-feet overlays are idle-only (rule, Q69 / Q70 / Q71).** `flinch`, `frustration_shake`, `frustration_flap`, `startle_puff`, `startle_settle` and `smug` have planted feet and show only while the character is in `idle`; in `run` or any move the overlay is not shown (it would skate). Q69 permits idle or run, so this narrows the choice inside the rule; CF02-I records it as a data value under the Q69, Q70 and Q71 IDs.

Other reuse (no new frames): the dive bounce (Q53) uses `jump_rise` and `jump_fall`; the air kick and bail-out landing recoveries (4 ticks, Q56) use the existing `land` frame; Donut's bail-out (Q55) uses her jump frames.

## 4. Donut (budget 4 new frames)

| # | Frame | K/D | Serves | Beat | Silhouette | Ticks | Contact |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | `startle_puff` | K | Q71 startle (crash, blast, wall slam or Carl hurt within 160 px) | the jolt | Idle body raised 3 px on **stiff straight legs** (the classic arch), head and tiara up with it, **tail bottle-brushed**: lifted 4 px with a 1 px dark fur rim, pupils shrunk to 1 x 2 slits in the gold eyes (wide-eyed) | t0-5 | paws on row 43 |
| 2 | `startle_settle` | D | Q71 startle, second frame | wary | From `startle_puff`: body 2 px back down, tail still puffed but 2 px lower, pupils back to idle size, head 1 px toward the event side (no new head) | t6-11 | paws on row 43 |
| 3 | `smug` | K | Q71 smug (her hit lands) | "naturally" | Idle with the head, face and tiara 1 px higher (chin up), **eyes half-lidded** (upper half of each eye painted over with the coat colour), tail lifted 2 px. Cute, not a sneer | 20 ticks when idle | paws on row 43 |
| 4 | spare | - | reserved | | Not drawn. Kept in reserve for a fix the review may ask for (for example a second smug frame); spending it needs no new budget decision, a fifth frame does | | |

Startle and smug are full-body overlays that show only in `idle` (the planted-feet rule of section 3).

## 5. Goblin (budget 11 new frames per sheet)

| # | Frame | K/D | Serves | Beat | Silhouette | Ticks | Contact |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | `hesitate` | K | Q61 / Q72 hesitate | double take | Idle legs; **the head mirrored to look back** toward the cause, the upper block jerked 2 px back and 1 px up, **the club pulled in tight**: held upright against the belly with the fist at the belt and the pineapple top on row 28 (below the shoulder line), **the free hand raised palm-out** in front of the chest. Must stay readable as "not attacking": the club never rises above the shoulders. Acceptance: in an unlabelled 1x strip of idle 0-3 and `hesitate`, a reviewer who has not read this spec picks out the hesitate | 20 ticks (Q61 stand-still) | feet on row 55 |
| 2 | `panic_flee` | K | Q61 / Q72 panic, flee cycle frame 1 | terror | The knockback upper body **mirrored** so it leans into the run, **the empty arm thrown up** with a splayed open hand, and **the face looks back** over the shoulder at the threat; **the club dragged low behind** in the trailing hand (the frame-14 club, mirrored; pineapple top on row 40, never above the shoulder line, so the flee never flashes the GOB-01 R1 attack-tell sign); legs of walk frame 5 (feet point in the run direction). The sheet's right is the flee direction: the presenter faces the sprite along the flee velocity, not toward the hero | 6 ticks per frame, alternating with 3, for the Q58 panic length (60-90 ticks) | feet on row 55 |
| 3 | `panic_flee_b` | D | Q72 panic, flee cycle frame 2 | | `panic_flee` with the legs of walk frame 8 (the other stride), the raised empty arm 2 px lower, the dragged club 1 px higher (a bounce, still below the shoulder line), body 1 px higher | alternates with 2 | feet on row 55 |
| 4 | `taunt_thump` | K | Q65 / Q72 taunt (chest thump) | arrogant | Club tucked behind under the far arm with the pineapple low at the back hip (the club never above the shoulders, so the taunt can never be mistaken for `attack_tell`), **the upper block leaned back** (head 2 px, chest 1 px: chest out), **the snout tipped up 1 px** (chin up), the near fist thumping the chest, mouth open in a jeer | 9 ticks per frame, alternating with 5, for the Q65 36-tick taunt; a non-launching hit holds the shown frame 6 extra presentation ticks (Q72) | feet on row 55 |
| 5 | `taunt_beckon` | D | Q72 taunt (beckon) | "come on then" | From `taunt_thump`: the near arm extended forward at chest height, **open palm up, the hand at least 3 px wide with all fingers together**, no single-pixel column above the knuckles (at 1x a lone spur reads as a raised middle finger), head 1 px forward. No obscene gesture (Q74); the ART-GOB-CF02 review checks this frame at 1x | alternates with 4 | feet on row 55 |
| 6 | `stomped_impact` | K | Q21 stomped (`Hit{Stomp}` or `Hit{DiveStomp}`), Q43 | the fold | The corpse body **folded into a V around the stamp**: belly pressed 2 px into the floor, head and knees snapped up 5 px, a blood spit from the mouth (gore layer). The stamping foot covers the belly in the scene | 8-tick impact hold (Q21), after any taunt hold | belly on row 55; the body stays where it died (lethal knock 0 / 0) |
| 7 | `stomped_rest` | K | Q21 stomped aftermath | dead bug | On the back, belly flattened, **both legs sticking straight up and splayed**, head lolled, a thin pool under the body (gore layer). Distinct from `crushed_remains` (spread flat under a slab) and from the routine corpse (knees bent) | rest (corpse) | on row 55 |
| 8 | `wall_impact` | K | Q21 wall crumple (`WallSlam`) | splat | The knockback body with everything behind x16 removed: a **flat back pressed on the wall**, **squashed** (2 rows out of the torso, 2 out of the legs so the feet pull up, 2 px wider at the wall), the hurt head with **eyes squeezed shut**, the back arm flung up the wall face with a splayed hand, the front arm thrown forward with an open hand. The club leaves the hand (the presenter keeps the flying club prop) | 8-tick impact hold, airborne | lowest-row anchored; wall contact column x16 |
| 9 | `wall_rest` | K | Q21 wall crumple aftermath | dead, slumped | The get-up torso sitting with the **back on the wall**, **the dead head** of `corpse` 25 (X eye, slack mouth, tongue out) **lolled sideways onto the near shoulder** with the ear hanging, the near arm limp with the hand in the lap, both legs out flat along the floor (toes up), a short smear on the gore layer on the wall face at x14, the club fallen beside it. No live head, no open eye: it must never read as getting up | rest | seat and heel on row 55; leftmost column x14 on the wall face |
| 10 | `flung_impact` | K | Q21 flung (lethal kick, Q06) | face-plant | The knockback-air body landed **head first**: head on the floor, body diagonal up and back, legs still in the air. The club lands separately | 8-tick impact hold | head on row 55 |
| 11 | `flung_rest` | K | Q21 flung aftermath | heap | The face-plant collapsed: legs fallen over onto the back into a **low folded heap**, face down. Distinct from the routine corpse (on its back) | rest | on row 55 |

The gore layer on the stomped keys is a preset-4 reference only: preset 1 draws no blood sprites, so the gore stays a separate layer that the presenter can hide. Gib quarters and separated parts (Q43) are particles from CF02-D, not sprite frames.

**Goblin B** (`goblin_b_h36_cf02` and the B slammed frames) derives from goblin A: the same body pixels, plus goblin B's `accessory` layer (the red headband, palette `B` rag_red_dark) moved with the head of each frame. Where the headband needs per-frame repainting, the execution repaints it **on the accessory layer only** (no budget cost): on the mirrored panic and hesitate heads the accessory cel is mirrored together with the head cel (only the knot side needs a hand touch); on the folded stomped head, the face-plant flung head and the lolled `wall_rest` dead head it is hand-repainted there. The headband's `B` rag_red_dark has the same RGBA as the gore layer's `B` blood_dark (`#6e1a22`), so on every B frame with gore the headband stays **visually separate from the blood**: at least a 1 px outline or `u` rag_light gap between the accessory and gore pixels, never touching. If a frame needs body edits beyond the accessory, it counts again against B's own 11 (at most 22 in total, Q73).

## 6. Budget against Q73

| Character | Q73 cap | New frames | Keys in this PR | Derived by execution | Notes |
| --- | --- | --- | --- | --- | --- |
| Carl | 13 | **13** (stomp 3, air kick 2, dive 3, bail-out 2, flinch 1, frustration 2) | 9 | 4 | follow-through, bounce and landing recoveries reuse approved frames |
| Donut | 4 | **3** (startle 2, smug 1) | 2 | 1 | 1 spare kept in reserve |
| Goblin A | 11 | **11** (slammed 6, hesitate 1, panic 2, taunt 2) | 9 | 2 | the 6 slammed frames live in the new death sheet but count against this cap |
| Goblin B | 11 | **11** by derivation (accessory layer) | 0 | 11 | counts again only where body pixels change |
| Llama | 0 | 0 | 0 | 0 | |

Total: 27 new frames for A + Carl + Donut, 38 with goblin B, within the Q73 cap for the whole iteration (at most 28 without goblin B, or 39 with it; the unused Donut spare is the difference).

## 7. Timing diagrams (60 Hz ticks)

Legend: each character is one tick; `S` startup, `A` active, `R` recovery, `|` frame change; the letters inside the brackets name the frame.

```text
CARL
Stomp (Q02, 26)       t 0         9  12    16        25
                        SSSSSSSSS AAA RRRR RRRRRRRRRR
                        [knee_up ][stamp  ][recover  ]      hit t9, hit-stop 6 (+3) holds [stamp]
FollowKick (Q05, 32)  t 0      7  9    13   15   20     27   31
                        SSSSSSS SS AAAA RR RRRRR RRRRRRR RRRRR
                        [k0    ][k2][k3     ][k4 ][k5    ][k6 ]  k1 chamber skipped
PushKick (legacy, 36)   [k0 4][k1 7  ][k2 2][k3 6][k4 5][k5 7][k6 5]   unchanged
Air kick (Q50, 21)    t 0    5         15     20
                        SSSSS AAAAAARRRR RRRRRR
                        [chmb][boot     ][jump_fall]        landing inside: [land] 4 ticks
Dive (Q52, Q53, Q56)  t 0     6
                        TTTTTT AAAAAAAA... (until landing, max 30)
                        [tuck ][plunge ...........]
                        hit  -> bounce: [jump_rise][jump_fall], landing recovery 0
                        whiff-> [heavy_landing] 10 ticks, then idle
Bail-out (Q54, Q56)   t 0  3              ~33
                        [po][scramble .........][land 4]   out of a 30 px swing at about t5
Flinch (Q70)            [flinch 8]                          idle only
Frustration (Q70)       [shake 6][flap 6][shake 6][flap 6]   idle only, cooldown 600

DONUT
Startle (Q71)           [puff 6][settle 6]                  idle only, cooldown 180
Smug (Q71)              [smug 20]                           idle only, cooldown 300

GOBLIN
Hesitate (Q61)          [hesitate 20]
Panic (Q61, Q58)        [flee 6][flee_b 6][flee 6]...       60-90 ticks, faces the flee direction
Taunt (Q65)             [thump 9][beckon 9][thump 9][beckon 9]   36 ticks; hit mid-taunt: +6 hold of the shown frame
Slammed (Q21, Q72)      ([taunt hold 6]) [impact >= 8] [rest ...]
                        stomped: [stomped_impact][stomped_rest]
                        wall:    [knockback ...][wall_impact][wall_rest]
                        flung:   [knockback ...][flung_impact][flung_rest]
```

Selector priority (Q69) is unchanged by the art: death > hurt > move > morale reaction > acting overlay > locomotion. Every frame above is chosen from the deterministic move or reaction clock, never from wall-clock time; the `.grid.json` `duration_ms` values are preview-only (`round(ticks x 1000 / 60)`).

## 8. Antihero tone check

| Gate question ([antihero tone](../01-design/antihero-tone.md)) | Answer |
| --- | --- |
| Normal person under extraordinary pressure? | Yes. Carl's new poses are built from his approved body, face and costume; nothing is broader, leaner or more heroic. The stomp hunches, the dive is a straight-legged drop with arms flung out, the landing braces on a knee, the bail-out backpedals yelling with the arms flailing |
| Earned, improvised, a little messy? | Yes. Every move costs something visible: lurch (stomp), lean-away (air kick), tuck and drop (dive), knees (heavy landing), scramble (bail-out) |
| Danger, dark humour, stakes? | The comedy is situational: a goblin punished mid-chest-thump, a dead bug with its legs in the air after a stamp, a face-plant heap after a kick, a bully that runs with its arms up. Nobody mugs at the camera |
| Pixel readability and materials? | Approved palettes only, 0 isolated and 0 orphan pixels, stable pivots; checked at 1x in two lit 640 x 360 Floor 1 compositions (evidence) |
| Lore and provenance? | No new lore; Croc soles, boxers and jacket are the approved costume; goblins keep the pineapple club |
| Original expression? | Yes; no copied poses, film stills or third-party sprites |

Explicitly avoided: three-point or superhero landings, flips, poised kicks, fists raised above the head (they read as cheering), victory poses, obscene goblin gestures, any voice or text.

## 9. Acceptance targets for the execution PRs

**Common (all three PRs).** Keys copied pixel for pixel from `cf02_keys` (a test compares them; Carl's airborne keys may only be lifted 2 px); derived frames drawn from the named sources; approved palette only; 0 isolated and 0 orphan pixels in every frame; first and last canvas columns empty; grounded frames' lowest row on the pivot row, airborne frames per section 2; a manifest `moves` / `reactions` block with the tick ranges of section 7; contact sheet 1x and 3x, timing diagram, and 1x GIF at tick timing; the new sheets are separate files and the approved DEMO-01 sheets stay byte-identical; the art validators and `export_assets.sh` pass; frame count within section 6.

**ART-CARL-CF02** (`carl_h52_cf02`, 48 x 56, pivot (25, 55), 13 frames). Tags `stomp` (3), `air_kick` (2), `dive_tuck` (1), `dive_plunge` (1), `dive_land` (1), `bailout` (2), `flinch` (1), `frustration` (2). The `stomp_knee_up` Croc stays flat and 7 px off the floor until the stamp; the kicking Croc of `air_kick_boot` is the foremost-lowest point of the frame (the lowest pixel forward of the pivot); the far Croc may hang up to 3 px lower; `dive_tuck` and `dive_plunge` differ in silhouette height by at least 6 px at 1x; no new Carl head (only v0 to v3 and the pain head). In-scene 1x GIF: stomp on a lying goblin, jump-in air kick, dive whiff landing, bail-out from a missed kick. `air_kick_chamber` is checked at 1x against `dive_tuck` (it must not read as the tuck), and the `frustration_shake` / `frustration_flap` pair is checked in the 1x GIF (it must not read as a guard; if it does, it goes back to design). Antihero review gate passed by the reviewer.

**ART-DONUT-CF02** (`donut_h40_cf02`, 72 x 44, pivot (46, 43), at most 4 frames). Tags `startle` (2), `smug` (1). Paws on row 43 in every frame; the tiara never touches row 0; the face stays cute (no snarl) in all three; tail, tiara and coat patch map unchanged except the startle fur rim.

**ART-GOB-CF02** (`goblin_a_h36_cf02` and `goblin_b_h36_cf02` with 5 acting frames each, plus the slammed sheets under `goblin/death/` with 6 frames each, 64 x 56, pivot (28, 55)). Tags `hesitate` (1), `panic` (2), `taunt` (2), `slammed_stomped` (2), `slammed_wall` (2), `slammed_flung` (2). The taunt, hesitate and panic clubs stay below the shoulder line (pineapple top on row 28 or lower; never confusable with `attack_tell`, GOB-01 R1); the panic frames face the flee direction with the head looking back and the empty arm up; the hesitate passes the unlabelled idle-strip check of section 5; `taunt_beckon` shows an open palm of at least 3 px with no single-pixel column above the knuckles at 1x; goblin B's headband never touches the gore layer; the wall frames keep the contact column of section 5 and the presenter contract is written in the manifest; the gore layer is separate. **Blind check:** a reviewer who has not read this spec names the three slammed finishes correctly from the 1x rest frames shown next to `crushed_remains` and the routine corpse.

## 10. Open art questions (for the per-sheet reviews and the owner)

1. **Hesitate subtlety (resolved in review).** The first key (a 2 px head pull and a sagging club) read as idle at 1x. The redesigned key mirrors the head, jerks the upper block back, pulls the club in and raises the free hand; `hesitate_check_1x.png` in the evidence is the unlabelled idle-strip check.
2. **Wall impact needs its wall (resolved in review).** The frame only plays on a `WallSlam`, so a wall is always present; the presenter places contact column x16 on the wall face. The key itself now carries the squash, squeezed eyes and splayed arms, and the evidence shows it against a wall stub.
3. **Overlays in `run` (resolved in review).** Planted-feet overlays are idle-only; see the rule in section 3.
4. **Donut spare.** Kept unspent; the owner may prefer a second smug frame (a tail flick) after playing.
5. **Goblin B cost (resolved in review).** Headband repaints happen on the accessory layer only and cost no budget; the headband stays visibly apart from the same-coloured blood (section 5).
6. **Comedy line.** The dead bug, the face-plant heap and the chest thump are pantomime; whether the fleeing goblin stays funny after the third attempt is the owner's call (design open question 6).
