# ENEMY-GAMEPLAY-ENGINEER — the Goblin Engineer as a real enemy (LAB-04 design sheet)

> **Status: design adopted under the owner's design-defaults directive (AGENTS.md, 2026-11-19); not implemented, NOT game-feel accepted.**
> Every choice below has one adopted value with an `ENG-G-Q##` ID ([section 12](#12-adopted-defaults-eng-g-q01eng-g-q26)). Each value is a tuning hypothesis stored as data. The owner reviews game feel after the implementation merges and may change any value in a follow-up PR. Only [section 13](#13-genuinely-open-questions) waits for the owner.
> Design: Claude Opus (top tier). Execution: medium tier, see the [implementation brief](#14-implementation-brief-medium-tier-executor). Review: a separate top-tier agent.

**Why:** GitHub issue #41 (goblin trio and Murder Dozer) is done as art: GOB-01 goblins A/B (#44, in the Bevy scene), ENG-01 Engineer readability set (#86, loaded only by the camera lab, no gameplay; the point tell and pot ram are placeholders) and DOZ-01 (#47). The owner wants more enemy variety in the next playtest. This sheet turns the Engineer into an enemy under the [LAB-04 new-enemy rule](../07-agents/combat-lab-01-workorder.md#lab-04-new-enemy-rule) and LAB-Q14 (a separate `ENEMY-GAMEPLAY-<name>` item that reuses a merged readability set, not new art).

## 1. Authority and locks

- [ENG-01 spec](../02-art/eng-01-spec.md): 40 px, frame 64 x 56, pivot (28, 55), 18 frames; tell = the DOZ-01 rider's **point**, attack = a head-down **pot ram**, recover = the pot slid over the eyes. Section 2 lists what is not approved.
- [Floor 01 opening direction](../02-art/floor-01-opening-catacomb-goblins.md#first-goblins--selected-concept-direction): the wrench is non-canonical shorthand; **ranged bombs, buffs, in-combat repairs and class perks are not invented here**. This design uses only the ram the readability set already shows. No wrench use, no projectile, no mine or turret, no buff to other goblins, no repair, no drop.
- [COMBAT-FUN-02](combat-fun-02-post-demo01-proposals.md): the owner found fights repetitive, impacts inconsequential and goblins identical. Its boundaries apply: no new button, deterministic 60 Hz, seeded variety only, everything off outside `--cf02`, no health sponges.
- [COMBAT-LAB-01](combat-lab-01-advanced-playtest.md#enemy-roster-and-gating): a new archetype stays only if it **visibly changes a tactical choice**; it is added one at a time with an A/B comparison; at most 4 enemies active at once (LAB-Q04).
- [Antihero tone](antihero-tone.md): enemies are threatening yet eccentric, with motivated behaviour and readable tells.

## 2. Tactical role

**The Engineer is the goblins' mid-range enforcer: from up to 96 px away he points at a grounded hero, then charges 64 px along the floor head-first behind his saucepan. The ram punishes a hero who is busy at middle distance, is beaten by a real jump, by distance or by a hit during the point, and can be baited into a wall, the parapet or his own crew, which leaves him dazed with the pot over his eyes.**

| | Goblin A/B | Bad Llama | Engineer |
| --- | --- | --- | --- |
| Range | Close: tell at 30 px, swing reach 8..30 px, in place | Long: spit at 42..48 px height over the yard | **Middle: tell at 96 px or less, ram travels 64 px** |
| What it punishes | Standing next to it; a whiffed move in front of it | Standing in the open at height 42..48 | **Being committed (in a move, in hurt, busy with a goblin) at middle distance on the floor** |
| Main answers | Jab and kick first, bail-out hop, swat the tell | Jump, duck behind the parapet, a perch, Donut stays low | **Jump over it (the bail-out hop is too low), get out of range, hit him during the point, or stand in front of an obstacle and dodge** |
| What it adds to the room | Pressure, morale, pincer | Area denial, cover choice | **A lane: where you stand relative to walls and goblins matters; a punish window you earn by dodging** |
| Body | 24 x 38, HP 2 | 46 x 39, HP 3, leashed to its yard | 24 x 38 (40 px art), **HP 3** |

Why it is not a duplicate: goblins threaten the space **next to** the hero and the llama the space **above the floor**; the Engineer threatens a **horizontal lane on the floor at middle distance**. The bail-out hop (apex about 32 px, CF02-Q54) clears a goblin's reach but **not** the ram (box top 33 px), so the player must choose the right evasive verb per enemy. That is the one-at-a-time A/B claim the playtest must confirm ([section 11](#11-acceptance-criteria)).

## 3. Player-readable tell

1. **Approach:** he walks at 40 px/s, slower than goblins (62), pot rattling in idle (ENG-01 frames 0 to 2). Slow and stubborn: a foreman, not a runner.
2. **The point (32 ticks):** rear (frame 9, 8 ticks), then a level arm pointing straight at the hero with the head and pot pulled back (frame 10, held 24 ticks). This silhouette is unique in the roster: no goblin raises a straight arm at shoulder height, and ENG-01's silhouette gate requires frame 10 to differ from every other frame. **He points at the hero he will ram**; in 2P that tells each player who is targeted.
3. **Launch (3 ticks, frame 11):** arm swings back, head drops. The last instant to start a jump that clears.
4. **Ram (frame 12):** the lowest standing frame and the only one with the pot ahead of the snout.

Audio: the existing goblin tell cue at the start of the point (no new sound asset in v1, ENG-G-Q23). No ground decal or other new VFX: the pose is the tell.

## 4. Moves and frame data (60 Hz, all data knobs)

Attack tick 0 = the tell starts. Pixels from the pivot (x forward, y up from the feet).

| Phase | Ticks | Frames | Motion | Boxes | Interrupt |
| --- | --- | --- | --- | --- | --- |
| Tell: rear | 0..8 | 9 | none | hurtbox = body | Any hero hit staggers him and cancels the attack (token returned), like a goblin tell; Donut's swat too (Q10 light guard rules) |
| Tell: point | 8..32 | 10 | none, facing locked at tick 0 | body | as above |
| Launch | 32..35 | 11 | none | body | as above |
| Ram (active) | 35..51 at most | 12 held | **240 px/s** forward (4 px per tick, 64 px at most) | Ram box `[14, 24) x [14, 33)` every active tick; hurtbox = body | **Not interruptible** by ordinary hits (damage and hit-stop only, as a goblin swing); a lethal hit or a CF02-B contact knockdown (Q35) ends it |
| Recover (after a whiff or a hero hit) | 30 | 13 held | stops dead | body | Any hit staggers him (as a goblin recovery) |
| Bonk (the ram met a solid) | 12 + 48 = 60 | 14, then 13 held | stops at the solid | body | Hits land normally; a launch or knockdown overrides |
| Cooldown before the next tell | 60 | idle / walk | approach or hold | | |

How a ram ends (checked every active tick, in this order):

1. **Hero:** the ram box overlaps a hero hurtbox → 1 damage, the hero hurt path (`HERO_HURT_TICKS` 18, hit-stop 4) with the Q42 knock **120 / 160 px/s** in the ram direction, invulnerability respected. One hero per ram; the ram stops and enters Recover.
2. **Solid:** his body would enter a solid (end wall, parapet face) → **bonk**: he takes **1** self-damage (the Q11 wall-slam damage value), the hero he pointed at is credited, and he enters Bonk. At 0 health he is defeated in place by the existing wall-slam defeat path.
3. **Crew:** the ram box overlaps a **living goblin on its feet** (Neutral, Attack or Hurt; never a lying, launched, defeated or Engineer body) → that goblin gets the existing CF02-B contact knockdown (`contact_knockdown`, Q09 mode `knockdown`, Q12 damage 0) pushed in the ram direction; a goblin mid-swing loses the swing (Q35); a `BodyContact` event credits the pointed-at hero. The Engineer stops and enters Recover.
4. **Distance:** 16 active ticks elapsed → Recover (a whiff).

The ram passes through the llama and under the perches (his 40 px fits under the 68 px slab underside), and over lying bodies.

## 5. Brain: targeting, lane check and tokens

- **Sight:** 220 px, the goblin sight, so he wakes by position like the CF02-F waves.
- **Eligible target:** the nearest living hero whose feet are within **8 px** of his floor line. A hero on a perch (80 px), on the parapet top (48 px) or in a jump is not a ram target.
- **Lane check (at tell start only):** no solid and no living goblin on its feet lies between his front edge and the target's column. If the lane is blocked he does not tell; he holds at the standoff. A goblin or hero that **moves into the lane during the point** is the player's doing: that is the bait.
- **Approach:** walk toward the target until it is within **96 px** (pivot to pivot) and stop there (the standoff). With no token he holds at 96 px facing the target. There is no minimum range: point-blank he still points, which makes him easy prey for a jab during the 32-tick tell.
- **Perch and parapet:** while no hero is eligible he walks to 96 px from the nearest hero's column and waits there, facing it. A hero who walks off the perch lands in his range; a hero who dives on him from the perch (Q52) knocks him down. He never climbs.
- **Tokens:** he shares the goblins' two attack tokens (`MAX_ATTACKERS` 2) and the 12-tick tell spacing, holding a token from the tell start until the ram ends. So a fight never has more than two attacks in flight, Engineer included. The CF02-Q66 pincer preference applies only to goblins, but counts his side as a holder side.

## 6. Size, collider and hit reactions

- **Body and hurtbox:** the goblin's 24 x 38 body (`body_rect`). ENG-01's measured idle hurtbox (-9..14 x 0..37) is within 1 px of it, so v1 keeps one rect for physics and hurt, which keeps every existing contact, wall, shore and Donut rule valid. Per-frame hurtboxes from ENG-01 section 3 are deferred.
- **Ram box:** `[14, 24) x [14, 33)` from ENG-01 frame 12 (pot front and snout).
- **Health 3**, one more than a goblin (level 3), never a sponge: a jab-haymaker string plus one hit, one kick-wall-slam plus one hit, or two baited bonks plus one hit kills him.
- **Hit reactions:** exactly a goblin's under `cf02_rules`: jab and haymaker stagger, the push kick launches him (contacts and wall slams apply), CF02 lying, the Q34 keep-down rule, stomp, dive, air kick, the Q38 pounce hold, Donut's swat light guard. Only his attack differs.

## 7. Counterplay

| Who | Answer | Cost or risk |
| --- | --- | --- |
| Carl | **Jump** (apex 92 px; feet above 33 px from about tick 5 to tick 45 of the jump), started anywhere from the middle of the point to the launch; land and punish the 30-tick recover from behind | Jumping too early lands in the ram; the llama's spit line is 42..48 px |
| Carl | **Get out of range**: running away at 185 px/s during the point always escapes the 88 px reach | Gives up ground under llama fire; no punish |
| Carl | **Hit him during the point** (jab, startup 3): stagger, token back | Must close in; a goblin may be swinging |
| Carl | **Matador:** stand with a wall or the parapet behind you, jump on the point → he bonks (1 self-damage, 60-tick daze) | Mistimed, you are pinned against the wall with 1 damage |
| Carl | **Bowl the crew:** stand so a goblin is in the lane, or kick one into it during the point → the ram knocks his own goblin down (and a lying goblin is a stomp target) | Needs reading two enemies at once |
| Carl | From a perch: **dive** onto him while he waits below | Leaving the perch exposes you to spit |
| Donut | **Swat the point** (startup 4) to cancel it; her cat hop (apex about 43 px) clears the ram; pounce or Magic Missile (post-biscuit only) in the recover | Her 24 px hurtbox is fully inside the ram box: she cannot duck under it |
| Pair | One hero is pointed at and baits; the other punishes the recover or bonk from the side | |

The bail-out hop (apex about 32 px) does **not** clear the ram: an intentional difference from the goblins.

## 8. 1P, 2P and the AI companion

- **Solo, Carl controlled, CF02 assistant on:** the Engineer is an ordinary goblin target in the `CompanionView` with the state mapping tell → `Tell`, launch → `Smear`, ram → `Impact`, recover and bonk → `Recovery`. The `cf02_assist` profile is unchanged: it may swat his point within 96 px (Q14) and never kills (`ca19_kill_policy` never). v1 adds **no** ram-lane avoidance to the AI; rams that hit the AI Donut are counted and reported. If the solo arena fixtures show more than one ram hit on the AI Donut per clear, a COMPANION-AI-01 follow-up item is opened (AI behaviour belongs there, LAB-Q12).
- **Solo, Donut controlled:** the same rules; AI Carl (CA-37 limits) is measured, not required to match.
- **2P:** he targets the nearest eligible hero at the tell start and shows it by pointing; one hero per ram; the partner is never hit by the same ram. No friendly fire. Both heroes can use every answer in [section 7](#7-counterplay).
- **Swap:** the facing and target lock at the tell start; swapping heroes during the point does not retarget.

## 9. Death and impact type

- **Authoritative:** unchanged. His defeats use the existing causes and force tiers (melee, wall slam, shore block, blast), with no new death physics.
- **Presentation v1 (no new art):** a new species entry `goblin_engineer` whose recipes all map to `engineer_down` = ENG-01 `down` frames 15 to 17 (the fall beat, the floor impact, the pot landing rim-down over his face). The lethal knock arc is the goblin's. No goblin A finish is borrowed in v1, because GOB-02 and the slammed cels have no pot and the pot would vanish.
- **Presentation after ENG-02 (art order, [section 10](#10-frames-per-state-and-the-missing-art-order-eng-02)):** the same recipes as the goblin plus the CF02 `slammed` overlay (stomped, wall crumple, flung), crushed and decapitated, each a goblin A cel plus a `pot` accessory layer; the pot becomes a tumbling death part in the wall, flung and exploded finishes.
- **Morale and CINE:** his defeat counts as **spectacular** (-35) for every goblin witness, whatever the cause: the crew sees the foreman fall (the target-priority decision). A bonk emits the appended `RamBonk` event; morale treats it as a `WallSlam` (-15); it never makes a defeat CINE-01 eligible (CF02-Q25 unchanged).

## 10. Frames per state and the missing art order ENG-02

| Gameplay state | ENG-01 frames (tag) | Playback |
| --- | --- | --- |
| Idle, holding at the standoff | 0 to 2 (`idle`) | loop 38 |
| Walk | 3 to 8 (`walk`) | loop 36 (its contact plan is exactly 40 px/s) |
| Tell | 9 (8), 10 (24) (`attack_tell`) | as authored |
| Launch | 11 (`attack_ram`) | 3, as authored |
| Ram active | 12 (`attack_ram`) | **held** for the active ticks (authored 6) |
| Recover | 13 (`attack_recover`) | **held** 30 (authored 18) |
| Bonk | 14 (12 ticks), then 13 (48) | state driven |
| Hurt, stagger, contact pose | 14 (`hurt`) | state driven |
| Launched or airborne | 14 | state driven |
| Landing, lying (`Downed`), Q03 | 15, 16, then 17 (`down`) | state driven |
| Get-up | **13** (he stands up with the pot over his eyes) | for `q04_getup_ticks` |
| Defeat (all finishes, v1) | 15 to 17 (`down`) | 5 + 10, then hold |

The held frames are playback only; the ENG-01 sources, manifest and validator do not change.

**ENG-02 (separate art order, not blocking this item, recommended after the owner's game-feel review, ENG-G-Q24).** Route: Easy, variant of the GOB-01/GOB-02 cels plus the DOZ-01 pot, the same way GOB-02B derives B from A ([GOB-02B spec](../02-art/gob-02b-death-spec.md)). Top tier writes the spec and the pot placement per frame, medium tier executes, separate top-tier review.

| Set | Frames | Source |
| --- | --- | --- |
| Get-up | 2 (sit up with the pot over the face, shove the pot back up) | GOB-01 get-up cels plus the pot |
| Slammed | 6 (stomped, wall, flung: impact and rest) | [CF02 acting](../02-art/cf02-acting-spec.md) goblin A slammed cels plus a `pot` accessory layer, the pot knocked loose in wall and flung |
| Crush and decap | 4 (`crush_contact`, `crushed_remains`, `decap_contact`, `headless_collapse`) | [GOB-02 A](../02-art/gob-02-death-spec.md) cels plus the pot (flattened under the block; the head part keeps the pot) |
| Death part | 1 part (3 rotation cels): the saucepan | the DOZ-01 pot pixels |
| Optional | 2: a ram run cycle (legs alternate under frame 12's torso) | only if the held frame 12 reads as a slide at 1x |

No hesitate, panic or taunt frames are needed: he has no morale reactions (ENG-G-Q18).

## 11. Acceptance criteria

Machine (CI):

1. Every fixture in [section 14](#fixtures-to-add) passes and replays identically twice; its golden is pinned.
2. Every golden listed under [goldens that must stay unchanged](#goldens-that-must-stay-unchanged) is byte-identical.
3. `enemy_engineer_v0.json` loads, validates (a load error names the field) and matches its pinned FNV-1a hash; a completeness test proves every ENG-G-Q has a knob or a recorded no-knob reason.
4. The gate tests in [section 14](#flag-gating) pass: the Engineer can only appear under `cf02_rules` with the engineer switch.
5. Performance: frame time of `cf02_arena_eng` versus `cf02_arena` reported (before and after, same machine); no regression claimed without numbers.

Evidence (implementation PR):

6. Clips at 1x speed, 640 x 360: point → ram → whiff → punish; jump-over; matador bonk against the parapet; bowling a goblin; a perch dive on a waiting Engineer; a 2P point that shows who is targeted. A contact sheet of the tell next to goblin A's tell at 1x.
7. A/B report (LAB-04 one-at-a-time proof): the same scripted routes and seeds on `cf02_arena` and `cf02_arena_eng`, reporting ticks in the yard, damage taken, jumps, bail-outs, perch time, `RamBonk` and bowl counts, and hits taken by the AI Donut.

Human (owner, game feel; the item stays "merged, game-feel review pending" until then):

8. The owner can name, without being told, who the Engineer is about to ram and when to jump.
9. In at least one of three attempts the owner changes plan because of him (jumped instead of bailing out, baited a bonk or a bowl, dived from the perch, or killed him first to break the crew).
10. He is not annoying: no unreadable hits, no pinning loop, no stalling turret problem at the perch.

## 12. Adopted defaults (ENG-G-Q01..ENG-G-Q26)

Knob names follow the CF02 convention: the knob name starts with the question ID (`engq02_health`). The file is `assets/data/gameplay/enemy_engineer_v0.json`.

| ID | Question | Adopted value | Rationale | Knob |
| --- | --- | --- | --- | --- |
| ENG-G-Q01 | Role | Middle-range floor-lane charger with a punish window, [section 2](#2-tactical-role) | Distinct from goblin (close) and llama (height); uses only the ENG-01 ram | none (design) |
| ENG-G-Q02 | Health | **3** | Level 3, one more than a goblin, not a sponge | `engq02_health` |
| ENG-G-Q03 | Body and boxes | Body 24 x 38 (the goblin's); ram box `[14, 24) x [14, 33)` | Within 1 px of ENG-01; reuses every goblin contact rule | `engq03_ram_box` |
| ENG-G-Q04 | Walk speed and sight | **40** px/s; sight **220** | ENG-01 contact plan; goblin sight wakes the wave by position | `engq04_walk_speed`, `engq04_sight` |
| ENG-G-Q05 | Trigger and standoff | Tell at **96** px or less (pivot to pivot); standoff **96**; no minimum | Ram reach is 64 + 24 = 88 px, so 96 starts just outside his hit | `engq05_trigger_px`, `engq05_standoff_px` |
| ENG-G-Q06 | Target eligibility | Nearest living hero with feet within **8** px of his floor line; facing and target lock at tell start | No ram on perched or jumping heroes; a readable, unchanging point | `engq06_floor_tolerance_px` |
| ENG-G-Q07 | Lane check | At tell start: no solid and no living goblin on its feet between his front and the target's column, else no tell | Stops trivial wall-cheese and accidental crew bowling; bait needs player timing | `engq07_lane_check` (switch) |
| ENG-G-Q08 | Tell | **32** ticks: rear 8 + point 24; interruptible by any hero hit, swat included | ENG-01 ticks; the longest enemy tell matches the longest reach | `engq08_rear_ticks`, `engq08_point_ticks` |
| ENG-G-Q09 | Launch | **3** ticks, no motion, no box | ENG-01 frame 11 | `engq09_launch_ticks` |
| ENG-G-Q10 | Ram | Active at most **16** ticks at **240** px/s (64 px); not interruptible except by a lethal hit or a contact knockdown | Faster than Carl's run (185) but short; armour matches the goblin swing rule | `engq10_active_ticks`, `engq10_speed_px_s` |
| ENG-G-Q11 | Ram hit on a hero | **1** damage, 18-tick hurt, hit-stop 4, knock **120 / 160** px/s in the ram direction; one hero per ram; the ram stops | Existing hurt path and Q42 impulse; a heavier hit than a swing, no new frames | `engq11_damage`, `engq11_knock_x`, `engq11_knock_y` |
| ENG-G-Q12 | Recover | **30** ticks (frame 13 held); hits stagger him | A punish window you earn by dodging | `engq12_recover_ticks` |
| ENG-G-Q13 | Bonk | Hurt **12** + daze **48**; self-damage **1** (Q11 value); credit to the pointed-at hero; appended `RamBonk` event; existing wall-slam defeat path; not CINE eligible | The matador answer is a real kill path but needs two well-timed dodges plus a hit | `engq13_hurt_ticks`, `engq13_daze_ticks`, `engq13_self_damage` (0 = off) |
| ENG-G-Q14 | Crew bowl | A living goblin on its feet in the ram box gets the CF02-B contact knockdown in the ram direction (Q12 damage 0); `BodyContact` credits the pointed-at hero; the Engineer recovers 30 | Player-authored causality with the CF02-B system, no new physics | `engq14_bowl` (switch) |
| ENG-G-Q15 | Cooldown | **60** ticks after each attack (goblin 40) | One ram about every 2.3 s at most; he is a punisher, not a spammer | `engq15_cooldown_ticks` |
| ENG-G-Q16 | Tokens | Shares `MAX_ATTACKERS` 2 and the 12-tick tell spacing; holds a token from tell start to ram end | At most two attacks in flight, Engineer included | none (code) |
| ENG-G-Q17 | Shore | CF02-Q31 applies to his ram box: one shore hit per ram on a shore at `damaged_1` or later (`ShoreTripped`); an intact shore ignores him; the shore column is not a solid, so the ram passes | One consistent self-own rule for the goblin family | none (reuses `q31_*`) |
| ENG-G-Q18 | Morale | No nerve, no reactions (never hesitates, panics, taunts or enrages) and never witnesses; he counts as a living goblin for the CF02-Q62 "last goblin" rule | The stubborn foreman contrasts with the cowardly crew; no acting frames needed | `engq18_morale` (switch, `false`) |
| ENG-G-Q19 | Crew reaction to him | His defeat is a spectacular witness event (**-35**) for goblins in range; `RamBonk` counts as `WallSlam` (**-15**); his hit on a hero is a hero-hurt event (+15) | Killing him first breaks the crew's nerve: a target-priority choice using only existing deltas | `engq19_defeat_spectacular` (switch) |
| ENG-G-Q20 | Hit reactions | The goblin's under `cf02_rules`, unchanged | One rig, one reaction language | none |
| ENG-G-Q21 | Death presentation | Species `goblin_engineer`; v1 every recipe → `engineer_down` (ENG-01 15 to 17); after ENG-02, goblin recipes plus pot | No pot vanishing; authoritative death unchanged | data (death profile) |
| ENG-G-Q22 | Companion | Seen as a goblin target with the state mapping of [section 8](#8-1p-2p-and-the-ai-companion); no new AI behaviour; AI Donut ram hits reported, follow-up if more than 1 per clear | AI changes belong to COMPANION-AI-01 | none |
| ENG-G-Q23 | Cue | The existing goblin tell cue at the point start; no new audio asset | No invented sound; the owner judges audio later | none |
| ENG-G-Q24 | Art | v1 uses ENG-01 only ([section 10](#10-frames-per-state-and-the-missing-art-order-eng-02)); ENG-02 is ordered after the owner's game-feel review | Do not spend art on a role the owner may reject | none (process) |
| ENG-G-Q25 | Gate and placement | `cf02_rules` plus the engineer switch, reached only by `floor01_scene --cf02 --cf02-engineer`; fixture `cf02_arena_eng`: the CF02-F room with the yard goblin at x 940 (index 4) replaced by the Engineer at x 940 | Plain `--cf02` stays the regression room; one-for-one swap keeps six enemies and the LAB-Q04 cap | `engq25_enabled` (master switch) |
| ENG-G-Q26 | Temperaments in the variant | The four remaining goblins take slots 0 to 3 of the same CF02-Q58 shuffled bag; the fifth slot is unused, so the missing temperament varies per attempt | No change to Q58; still different on every restart | none |

## 13. Genuinely open questions

1. **Feel of the ram on a real pad:** 240 px/s over 64 px, the 30-tick recover, and whether the held frame 12 reads as a charge or a slide at 1x (the trigger for the optional ENG-02 run cycle).
2. **Is the matador bonk fun or a cheese?** `engq13_self_damage` 0 turns the bonk into a daze-only punish.
3. **Story placement:** whether the Engineer fights on foot in the Floor 01 opening after the Dozer is wrecked (ENG-01 section 13), and whether he joins plain `--cf02` or a LAB room (R04 Pressure Chamber is the natural candidate after LAB-03). Both wait for the owner's feel review.
4. **ENG-02 timing:** commission the death and get-up frames after the feel review (adopted) or before.
5. **The wrench** stays a stowed prop; any tool use needs a new design decision.

## 14. Implementation brief (medium-tier executor)

One PR (speed mode allows grouping), or ENG-I1 (Bevy-free domain, data, fixtures) then ENG-I2 (Bevy wiring, flag, evidence). Do not edit `docs/04-production/backlog.md`; put the status in the PR body.

### Files to touch

| File | Change |
| --- | --- |
| `assets/data/gameplay/enemy_engineer_v0.json` (new) | The `engq*` knobs of section 12, embedded with `include_str!`, FNV-1a pinned like `cf02_combat_v0.json` |
| `src/engineer.rs` (new, Bevy-free) | `EngineerTuning` (load, validate, pin, completeness list), the attack timeline (`EngineerPhase`: rear, point, launch, ram, recover, bonk), the lane check, ram stepping; unit tests |
| `src/goblin.rs` | `kind: GoblinKind { Common, Engineer }` (default `Common`, mixed into the state hash **only** when `Engineer`); `step_with` dispatches the attack and brain to `engineer.rs` for the Engineer; the legacy branch is untouched |
| `src/combat_scene.rs` | `Fixture::engineers: &'static [EnemySpawn]` (empty everywhere existing; the `llamas` field is the precedent), built as Engineer goblins after the fixture's goblins; ram-on-hero hit through the existing hero hurt path; the bonk and appended `CombatEvent::RamBonk`; tokens and tell spacing include him |
| `src/combat_scene/contacts_cf02.rs` | The crew bowl (`contact_knockdown` and `BodyContact` with the pointed-at hero as instigator) |
| `src/morale.rs`, `src/combat_scene/morale_cf02.rs` | Skip Engineer nerve; his defeat → spectacular delta; `RamBonk` → the `WallSlam` delta; the Q58 draw index counts common goblins only; Q62 counts him as living |
| `src/cf02.rs` (`arena` module) | `GOBLINS_ENG` (520, 600, 680, 880), `ENGINEERS` (940), fixtures `cf02_arena_eng` and `cf02_arena_eng_cp2`, `fixture_uses_arena` entries |
| `src/cf02_fixtures.rs` | The new fixtures and their goldens |
| `src/bin/floor01_scene.rs` | The `--cf02-engineer` token, rejected without `--cf02` and by `--demo`, the story route and `--scene combat_lab` |
| `assets/data/death/species_death_profiles_cf02.json` and the death presenter | Species `goblin_engineer` → `engineer_down`; if the overlay schema cannot add a species, extend it explicitly and record the migration. **`species_death_profiles.json` stays byte-identical** |
| Scene presentation (the goblin sprite adapter) | Engineer kind → the `eng_01` atlas (`assets/manifests/eng_01.json`) with the frame mapping of section 10 |
| `src/companion/` | The Engineer in `CompanionView` with the section 8 state mapping; no behaviour change |
| PLAYTRACE observation | If it enumerates goblins, add the kind only for non-common goblins, so existing traces stay byte-identical; otherwise record a follow-up |

Do **not** touch: `src/camera_lab/` (its `ENGINEER` FoeSpec is a lab hypothesis, not this design), `src/combat_lab/descriptor.rs` (`engineer` is not a lab roster entry yet), the ENG-01 art sources and manifest, `cf02_combat_v0.json`.

### Fixtures to add

All headless on the arena kit, under `cf02_rules` with the engineer switch, golden-pinned:

| Fixture | Proves |
| --- | --- |
| `eng_ram_hits_idle_carl` | Carl idle at 80 px: tell 32, launch 3, hit on the computed active tick, 1 damage, knock 120 / 160, Recover 30 |
| `eng_jump_over_ram` | Jump at point tick 20: no damage, whiff, a jab lands during the recover |
| `eng_bailout_does_not_clear` | Bail-out on the point: Carl is still hit (the distinct-verb property) |
| `eng_run_out_of_range` | Running away at the tell start: whiff |
| `eng_point_interrupt_jab`, `eng_point_interrupt_swat` | Point cancelled, token returned (the swat with controlled Donut) |
| `eng_ram_armor` | A jab in the ram: damage and hit-stop, no stagger, the ram continues |
| `eng_parapet_bonk` | Matador at the parapet's right face: `RamBonk`, health 3 → 2, 60-tick bonk |
| `eng_end_wall_bonk_kill` | Third bonk defeats him; credit to Carl; no CINE trigger |
| `eng_lane_blocked_no_tell`, `eng_perch_not_targeted` | No tell through a goblin, a solid or onto a perch; he waits at the standoff |
| `eng_bowls_goblin` | A goblin kicked into the lane during the point is knocked down; `BodyContact` credits Carl |
| `eng_shore_trip` | A ram over a damaged shore's strike zone counts once (`ShoreTripped`); an intact shore ignores it |
| `eng_defeat_scares_crew` | A goblin within 160 px loses 35 nerve on his defeat |
| `eng_perch_dive` | Dive stomp from the right perch knocks the waiting Engineer down |
| `eng_kick_launch_contact` | A push kick launches him into a goblin (CF02-B contact) |
| `eng_2p_targets_nearest` | 2P: the nearest grounded hero is pointed at and only that hero is hit |
| `eng_solo_assist` | Solo with the CF02 assistant: no softlock, ram hits on the AI Donut counted |
| `cf02_arena_eng`, `cf02_arena_eng_cp2` | The live variants: replay identical; a restart reshuffles (Q58) |

Gate tests: `--cf02-engineer` without `--cf02` errors; `--demo`, the story route and the lab entry reject it; with the engineer switch off a run hashes exactly like `cf02_arena`; the data-file pin and completeness tests.

### Goldens that must stay unchanged

Legacy DEMO-01 and cf02-off scene hashes; every entry of `GOLDENS` in `src/cf02_fixtures.rs` (including `cf02_arena` and `cf02_arena_cp2`); the companion, Donut, party, boss, CINE, death presenter, PLAYTRACE and combat-lab fixture goldens; the pinned AUTOPLAY `autoplay-episode/1` goldens; `cf02_combat_v0.json` and its `TUNING_HASH`; `species_death_profiles.json`; the ENG-01 tests (`tests/test_art_eng01.py`).

### Flag gating

The Engineer exists only when `cf02_rules` is on **and** `engq25_enabled` is set by `--cf02-engineer`; switches are read through one accessor (as `Cf02Tuning::on`), never raw. `--demo`, the story route, plain `--cf02` and the lab profile never load him.

## 15. Placement plan

**`floor01_scene --cf02 --cf02-engineer` runs `cf02_arena_eng`: the CF02-F microarena with the second wave's goblin at x 940 replaced by the Engineer, so he guards the llama yard between the parapet (a matador wall), the right perch (a dive spot) and goblin 880 (a bowling pin), under llama fire.**

## 16. Horizon fit

Consulted the [character progression horizon](character-progression-horizon.md). Still consistent, no guide change: the Engineer adds no player verb, unlock or button; he makes existing Floor 01 verbs (the jump, the dive, the push kick, wall and body impacts, Donut's swat) the right answer at the right time, and rewards positioning against terrain and enemies. Solo and 2P get the same answers. The engine contract stays IP-neutral (`GoblinKind::Engineer` is a generic data kind; the fan-concept layer is only the art).

## 17. Rights and provenance

No new art, audio or text from third parties. The ENG-01 sources and provenance are unchanged ([ENG-01 section 15](../02-art/eng-01-spec.md#15-rights-and-provenance)). Gameplay values are original design hypotheses. Design by Claude Opus (top tier); execution and review are not done yet.
